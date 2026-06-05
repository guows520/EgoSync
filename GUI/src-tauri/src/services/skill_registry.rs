use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::db::skills;
use crate::error::AppError;
use crate::models::skill::{
    ImportCustomSkillInput, ImportCustomSkillResult, PreviewCustomSkillInput, SkillDuplicateInfo,
    SkillImportPreview, SkillRegistryEntry, SkillRoleScope, BUTLER_SCOPE_ID,
};

const SKILL_FILE_NAME: &str = "SKILL.md";

pub async fn list_registry(pool: &SqlitePool) -> Result<Vec<SkillRegistryEntry>, AppError> {
    skills::list_skills(pool).await
}

pub async fn list_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    let mut skill_ids = crate::db::skill_bindings::skill_ids_for_role(pool, role_id).await?;
    if let Ok(role) = crate::db::roles::get_role(pool, role_id).await {
        for skill_id in crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config) {
            if !skill_ids.iter().any(|id| id == &skill_id) {
                skill_ids.push(skill_id);
            }
        }
    }
    filter_registry_by_ids(pool, &skill_ids).await
}

pub async fn list_all_role_skills(pool: &SqlitePool) -> Result<Vec<SkillRegistryEntry>, AppError> {
    let mut skill_ids = crate::db::skill_bindings::all_role_skill_ids(pool).await?;
    if let Ok(butler_skills) = crate::services::butler_config::get_butler_skills(pool).await {
        for skill_id in butler_skills.enabled_skill_ids {
            if !skill_ids.iter().any(|id| id == &skill_id) {
                skill_ids.push(skill_id);
            }
        }
    }
    filter_registry_by_ids(pool, &skill_ids).await
}

async fn filter_registry_by_ids(
    pool: &SqlitePool,
    skill_ids: &[String],
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    let registry = skills::list_skills(pool).await?;
    Ok(registry
        .into_iter()
        .filter(|skill| skill_ids.iter().any(|id| id == &skill.id))
        .collect())
}

#[cfg(test)]
pub async fn scope_for_skill(pool: &SqlitePool, skill_id: &str) -> Result<SkillRoleScope, AppError> {
    let role_ids = crate::db::skill_bindings::role_ids_for_skill(pool, skill_id).await?;
    let all_roles = role_ids
        .iter()
        .any(|id| id == crate::db::skill_bindings::ALL_ROLES_BINDING);
    Ok(SkillRoleScope {
        all_roles,
        role_ids: if all_roles { Vec::new() } else { role_ids },
    })
}

fn normalized_scope(input: Option<&SkillRoleScope>) -> SkillRoleScope {
    match input {
        Some(scope) if scope.all_roles => SkillRoleScope {
            all_roles: true,
            role_ids: Vec::new(),
        },
        Some(scope) => SkillRoleScope {
            all_roles: false,
            role_ids: unique_role_ids(scope.role_ids.iter().cloned()),
        },
        None => SkillRoleScope {
            all_roles: false,
            role_ids: Vec::new(),
        },
    }
}

fn unique_role_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut unique = Vec::new();
    for id in ids {
        let trimmed = id.trim();
        if !trimmed.is_empty() && !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
    }
    unique
}

fn role_binding_ids(scope: &SkillRoleScope) -> Vec<String> {
    scope
        .role_ids
        .iter()
        .filter(|id| id.as_str() != BUTLER_SCOPE_ID)
        .cloned()
        .collect()
}

fn scope_includes_butler(scope: &SkillRoleScope) -> bool {
    scope.all_roles || scope.role_ids.iter().any(|id| id == BUTLER_SCOPE_ID)
}

async fn apply_scope_to_butler(
    pool: &SqlitePool,
    skill_id: &str,
    scope: &SkillRoleScope,
) -> Result<(), AppError> {
    let mut skills = crate::services::butler_config::get_butler_skills(pool).await?;
    let has_skill = skills.enabled_skill_ids.iter().any(|id| id == skill_id);
    if scope_includes_butler(scope) && !has_skill {
        skills.enabled_skill_ids.push(skill_id.to_string());
    }
    if !scope_includes_butler(scope) && has_skill {
        skills.enabled_skill_ids.retain(|id| id != skill_id);
    }
    crate::services::butler_config::set_butler_skills_config(pool, &skills).await?;
    Ok(())
}

async fn apply_scope(
    pool: &SqlitePool,
    skill_id: &str,
    scope: &SkillRoleScope,
) -> Result<(), AppError> {
    apply_scope_to_roles(pool, skill_id, scope).await?;
    apply_scope_to_butler(pool, skill_id, scope).await
}

pub async fn remove_skill_from_role(
    pool: &SqlitePool,
    skill_id: &str,
    role_id: &str,
) -> Result<(), AppError> {
    if let Ok(role) = crate::db::roles::get_role(pool, role_id).await {
        if let Some(next_config) =
            crate::services::role_config::remove_enabled_skill_id(&role.skills_config, skill_id)
        {
            crate::db::roles::set_role_skills_config_raw(pool, &role.id, &next_config).await?;
        }
    }

    let binding_ids = crate::db::skill_bindings::role_ids_for_skill(pool, skill_id).await?;
    if binding_ids.iter().any(|id| id == crate::db::skill_bindings::ALL_ROLES_BINDING) {
        let roles = crate::db::roles::list_all_roles(pool).await?;
        let remaining_role_ids: Vec<String> = roles
            .into_iter()
            .filter(|role| role.id != role_id)
            .map(|role| role.id)
            .collect();
        crate::db::skill_bindings::replace_bindings(pool, skill_id, false, &remaining_role_ids).await?;
        return Ok(());
    }

    crate::db::skill_bindings::remove_binding_for_role(pool, skill_id, role_id).await
}

async fn apply_scope_to_roles(
    pool: &SqlitePool,
    skill_id: &str,
    scope: &SkillRoleScope,
) -> Result<(), AppError> {
    let roles = crate::db::roles::list_all_roles(pool).await?;
    for role in roles {
        let should_enable = scope.all_roles || scope.role_ids.iter().any(|id| id == &role.id);
        let mut skills = crate::services::role_config::skills_from_config(&role.skills_config);
        let mut enabled = skills.enabled_skill_ids.unwrap_or_default();
        let has_skill = enabled.iter().any(|id| id == skill_id);
        if should_enable && !has_skill {
            enabled.push(skill_id.to_string());
        }
        if !should_enable && has_skill {
            enabled.retain(|id| id != skill_id);
        }
        skills.enabled_skill_ids = Some(enabled);
        let next_config = crate::services::role_config::normalize_skills_config_with_existing(
            &role.skills_config,
            &skills,
        )?;
        if next_config != role.skills_config {
            crate::db::roles::set_role_skills_config_raw(pool, &role.id, &next_config).await?;
        }
    }
    Ok(())
}

/// 删除一条自定义 Skill（P3）：移除 registry 记录、删除受控副本文件，并清理所有角色
/// enabledSkillIds 中的死 id，避免「权限开着但 prompt 无 Skill」的漂移与死 id 永久累积。
/// 返回被清理了该 id 的角色 id 列表，供命令层触发 opencode 同步。
pub async fn delete_custom_skill(pool: &SqlitePool, skill_id: &str) -> Result<Vec<String>, AppError> {
    // 先取记录以拿到受控副本路径（用于删除磁盘文件）。
    let entry = skills::get_skill(pool, skill_id).await?;
    skills::delete_skill(pool, skill_id).await?;
    crate::db::skill_bindings::delete_bindings_for_skill(pool, skill_id).await?;

    // best-effort 删除受控副本目录；失败不阻断（registry 已删，文件残留仅占空间）。
    let managed_path = Path::new(&entry.managed_path);
    if let Some(parent) = managed_path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }

    // 清理所有角色（含归档）enabledSkillIds 中的该 id。
    let mut affected = Vec::new();
    let roles = crate::db::roles::list_all_roles(pool).await?;
    for role in roles {
        if let Some(next_config) =
            crate::services::role_config::remove_enabled_skill_id(&role.skills_config, skill_id)
        {
            crate::db::roles::set_role_skills_config_raw(pool, &role.id, &next_config).await?;
            affected.push(role.id);
        }
    }
    Ok(affected)
}

pub async fn preview_custom_skill(
    pool: &SqlitePool,
    input: &PreviewCustomSkillInput,
) -> Result<SkillImportPreview, AppError> {
    let content = load_skill_content(input.content.as_deref())?;
    let parsed = parse_skill_content(&content)?;
    preview_from_parsed(pool, parsed).await
}

pub async fn import_custom_skill(
    pool: &SqlitePool,
    skills_root: &Path,
    input: &ImportCustomSkillInput,
) -> Result<ImportCustomSkillResult, AppError> {
    let content = load_skill_content(input.content.as_deref())?;
    let parsed = parse_skill_content(&content)?;
    let preview = preview_from_parsed(pool, parsed.clone()).await?;
    let scope = normalized_scope(input.role_scope.as_ref());
    if let Some(duplicate) = preview.duplicate.as_ref() {
        if !input.overwrite_existing {
            let role_ids = role_binding_ids(&scope);
            crate::db::skill_bindings::replace_bindings(
                pool,
                &duplicate.existing.id,
                scope.all_roles,
                &role_ids,
            )
            .await?;
            apply_scope(pool, &duplicate.existing.id, &scope).await?;
            return Ok(ImportCustomSkillResult {
                status: "duplicate".to_string(),
                entry: Some(duplicate.existing.clone()),
                preview,
            });
        }
    }

    let managed_path = managed_skill_path(skills_root, &parsed.name, &parsed.content_hash);
    if let Some(parent) = managed_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::ValidationError(format!("创建受控 Skill 目录失败: {}", e)))?;
    }
    std::fs::write(&managed_path, &content)
        .map_err(|e| AppError::ValidationError(format!("保存 Skill 文件失败: {}", e)))?;
    let managed_path_text = managed_path.to_string_lossy().to_string();

    let entry = if let Some(duplicate) = preview.duplicate.as_ref() {
        skills::update_skill_metadata(
            pool,
            &duplicate.existing.id,
            &parsed.name,
            &parsed.description,
            &managed_path_text,
            &parsed.content_hash,
        )
        .await?
    } else {
        skills::create_skill(
            pool,
            &parsed.name,
            &parsed.description,
            &managed_path_text,
            &parsed.content_hash,
        )
        .await?
    };

    let role_ids = role_binding_ids(&scope);
    crate::db::skill_bindings::replace_bindings(
        pool,
        &entry.id,
        scope.all_roles,
        &role_ids,
    )
    .await?;
    apply_scope(pool, &entry.id, &scope).await?;

    Ok(ImportCustomSkillResult {
        status: "imported".to_string(),
        entry: Some(entry),
        preview,
    })
}

#[derive(Debug, Clone)]
struct ParsedSkill {
    name: String,
    description: String,
    content_hash: String,
}

fn load_skill_content(content: Option<&str>) -> Result<String, AppError> {
    // 前端通过浏览器 File API / showDirectoryPicker 读取 SKILL.md 文本后，
    // 经 Tauri command 以 content 形式传入；后端不再从任意 source_path 读盘
    //（消除任意文件读取面，并明确单一数据路径）。
    match content.map(str::trim) {
        Some(text) if !text.is_empty() => Ok(content.unwrap_or_default().to_string()),
        Some(_) => Err(AppError::ValidationError(
            "所选 SKILL.md 文件内容为空".to_string(),
        )),
        None => Err(AppError::ValidationError(
            "请选择一个 SKILL.md 文件或包含 SKILL.md 的目录".to_string(),
        )),
    }
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, AppError> {
    let normalized = content.trim_start_matches('\u{feff}').trim_start();
    let Some(rest) = normalized.strip_prefix("---") else {
        return Err(AppError::ValidationError(
            "SKILL.md 缺少 frontmatter，请确认文件开头包含 name 和 description".to_string(),
        ));
    };
    let Some(end) = rest.find("\n---") else {
        return Err(AppError::ValidationError(
            "SKILL.md frontmatter 格式不完整".to_string(),
        ));
    };
    let frontmatter = &rest[..end];
    let name = frontmatter_value(frontmatter, "name")?;
    let description = frontmatter_value(frontmatter, "description")?;
    validate_skill_name(&name)?;
    Ok(ParsedSkill {
        name,
        description,
        content_hash: content_hash(content),
    })
}

async fn preview_from_parsed(
    pool: &SqlitePool,
    parsed: ParsedSkill,
) -> Result<SkillImportPreview, AppError> {
    let duplicate = if let Some(existing) =
        skills::find_skill_by_content_hash(pool, &parsed.content_hash).await?
    {
        Some(SkillDuplicateInfo {
            kind: "contentHash".to_string(),
            existing,
        })
    } else {
        skills::find_skill_by_name(pool, &parsed.name)
            .await?
            .map(|existing| SkillDuplicateInfo {
                kind: "name".to_string(),
                existing,
            })
    };
    Ok(SkillImportPreview {
        name: parsed.name,
        description: parsed.description,
        content_hash: parsed.content_hash,
        duplicate,
    })
}

fn frontmatter_value(frontmatter: &str, key: &str) -> Result<String, AppError> {
    for line in frontmatter.lines() {
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            continue;
        };
        if raw_key.trim() == key {
            let value = raw_value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }
    }
    Err(AppError::ValidationError(format!(
        "SKILL.md frontmatter 缺少 {} 字段",
        key
    )))
}

fn validate_skill_name(name: &str) -> Result<(), AppError> {
    // name 仅用于 UI 显示与 DB 记录，不再参与文件系统路径构造
    //（受控目录改用 content_hash 派生，见 managed_skill_path），
    // 因此放宽为允许 Unicode 字母/数字/中文，仅拒绝可能破坏展示或路径安全的字符：
    // 控制字符、路径分隔符（/ \）、前后空白边界。
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(
            "Skill name 不能为空".to_string(),
        ));
    }
    if trimmed.len() != name.len() {
        return Err(AppError::ValidationError(
            "Skill name 不能以空白字符开头或结尾".to_string(),
        ));
    }
    let invalid = name
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'));
    if invalid {
        Err(AppError::ValidationError(
            "Skill name 不能包含控制字符或路径分隔符（/ \\）".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn content_hash(content: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn managed_skill_path(skills_root: &Path, _name: &str, content_hash: &str) -> PathBuf {
    // 目录名使用 content_hash 派生，彻底脱钩于用户输入的 name：
    // 既避免 `..`/`.` 路径穿越、大小写不敏感文件系统的副本互相覆盖，
    // 也规避 Windows 保留名（CON/NUL 等）建目录失败。content_hash 由
    // content_hash() 生成，固定 16 位十六进制，文件系统安全。
    skills_root.join(content_hash).join(SKILL_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::tempdir;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                source_type TEXT NOT NULL CHECK(source_type IN ('custom')),
                managed_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create skills table");
        sqlx::query("CREATE UNIQUE INDEX idx_skills_content_hash ON skills(content_hash)")
            .execute(&pool)
            .await
            .expect("failed to create hash index");
        sqlx::query("CREATE UNIQUE INDEX idx_skills_name_source_type ON skills(name, source_type)")
            .execute(&pool)
            .await
            .expect("failed to create name index");
        sqlx::query(
            "CREATE TABLE skill_role_bindings (
                skill_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                PRIMARY KEY (skill_id, role_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create skill bindings table");
        sqlx::query(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create app settings table");

        pool
    }

    fn skill_content(name: &str, description: &str) -> String {
        format!(
            "---\nname: {}\ndescription: {}\n---\n\n# {}\n",
            name, description, name
        )
    }

    #[test]
    fn parse_skill_content_extracts_frontmatter() {
        let parsed = parse_skill_content(&skill_content("daily-review", "日复盘助手")).unwrap();
        assert_eq!(parsed.name, "daily-review");
        assert_eq!(parsed.description, "日复盘助手");
        assert!(!parsed.content_hash.is_empty());
    }

    #[test]
    fn parse_skill_content_returns_friendly_errors() {
        let err = parse_skill_content("# Missing frontmatter").unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(message) if message.contains("frontmatter"))
        );
    }

    #[tokio::test]
    async fn preview_reports_hash_duplicate() {
        let pool = setup_test_db().await;
        let content = skill_content("daily-review", "日复盘助手");
        let first = preview_custom_skill(
            &pool,
            &PreviewCustomSkillInput {
                content: Some(content.clone()),
            },
        )
        .await
        .unwrap();
        skills::create_skill(
            &pool,
            &first.name,
            &first.description,
            "path",
            &first.content_hash,
        )
        .await
        .unwrap();

        let preview = preview_custom_skill(
            &pool,
            &PreviewCustomSkillInput {
                content: Some(content),
            },
        )
        .await
        .unwrap();

        assert_eq!(preview.duplicate.unwrap().kind, "contentHash");
    }

    #[tokio::test]
    async fn import_copies_skill_to_managed_path_and_persists_registry() {
        let pool = setup_test_db().await;
        let dir = tempdir().unwrap();
        let result = import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(skill_content("daily-review", "日复盘助手")),
                overwrite_existing: false,
                role_scope: None,
            },
        )
        .await
        .unwrap();

        let entry = result.entry.unwrap();
        assert_eq!(result.status, "imported");
        assert_eq!(entry.name, "daily-review");
        assert!(Path::new(&entry.managed_path).exists());
        assert_eq!(skills::list_skills(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn import_persists_all_roles_scope() {
        let pool = setup_test_db().await;
        let dir = tempdir().unwrap();
        let result = import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(skill_content("daily-review", "日复盘助手")),
                overwrite_existing: false,
                role_scope: Some(SkillRoleScope {
                    all_roles: true,
                    role_ids: vec!["role-1".to_string()],
                }),
            },
        )
        .await
        .unwrap();
        let entry = result.entry.unwrap();

        assert_eq!(scope_for_skill(&pool, &entry.id).await.unwrap().all_roles, true);
        assert_eq!(list_for_role(&pool, "future-role").await.unwrap(), vec![entry.clone()]);
        assert_eq!(list_all_role_skills(&pool).await.unwrap(), vec![entry]);
    }

    #[tokio::test]
    async fn list_for_role_includes_legacy_enabled_skill_ids_without_binding() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "markitdown",
            "文件与文档转 Markdown",
            "skills/markitdown/SKILL.md",
            "hash-markitdown",
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO roles (id, name, skills_config) VALUES (?1, ?2, ?3)",
        )
        .bind("father-role")
        .bind("父亲")
        .bind(format!(r#"{{"enabledSkillIds":["{}"]}}"#, skill.id))
        .execute(&pool)
        .await
        .unwrap();

        let skills = list_for_role(&pool, "father-role").await.unwrap();

        assert_eq!(skills, vec![skill]);
    }

    #[tokio::test]
    async fn remove_skill_from_role_preserves_registry_and_other_roles() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "executive-briefing",
            "高管简报",
            "skills/executive-briefing/SKILL.md",
            "hash-executive-briefing",
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO roles (id, name, skills_config) VALUES (?1, ?2, ?3)",
        )
        .bind("role-1")
        .bind("产品经理")
        .bind(format!(r#"{{"enabledSkillIds":["{}"]}}"#, skill.id))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO roles (id, name, skills_config) VALUES (?1, ?2, ?3)",
        )
        .bind("role-2")
        .bind("父亲")
        .bind(format!(r#"{{"enabledSkillIds":["{}"]}}"#, skill.id))
        .execute(&pool)
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &skill.id,
            false,
            &["role-1".to_string(), "role-2".to_string()],
        )
        .await
        .unwrap();

        remove_skill_from_role(&pool, &skill.id, "role-1").await.unwrap();

        assert_eq!(skills::list_skills(&pool).await.unwrap(), vec![skill.clone()]);
        assert!(list_for_role(&pool, "role-1").await.unwrap().is_empty());
        assert_eq!(list_for_role(&pool, "role-2").await.unwrap(), vec![skill]);
        let role = crate::db::roles::get_role(&pool, "role-1").await.unwrap();
        assert!(!role.skills_config.contains("executive-briefing"));
    }

    #[tokio::test]
    async fn import_scope_with_butler_enables_butler_without_creating_role_binding() {
        let pool = setup_test_db().await;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        let dir = tempdir().unwrap();
        let result = import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(skill_content("daily-review", "日复盘助手")),
                overwrite_existing: false,
                role_scope: Some(SkillRoleScope {
                    all_roles: false,
                    role_ids: vec!["__butler__".to_string()],
                }),
            },
        )
        .await
        .unwrap();
        let entry = result.entry.unwrap();

        let butler_skills = crate::services::butler_config::get_butler_skills(&pool).await.unwrap();
        assert_eq!(butler_skills.enabled_skill_ids, vec![entry.id.clone()]);
        assert!(scope_for_skill(&pool, &entry.id).await.unwrap().role_ids.is_empty());
        assert_eq!(list_all_role_skills(&pool).await.unwrap(), vec![entry]);
    }

    #[tokio::test]
    async fn import_all_roles_scope_includes_butler() {
        let pool = setup_test_db().await;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        let dir = tempdir().unwrap();
        let result = import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(skill_content("daily-review", "日复盘助手")),
                overwrite_existing: false,
                role_scope: Some(SkillRoleScope {
                    all_roles: true,
                    role_ids: vec![],
                }),
            },
        )
        .await
        .unwrap();
        let entry = result.entry.unwrap();

        let butler_skills = crate::services::butler_config::get_butler_skills(&pool).await.unwrap();
        assert_eq!(butler_skills.enabled_skill_ids, vec![entry.id]);
    }

    #[tokio::test]
    async fn import_duplicate_without_overwrite_does_not_create_second_entry() {
        let pool = setup_test_db().await;
        let dir = tempdir().unwrap();
        let content = skill_content("daily-review", "日复盘助手");
        import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(content.clone()),
                overwrite_existing: false,
                role_scope: None,
            },
        )
        .await
        .unwrap();

        let duplicate = import_custom_skill(
            &pool,
            dir.path(),
            &ImportCustomSkillInput {
                content: Some(content),
                overwrite_existing: false,
                role_scope: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(duplicate.status, "duplicate");
        assert_eq!(skills::list_skills(&pool).await.unwrap().len(), 1);
    }
}
