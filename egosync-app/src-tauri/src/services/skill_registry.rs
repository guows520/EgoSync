use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::db::skills;
use crate::error::AppError;
use crate::models::skill::{
    DiscoverOpencodeSkillsResult, ImportCustomSkillInput, ImportCustomSkillResult,
    ImportOpencodeSkillInput, ImportOpencodeSkillResult, OpencodeSkillCandidate,
    OpencodeSkillSkippedSummary, PreviewCustomSkillInput, SkillDuplicateInfo, SkillImportPreview,
    SkillRegistryEntry, SelectableSkill, SkillRoleScope, BUTLER_SCOPE_ID, SOURCE_TYPE_CUSTOM, SOURCE_TYPE_OPENCODE,
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

/// Story 10.1: 解析当前 Agent 作用域下指定 Skill 是否存在、已添加且启用。
/// `role_id = None` 表示管家作用域；`role_id = Some` 表示指定角色作用域。
/// 失败时返回明确领域错误（`SkillNotFound` / `SkillNotAddedToScope` / `SkillDisabled`），
/// 不进入 Agent Runtime（AC-3, AC-4）。
pub async fn resolve_enabled(
    pool: &SqlitePool,
    role_id: Option<&str>,
    skill_id: &str,
) -> Result<SkillRegistryEntry, AppError> {
    let entry = skills::get_skill(pool, skill_id)
        .await
        .map_err(|e| match e {
            AppError::NotFound(_) => AppError::SkillNotFound(skill_id.to_string()),
            other => other,
        })?;

    let enabled_ids = match role_id {
        None => crate::services::butler_config::get_butler_skills(pool)
            .await
            .map_err(|e| AppError::DbError(format!("读取管家 Skill 配置失败: {}", e)))?
            .enabled_skill_ids,
        Some(rid) => {
            // 先验证角色已添加该 Skill（绑定表）
            let bound_ids = crate::db::skill_bindings::skill_ids_for_role(pool, rid)
                .await
                .map_err(|e| AppError::DbError(format!("查询角色 Skill 绑定失败: {}", e)))?;
            if !bound_ids.iter().any(|id| id == skill_id) {
                return Err(AppError::SkillNotAddedToScope(format!(
                    "Skill {} 未添加到角色 {}",
                    skill_id, rid
                )));
            }
            // 再验证该 Skill 在角色 enabledSkillIds 中
            let role = crate::db::roles::get_role(pool, rid)
                .await
                .map_err(|e| AppError::DbError(format!("查询角色失败: {}", e)))?;
            crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config)
        }
    };

    if !enabled_ids.iter().any(|id| id == skill_id) {
        return Err(AppError::SkillDisabled(format!(
            "Skill {} 未在当前作用域启用",
            skill_id
        )));
    }
    Ok(entry)
}

const META_FIND_SKILLS_KEY: &str = "meta:find-skills";
const META_SKILL_CREATOR_KEY: &str = "meta:skill-creator";

fn registry_selectable(entry: SkillRegistryEntry) -> SelectableSkill {
    SelectableSkill {
        key: format!("registry:{}", entry.id),
        name: entry.name,
        description: entry.description,
        kind: "registry".to_string(),
        source_type: entry.source_type,
    }
}

fn meta_selectable(key: &str, name: &str, description: &str) -> SelectableSkill {
    SelectableSkill {
        key: key.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        kind: "meta".to_string(),
        source_type: "meta".to_string(),
    }
}

async fn scope_meta_flags(pool: &SqlitePool, role_id: Option<&str>) -> Result<(bool, bool), AppError> {
    match role_id {
        None => {
            let skills = crate::services::butler_config::get_butler_skills(pool)
                .await
                .map_err(|e| AppError::DbError(format!("读取管家 Skill 配置失败: {}", e)))?;
            Ok((skills.find_skills, skills.skill_creator))
        }
        Some(rid) => {
            let role = crate::db::roles::get_role(pool, rid)
                .await
                .map_err(|e| AppError::DbError(format!("查询角色失败: {}", e)))?;
            let skills = crate::services::role_config::skills_from_config(&role.skills_config);
            Ok((skills.find_skills, skills.skill_creator))
        }
    }
}

pub async fn list_selectable(
    pool: &SqlitePool,
    role_id: Option<&str>,
) -> Result<Vec<SelectableSkill>, AppError> {
    let mut items = list_enabled(pool, role_id)
        .await?
        .into_iter()
        .map(registry_selectable)
        .collect::<Vec<_>>();
    let (find_skills, skill_creator) = scope_meta_flags(pool, role_id).await?;
    if find_skills {
        items.push(meta_selectable(
            META_FIND_SKILLS_KEY,
            "find-skills",
            "发现并推荐适合当前任务的 Skill。",
        ));
    }
    if skill_creator {
        items.push(meta_selectable(
            META_SKILL_CREATOR_KEY,
            "skill-creator",
            "创建或扩展当前 Agent 需要的新 Skill。",
        ));
    }
    Ok(items)
}

pub async fn resolve_selectable(
    pool: &SqlitePool,
    role_id: Option<&str>,
    key: &str,
) -> Result<SelectableSkill, AppError> {
    if let Some(skill_id) = key.strip_prefix("registry:") {
        if skill_id.is_empty() {
            return Err(AppError::SkillNotFound(key.to_string()));
        }
        return resolve_enabled(pool, role_id, skill_id).await.map(registry_selectable);
    }

    let (find_skills, skill_creator) = scope_meta_flags(pool, role_id).await?;
    match key {
        META_FIND_SKILLS_KEY if find_skills => Ok(meta_selectable(
            META_FIND_SKILLS_KEY,
            "find-skills",
            "发现并推荐适合当前任务的 Skill。",
        )),
        META_SKILL_CREATOR_KEY if skill_creator => Ok(meta_selectable(
            META_SKILL_CREATOR_KEY,
            "skill-creator",
            "创建或扩展当前 Agent 需要的新 Skill。",
        )),
        META_FIND_SKILLS_KEY | META_SKILL_CREATOR_KEY => Err(AppError::SkillDisabled(format!(
            "Skill {} 未在当前作用域启用",
            key
        ))),
        _ => Err(AppError::SkillNotFound(key.to_string())),
    }
}

// Story 10.1: 返回当前 Agent 作用域下已添加且启用的 Skill 列表（AC-1）。
/// 管家分支返回 `butler.enabled_skill_ids` 对应的 Skill；角色分支返回
/// 角色绑定与 `enabledSkillIds` 交集对应的 Skill。前端候选查询复用此方法，
/// 不自行拼装集合（后端是授权边界）。
pub async fn list_enabled(
    pool: &SqlitePool,
    role_id: Option<&str>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    let enabled_ids = match role_id {
        None => crate::services::butler_config::get_butler_skills(pool)
            .await
            .map_err(|e| AppError::DbError(format!("读取管家 Skill 配置失败: {}", e)))?
            .enabled_skill_ids,
        Some(rid) => {
            let role = crate::db::roles::get_role(pool, rid)
                .await
                .map_err(|e| AppError::DbError(format!("查询角色失败: {}", e)))?;
            let enabled = crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config);
            let bound_ids = crate::db::skill_bindings::skill_ids_for_role(pool, rid)
                .await
                .map_err(|e| AppError::DbError(format!("查询角色 Skill 绑定失败: {}", e)))?;
            enabled
                .into_iter()
                .filter(|id| bound_ids.iter().any(|b| b == id))
                .collect::<Vec<_>>()
        }
    };
    filter_registry_by_ids(pool, &enabled_ids).await
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
    if role_id == BUTLER_SCOPE_ID {
        let mut skills = crate::services::butler_config::get_butler_skills(pool).await?;
        skills.enabled_skill_ids.retain(|id| id != skill_id);
        crate::services::butler_config::set_butler_skills_config(pool, &skills).await?;
        return Ok(());
    }

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

    // Butler 不在 roles 表中，必须单独清理；否则创建后的同步回滚会留下死 id。
    let mut butler_skills = crate::services::butler_config::get_butler_skills(pool).await?;
    if butler_skills.enabled_skill_ids.iter().any(|id| id == skill_id) {
        butler_skills.enabled_skill_ids.retain(|id| id != skill_id);
        crate::services::butler_config::set_butler_skills_config(pool, &butler_skills).await?;
    }
    Ok(affected)
}

/// Upgrade hash-based managed paths created by older EgoSync versions to the
/// `<name>/SKILL.md` layout required by OpenCode discovery.
pub async fn migrate_legacy_managed_paths(
    pool: &SqlitePool,
    skills_root: &Path,
) -> Result<(), AppError> {
    let registry = skills::list_skills(pool).await?;
    for entry in &registry {
        if registry.iter().filter(|item| item.name == entry.name).count() != 1
            || validate_skill_name(&entry.name).is_err()
        {
            continue;
        }
        let expected = managed_skill_path(skills_root, &entry.name, &entry.content_hash);
        let current = PathBuf::from(&entry.managed_path);
        if current == expected {
            continue;
        }
        let content = std::fs::read_to_string(&current).map_err(|e| {
            AppError::ValidationError(format!("迁移 Skill {} 失败：{}", entry.name, e))
        })?;
        if content_hash(&content) != entry.content_hash {
            return Err(AppError::ValidationError(format!(
                "迁移 Skill {} 失败：受控文件内容已变化",
                entry.name
            )));
        }
        if expected.exists() {
            let existing = std::fs::read_to_string(&expected).map_err(|e| {
                AppError::ValidationError(format!("读取 Skill {} 新路径失败：{}", entry.name, e))
            })?;
            if content_hash(&existing) != entry.content_hash {
                return Err(AppError::ValidationError(format!(
                    "迁移 Skill {} 失败：目标路径已被不同内容占用",
                    entry.name
                )));
            }
        } else {
            std::fs::create_dir_all(expected.parent().unwrap_or(skills_root))
                .map_err(|e| AppError::ValidationError(format!("创建 Skill 迁移目录失败：{}", e)))?;
            std::fs::write(&expected, &content)
                .map_err(|e| AppError::ValidationError(format!("写入 Skill 迁移文件失败：{}", e)))?;
        }
        skills::update_skill_metadata(
            pool,
            &entry.id,
            &entry.name,
            &entry.description,
            &expected.to_string_lossy(),
            &entry.content_hash,
        )
        .await?;
        if let Some(parent) = current.parent() {
            if parent.starts_with(skills_root)
                && parent != skills_root
                && parent != expected.parent().unwrap_or(skills_root)
            {
                let _ = std::fs::remove_dir_all(parent);
            }
        }
    }
    Ok(())
}

/// Add one owner without replacing the Skill's existing role scope.
pub async fn enable_skill_for_owner(
    pool: &SqlitePool,
    skill_id: &str,
    owner_id: &str,
) -> Result<(), AppError> {
    if owner_id == BUTLER_SCOPE_ID {
        let mut config = crate::services::butler_config::get_butler_skills(pool).await?;
        if !config.enabled_skill_ids.iter().any(|id| id == skill_id) {
            config.enabled_skill_ids.push(skill_id.to_string());
            crate::services::butler_config::set_butler_skills_config(pool, &config).await?;
        }
        return Ok(());
    }

    let mut bindings = crate::db::skill_bindings::role_ids_for_skill(pool, skill_id).await?;
    let all_roles = bindings
        .iter()
        .any(|id| id == crate::db::skill_bindings::ALL_ROLES_BINDING);
    if !all_roles && !bindings.iter().any(|id| id == owner_id) {
        bindings.push(owner_id.to_string());
        crate::db::skill_bindings::replace_bindings(pool, skill_id, false, &bindings).await?;
    }
    let role = crate::db::roles::get_role(pool, owner_id).await?;
    let mut config = crate::services::role_config::skills_from_config(&role.skills_config);
    let mut ids = config.enabled_skill_ids.unwrap_or_default();
    if !ids.iter().any(|id| id == skill_id) {
        ids.push(skill_id.to_string());
        config.enabled_skill_ids = Some(ids);
        let raw = crate::services::role_config::normalize_skills_config_with_existing(
            &role.skills_config,
            &config,
        )?;
        crate::db::roles::set_role_skills_config_raw(pool, owner_id, &raw).await?;
    }
    Ok(())
}

pub async fn preview_custom_skill(
    pool: &SqlitePool,
    input: &PreviewCustomSkillInput,
) -> Result<SkillImportPreview, AppError> {
    let content = load_skill_content(input.content.as_deref())?;
    let parsed = parse_skill_content(&content)?;
    preview_from_parsed(pool, parsed).await
}

pub async fn discover_opencode_skills(
    pool: &SqlitePool,
    role_id: &str,
    project_dir: &Path,
    home_dir: &Path,
) -> Result<DiscoverOpencodeSkillsResult, AppError> {
    let scope_skill_ids: Vec<String> = if role_id == BUTLER_SCOPE_ID {
        crate::services::butler_config::get_butler_skills(pool)
            .await?
            .enabled_skill_ids
    } else {
        list_for_role(pool, role_id)
            .await?
            .into_iter()
            .map(|skill| skill.id)
            .collect()
    };
    let mut items = Vec::new();
    let mut reasons = Vec::new();
    scan_opencode_root(
        pool,
        &project_dir.join(".opencode").join("skills"),
        "项目级",
        &scope_skill_ids,
        &mut items,
        &mut reasons,
    )
    .await?;
    scan_opencode_root(
        pool,
        &home_dir.join(".config").join("opencode").join("skills"),
        "全局",
        &scope_skill_ids,
        &mut items,
        &mut reasons,
    )
    .await?;
    Ok(DiscoverOpencodeSkillsResult {
        items,
        skipped: OpencodeSkillSkippedSummary {
            total: reasons.len(),
            reasons,
        },
    })
}

async fn scan_opencode_root(
    pool: &SqlitePool,
    root: &Path,
    source_location: &str,
    role_skill_ids: &[String],
    items: &mut Vec<OpencodeSkillCandidate>,
    reasons: &mut Vec<String>,
) -> Result<(), AppError> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Ok(());
    };
    let registry = skills::list_skills(pool).await?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                reasons.push("存在不可读取的 Skill 条目".to_string());
                continue;
            }
        };
        let Ok(file_type) = entry.file_type() else {
            reasons.push("存在不可读取的 Skill 条目".to_string());
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let skill_path = entry.path().join(SKILL_FILE_NAME);
        if !skill_path.exists() {
            reasons.push("缺少 SKILL.md 的条目已跳过".to_string());
            continue;
        }
        if registry.iter().any(|existing| {
            existing.source_type == SOURCE_TYPE_CUSTOM
                && paths_point_to_same_file(Path::new(&existing.managed_path), &skill_path)
        }) {
            continue;
        }
        let content = match std::fs::read_to_string(&skill_path) {
            Ok(content) => content,
            Err(_) => {
                reasons.push("不可读取的 SKILL.md 已跳过".to_string());
                continue;
            }
        };
        let parsed = match parse_skill_content(&content) {
            Ok(parsed) => parsed,
            // 任何解析错误（含非 ValidationError）都只跳过当前条目，绝不中断整个
            // 扫描——否则项目级目录中的一个坏 Skill 会让全局目录永远扫不到。
            Err(e) => {
                reasons.push(skip_reason_for(&parsed_skill_name(&content), &e));
                continue;
            }
        };
        let preview =
            match preview_from_parsed_with_source(pool, parsed.clone(), SOURCE_TYPE_OPENCODE).await
            {
            Ok(preview) => preview,
            Err(AppError::ValidationError(message)) => {
                reasons.push(message);
                continue;
            }
            Err(error) => return Err(error),
        };
        let already_imported = preview
            .duplicate
            .as_ref()
            .is_some_and(|duplicate| role_skill_ids.iter().any(|id| id == &duplicate.existing.id));
        items.push(OpencodeSkillCandidate {
            name: parsed.name,
            description: parsed.description,
            source_location: source_location.to_string(),
            source_path: skill_path.to_string_lossy().to_string(),
            source_type: SOURCE_TYPE_OPENCODE.to_string(),
            content_hash: parsed.content_hash,
            already_imported,
            duplicate: preview.duplicate,
        });
    }
    Ok(())
}

fn paths_point_to_same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// 尽力从 SKILL.md frontmatter 中取出 name 作为跳过摘要的定位标识；
/// 解析不出 name 时返回 None，调用方退化为通用文案。
fn parsed_skill_name(content: &str) -> Option<String> {
    let normalized = content.trim_start_matches('\u{feff}').trim_start();
    let rest = normalized.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    frontmatter_value(&rest[..end], "name").ok()
}

/// 为被跳过的无效 Skill 构造友好中文摘要：尽量带上 name 便于用户定位，
/// 不暴露底层堆栈（AC3）。
fn skip_reason_for(name: &Option<String>, err: &AppError) -> String {
    let detail = match err {
        AppError::ValidationError(message) => message.clone(),
        _ => "解析 SKILL.md 失败".to_string(),
    };
    match name {
        Some(name) if !name.is_empty() => format!("{}（{}）", detail, name),
        _ => detail,
    }
}

pub async fn import_opencode_skill(
    pool: &SqlitePool,
    input: &ImportOpencodeSkillInput,
    skills_root: &Path,
    project_dir: &Path,
    home_dir: &Path,
) -> Result<ImportOpencodeSkillResult, AppError> {
    let source_path = PathBuf::from(input.source_path.trim());
    if source_path.file_name().and_then(|name| name.to_str()) != Some(SKILL_FILE_NAME) {
        return Err(AppError::ValidationError("只能导入扫描到的 SKILL.md".to_string()));
    }
    if !is_allowed_opencode_skill_path(&source_path, project_dir, home_dir) {
        return Err(AppError::ValidationError("只能导入 opencode Skill 目录中的本地 SKILL.md".to_string()));
    }
    let content = std::fs::read_to_string(&source_path)
        .map_err(|_| AppError::ValidationError("所选 opencode Skill 不可读取".to_string()))?;
    let parsed = parse_skill_content(&content)?;
    // TOCTOU 一致性校验：若源文件在 discover→import 之间被替换，content_hash 会变，
    // 此时拒绝导入并提示重新发现，避免静默导入与 UI 展示不符的内容。
    if let Some(expected) = input.expected_content_hash.as_ref() {
        if expected != &parsed.content_hash {
            return Err(AppError::ValidationError(
                "所选 Skill 已发生变更，请重新发现后再导入".to_string(),
            ));
        }
    }
    let preview = preview_from_parsed_with_source(pool, parsed.clone(), SOURCE_TYPE_OPENCODE).await?;
    let scope = normalized_scope(input.role_scope.as_ref());
    if let Some(duplicate) = preview.duplicate.as_ref() {
        return finalize_opencode_binding(pool, duplicate.existing.clone(), &scope, "duplicate").await;
    }
    // 将 opencode SKILL.md 复制到 EgoSync 受控目录（与 2.11 自定义 Skill 一致）。
    // 受控目录即 opencode 项目级 skills 根，副本天然被 agent 自动发现（AC5），
    // 且源文件被删/改后 registry 仍指向稳定副本（AC4 持久性）。
    let managed_path = managed_skill_path(skills_root, &parsed.name, &parsed.content_hash);
    let source_dir = source_path.parent()
        .ok_or_else(|| AppError::ValidationError("opencode Skill 源目录无效".to_string()))?;
    replace_managed_skill_directory(source_dir, managed_path.parent().unwrap_or(skills_root))?;
    let managed_path_text = managed_path.to_string_lossy().to_string();
    let entry = match skills::create_skill_with_source(
        pool,
        &parsed.name,
        &parsed.description,
        &managed_path_text,
        &parsed.content_hash,
        SOURCE_TYPE_OPENCODE,
    )
    .await
    {
        Ok(entry) => entry,
        // 并发导入竞态：preview 阶段未见重复，但两个 INSERT 竞争唯一索引时
        // 第二个会撞 content_hash / (name, source_type) 约束。重新查重并退化为
        // duplicate 优雅处理，而非把 DB 唯一约束错误抛给用户。
        Err(AppError::DbError(_)) => {
            if let Some(existing) =
                skills::find_skill_by_content_hash(pool, &parsed.content_hash).await?
            {
                return finalize_opencode_binding(pool, existing, &scope, "duplicate").await;
            }
            if let Some(existing) =
                skills::find_skill_by_name_and_source(pool, &parsed.name, SOURCE_TYPE_OPENCODE)
                    .await?
            {
                return finalize_opencode_binding(pool, existing, &scope, "duplicate").await;
            }
            return Err(AppError::DbError("导入 opencode Skill 写入失败".to_string()));
        }
        Err(e) => return Err(e),
    };
    finalize_opencode_binding(pool, entry, &scope, "imported").await
}

/// 统一收口 opencode Skill 的角色绑定与 scope 落地，避免 imported/duplicate
/// 两条路径重复写绑定逻辑。
async fn finalize_opencode_binding(
    pool: &SqlitePool,
    entry: SkillRegistryEntry,
    scope: &SkillRoleScope,
    status: &str,
) -> Result<ImportOpencodeSkillResult, AppError> {
    let role_ids = role_binding_ids(scope);
    crate::db::skill_bindings::replace_bindings(pool, &entry.id, scope.all_roles, &role_ids).await?;
    apply_scope(pool, &entry.id, scope).await?;
    Ok(ImportOpencodeSkillResult {
        status: status.to_string(),
        entry: Some(entry),
        // registry 与绑定已落地；opencode agent 同步在 command 层执行，
        // synced 默认 true，command 层在 full_sync 失败时下调为 false。
        synced: true,
        runtime_ready: false,
        runtime_error: None,
    })
}

fn is_allowed_opencode_skill_path(source_path: &Path, project_dir: &Path, home_dir: &Path) -> bool {
    let Ok(source) = source_path.canonicalize() else {
        return false;
    };
    let roots = [
        project_dir.join(".opencode").join("skills"),
        home_dir.join(".config").join("opencode").join("skills"),
    ];
    roots.iter().any(|root| {
        let Ok(root) = root.canonicalize() else {
            return false;
        };
        let Ok(relative) = source.strip_prefix(&root) else {
            return false;
        };
        relative.components().count() == 2 && source.file_name().and_then(|name| name.to_str()) == Some(SKILL_FILE_NAME)
    })
}

pub async fn import_custom_skill(
    pool: &SqlitePool,
    skills_root: &Path,
    input: &ImportCustomSkillInput,
) -> Result<ImportCustomSkillResult, AppError> {
    let source_dir = input.source_path.as_deref().map(PathBuf::from);
    let content = if let Some(source_dir) = source_dir.as_ref() {
        let source_content = std::fs::read_to_string(source_dir.join(SKILL_FILE_NAME))
            .map_err(|e| AppError::ValidationError(format!("读取源 Skill 失败: {}", e)))?;
        if input.content.as_deref().is_some_and(|previewed| previewed != source_content) {
            return Err(AppError::ValidationError("Skill 文件已变更，请重新预览后导入".to_string()));
        }
        source_content
    } else {
        load_skill_content(input.content.as_deref())?
    };
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
                runtime_ready: false,
                runtime_error: None,
            });
        }
    }

    let managed_path = managed_skill_path(skills_root, &parsed.name, &parsed.content_hash);
    if let Some(source_dir) = source_dir.as_ref() {
        replace_managed_skill_directory(source_dir, managed_path.parent().unwrap_or(skills_root))?;
    } else {
        if let Some(parent) = managed_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::ValidationError(format!("创建受控 Skill 目录失败: {}", e)))?;
        }
        std::fs::write(&managed_path, &content)
            .map_err(|e| AppError::ValidationError(format!("保存 Skill 文件失败: {}", e)))?;
    }
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
        runtime_ready: false,
        runtime_error: None,
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
    preview_from_parsed_with_source(pool, parsed, crate::models::skill::SOURCE_TYPE_CUSTOM).await
}

async fn preview_from_parsed_with_source(
    pool: &SqlitePool,
    parsed: ParsedSkill,
    source_type: &str,
) -> Result<SkillImportPreview, AppError> {
    if let Some(existing) = skills::list_skills(pool)
        .await?
        .into_iter()
        .find(|entry| entry.name == parsed.name && entry.source_type != source_type)
    {
        return Err(AppError::ValidationError(format!(
            "Skill 名称 {} 已被另一来源占用（{}）",
            parsed.name, existing.source_type
        )));
    }
    let duplicate = if let Some(existing) =
        skills::find_skill_by_content_hash(pool, &parsed.content_hash).await?
    {
        Some(SkillDuplicateInfo {
            kind: "contentHash".to_string(),
            existing,
        })
    } else {
        skills::find_skill_by_name_and_source(pool, &parsed.name, source_type)
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
    let bytes = name.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && !name.contains("--");
    if !valid {
        return Err(AppError::ValidationError(
            "Skill name 必须为 1-64 位小写字母、数字或单个连字符，且不能以连字符开头或结尾".to_string(),
        ));
    }
    if matches!(name, "con" | "prn" | "aux" | "nul" | "com1" | "com2" | "com3" | "com4" | "com5" | "com6" | "com7" | "com8" | "com9" | "lpt1" | "lpt2" | "lpt3" | "lpt4" | "lpt5" | "lpt6" | "lpt7" | "lpt8" | "lpt9") {
        return Err(AppError::ValidationError("Skill name 不能使用系统保留名".to_string()));
    }
    Ok(())
}

fn content_hash(content: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn copy_skill_directory(source: &Path, destination: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(destination).map_err(|e| AppError::ValidationError(format!("创建受控 Skill 目录失败: {}", e)))?;
    for entry in std::fs::read_dir(source).map_err(|e| AppError::ValidationError(format!("读取 Skill 目录失败: {}", e)))? {
        let entry = entry.map_err(|e| AppError::ValidationError(format!("读取 Skill 条目失败: {}", e)))?;
        let file_type = entry.file_type().map_err(|e| AppError::ValidationError(format!("读取 Skill 条目类型失败: {}", e)))?;
        if file_type.is_symlink() { return Err(AppError::ValidationError("Skill 目录不能包含符号链接".to_string())); }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() { copy_skill_directory(&entry.path(), &target)?; }
        else if file_type.is_file() { std::fs::copy(entry.path(), target).map_err(|e| AppError::ValidationError(format!("复制 Skill 文件失败: {}", e)))?; }
    }
    Ok(())
}

fn replace_managed_skill_directory(source: &Path, destination: &Path) -> Result<(), AppError> {
    let temp = destination.with_extension("importing");
    if temp.exists() { std::fs::remove_dir_all(&temp).map_err(|e| AppError::ValidationError(format!("清理临时 Skill 目录失败: {}", e)))?; }
    copy_skill_directory(source, &temp)?;
    if destination.exists() { std::fs::remove_dir_all(destination).map_err(|e| AppError::ValidationError(format!("替换受控 Skill 目录失败: {}", e)))?; }
    std::fs::rename(temp, destination).map_err(|e| AppError::ValidationError(format!("保存 Skill 目录失败: {}", e)))
}

fn managed_skill_path(skills_root: &Path, name: &str, _content_hash: &str) -> PathBuf {
    // OpenCode 只发现 `.opencode/skills/<name>/SKILL.md`，且目录名必须与
    // frontmatter name 一致。validate_skill_name 已在到达这里前完成路径安全校验。
    skills_root.join(name).join(SKILL_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::tempdir;

    #[test]
    fn copy_skill_directory_preserves_scripts() {
        let source = tempdir().unwrap();
        std::fs::write(source.path().join(SKILL_FILE_NAME), skill_content("ppt-generation", "生成演示文稿")).unwrap();
        std::fs::create_dir_all(source.path().join("scripts")).unwrap();
        std::fs::write(source.path().join("scripts").join("generate.py"), "print('ok')").unwrap();
        let destination = tempdir().unwrap();
        let target = destination.path().join("ppt-generation");
        copy_skill_directory(source.path(), &target).unwrap();
        assert!(target.join(SKILL_FILE_NAME).is_file());
        assert!(target.join("scripts").join("generate.py").is_file());
    }

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
                source_type TEXT NOT NULL CHECK(source_type IN ('custom', 'opencode')),
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
                energy_updated_at TEXT,
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

    #[test]
    fn skill_name_matches_opencode_discovery_contract() {
        // WHY: OpenCode discovers a Skill by its frontmatter name and matching
        // directory, so accepting aliases that cannot become that directory
        // would make a successfully imported Skill impossible to load.
        for valid in ["uat-greeting", "a", "skill2"] {
            validate_skill_name(valid).unwrap();
        }
        for invalid in ["UAT-Greeting", "uat_greeting", "-skill", "skill-", "skill--name", "con"] {
            assert!(validate_skill_name(invalid).is_err(), "{invalid} must be rejected");
        }
    }

    #[test]
    fn managed_skill_path_uses_frontmatter_name_directory() {
        let root = Path::new("workspace/.opencode/skills");
        assert_eq!(
            managed_skill_path(root, "uat-greeting", "ignored-hash"),
            root.join("uat-greeting").join("SKILL.md")
        );
    }

    #[tokio::test]
    async fn enabling_duplicate_for_owner_preserves_existing_binding() {
        let pool = setup_test_db().await;
        for (id, name) in [("role-1", "原角色"), ("role-2", "新角色")] {
            sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
                .bind(id)
                .bind(name)
                .execute(&pool)
                .await
                .unwrap();
        }
        let entry = skills::create_skill(
            &pool,
            "uat-greeting",
            "问候语",
            "managed/uat-greeting/SKILL.md",
            "hash-1",
        )
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &entry.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();

        enable_skill_for_owner(&pool, &entry.id, "role-2").await.unwrap();

        assert_eq!(
            crate::db::skill_bindings::role_ids_for_skill(&pool, &entry.id)
                .await
                .unwrap(),
            vec!["role-1".to_string(), "role-2".to_string()]
        );
        let role = crate::db::roles::get_role(&pool, "role-2").await.unwrap();
        assert!(crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config)
            .contains(&entry.id));
    }

    #[tokio::test]
    async fn migrates_legacy_hash_directory_to_name_directory() {
        let pool = setup_test_db().await;
        let root = tempdir().unwrap();
        let content = skill_content("uat-greeting", "问候语");
        let hash = content_hash(&content);
        let legacy = root.path().join(&hash).join("SKILL.md");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, &content).unwrap();
        let entry = skills::create_skill(
            &pool,
            "uat-greeting",
            "问候语",
            &legacy.to_string_lossy(),
            &hash,
        )
        .await
        .unwrap();

        migrate_legacy_managed_paths(&pool, root.path()).await.unwrap();

        let migrated = skills::get_skill(&pool, &entry.id).await.unwrap();
        let expected = root.path().join("uat-greeting").join("SKILL.md");
        assert_eq!(PathBuf::from(migrated.managed_path), expected);
        assert!(expected.exists());
        assert!(!legacy.exists());
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
                source_path: None,
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
                source_path: None,
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
                source_path: None,
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
                source_path: None,
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
                source_path: None,
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
                source_path: None,
                overwrite_existing: false,
                role_scope: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(duplicate.status, "duplicate");
        assert_eq!(skills::list_skills(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn discover_opencode_skills_scans_only_project_and_global_skill_roots() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();
        let project_skill_dir = project.path().join(".opencode").join("skills").join("daily-review");
        std::fs::create_dir_all(&project_skill_dir).unwrap();
        std::fs::write(
            project_skill_dir.join("SKILL.md"),
            skill_content("daily-review", "日复盘助手"),
        )
        .unwrap();
        let global_skill_dir = home.path().join(".config").join("opencode").join("skills").join("writer");
        std::fs::create_dir_all(&global_skill_dir).unwrap();
        std::fs::write(global_skill_dir.join("SKILL.md"), skill_content("writer", "写作助手")).unwrap();
        let invalid_dir = project.path().join(".opencode").join("skills").join("invalid");
        std::fs::create_dir_all(&invalid_dir).unwrap();
        std::fs::write(invalid_dir.join("README.md"), "not a skill").unwrap();
        let nested_dir = project_skill_dir.join("nested").join("hidden");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(nested_dir.join("SKILL.md"), skill_content("hidden", "不应被扫描")).unwrap();

        let result = discover_opencode_skills(&pool, "role-1", project.path(), home.path()).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert!(result.items.iter().any(|item| item.name == "daily-review" && item.source_type == "opencode" && item.source_location == "项目级"));
        assert!(result.items.iter().any(|item| item.name == "writer" && item.source_type == "opencode" && item.source_location == "全局"));
        assert_eq!(result.skipped.total, 1);
        assert!(result.items.iter().all(|item| item.name != "hidden"));
    }

    #[tokio::test]
    async fn discover_opencode_skills_ignores_managed_custom_skill_and_keeps_scanning() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();

        let custom_dir = project
            .path()
            .join(".opencode")
            .join("skills")
            .join("uat-weekly-report");
        std::fs::create_dir_all(&custom_dir).unwrap();
        let custom_path = custom_dir.join(SKILL_FILE_NAME);
        let custom_content = skill_content("uat-weekly-report", "生成周报摘要的自定义技能");
        std::fs::write(&custom_path, &custom_content).unwrap();
        let custom = parse_skill_content(&custom_content).unwrap();
        skills::create_skill(
            &pool,
            &custom.name,
            &custom.description,
            custom_path.to_string_lossy().as_ref(),
            &custom.content_hash,
        )
        .await
        .unwrap();

        let global_dir = home
            .path()
            .join(".config")
            .join("opencode")
            .join("skills")
            .join("writer");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(
            global_dir.join(SKILL_FILE_NAME),
            skill_content("writer", "写作助手"),
        )
        .unwrap();

        let result = discover_opencode_skills(&pool, "role-1", project.path(), home.path())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "writer");
        assert_eq!(result.skipped.total, 0);
    }

    #[tokio::test]
    async fn discover_opencode_skills_skips_cross_source_name_conflict() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        skills::create_skill(
            &pool,
            "writer",
            "内部写作助手",
            "managed/writer/SKILL.md",
            "custom-writer-hash",
        )
        .await
        .unwrap();
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();

        let project_dir = project
            .path()
            .join(".opencode")
            .join("skills")
            .join("daily-review");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(
            project_dir.join(SKILL_FILE_NAME),
            skill_content("daily-review", "日复盘助手"),
        )
        .unwrap();
        let global_dir = home
            .path()
            .join(".config")
            .join("opencode")
            .join("skills")
            .join("writer");
        std::fs::create_dir_all(&global_dir).unwrap();
        std::fs::write(
            global_dir.join(SKILL_FILE_NAME),
            skill_content("writer", "外部写作助手"),
        )
        .unwrap();

        let result = discover_opencode_skills(&pool, "role-1", project.path(), home.path())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "daily-review");
        assert_eq!(result.skipped.total, 1);
        assert!(result.skipped.reasons[0].contains("已被另一来源占用"));
    }

    #[tokio::test]
    async fn scan_opencode_root_keeps_database_failures_explicit() {
        let pool = setup_test_db().await;
        let root = tempdir().unwrap();
        pool.close().await;
        let mut items = Vec::new();
        let mut reasons = Vec::new();

        let error = scan_opencode_root(&pool, root.path(), "项目级", &[], &mut items, &mut reasons)
            .await
            .unwrap_err();

        assert!(matches!(error, AppError::DbError(_)));
    }

    #[tokio::test]
    async fn discover_opencode_skill_marks_imported_only_for_current_role() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2), (?3, ?4)")
            .bind("role-1")
            .bind("产品经理")
            .bind("role-2")
            .bind("学习者")
            .execute(&pool)
            .await
            .unwrap();
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();
        let skill_dir = project.path().join(".opencode").join("skills").join("writer");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let source_path = skill_dir.join("SKILL.md");
        let content = skill_content("writer", "写作助手");
        std::fs::write(&source_path, &content).unwrap();
        let parsed = parse_skill_content(&content).unwrap();
        let entry = skills::create_skill_with_source(
            &pool,
            "writer",
            "写作助手",
            source_path.to_string_lossy().as_ref(),
            &parsed.content_hash,
            SOURCE_TYPE_OPENCODE,
        )
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &entry.id,
            false,
            &["role-2".to_string()],
        )
        .await
        .unwrap();

        let result = discover_opencode_skills(&pool, "role-1", project.path(), home.path()).await.unwrap();

        assert_eq!(result.items.len(), 1);
        let candidate = &result.items[0];
        assert_eq!(candidate.name, "writer");
        assert!(!candidate.already_imported);
        assert_eq!(candidate.duplicate.as_ref().unwrap().existing.id, entry.id);
    }

    #[tokio::test]
    async fn discover_opencode_skill_marks_imported_for_butler_scope() {
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
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();
        let skill_dir = project.path().join(".opencode").join("skills").join("writer");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let source_path = skill_dir.join("SKILL.md");
        let content = skill_content("writer", "写作助手");
        std::fs::write(&source_path, &content).unwrap();
        let parsed = parse_skill_content(&content).unwrap();
        let entry = skills::create_skill_with_source(
            &pool,
            "writer",
            "写作助手",
            source_path.to_string_lossy().as_ref(),
            &parsed.content_hash,
            SOURCE_TYPE_OPENCODE,
        )
        .await
        .unwrap();
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![entry.id.clone()],
            },
        )
        .await
        .unwrap();

        let result = discover_opencode_skills(&pool, BUTLER_SCOPE_ID, project.path(), home.path()).await.unwrap();

        assert_eq!(result.items.len(), 1);
        let candidate = &result.items[0];
        assert_eq!(candidate.name, "writer");
        assert!(candidate.already_imported);
        assert_eq!(candidate.duplicate.as_ref().unwrap().existing.id, entry.id);
    }

    #[tokio::test]
    async fn remove_skill_from_butler_scope_preserves_registry_and_role_bindings() {
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
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let skill = skills::create_skill(
            &pool,
            "daily-review",
            "日复盘助手",
            "managed/daily-review/SKILL.md",
            "hash-1",
        )
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &skill.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![skill.id.clone()],
            },
        )
        .await
        .unwrap();

        remove_skill_from_role(&pool, &skill.id, BUTLER_SCOPE_ID).await.unwrap();

        assert_eq!(skills::list_skills(&pool).await.unwrap(), vec![skill.clone()]);
        assert_eq!(list_for_role(&pool, "role-1").await.unwrap(), vec![skill]);
        let butler_skills = crate::services::butler_config::get_butler_skills(&pool).await.unwrap();
        assert!(butler_skills.enabled_skill_ids.is_empty());
    }

    #[tokio::test]
    async fn import_opencode_skill_registers_source_type_and_applies_scope() {
        let pool = setup_test_db().await;
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();
        let managed_root = tempdir().unwrap();
        let skills_root = managed_root.path().join(".opencode").join("skills");
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let skill_dir = project.path().join(".opencode").join("skills").join("daily-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let source_path = skill_dir.join("SKILL.md");
        std::fs::write(&source_path, skill_content("daily-review", "日复盘助手")).unwrap();

        let result = import_opencode_skill(
            &pool,
            &ImportOpencodeSkillInput {
                source_path: source_path.to_string_lossy().to_string(),
                role_scope: Some(SkillRoleScope {
                    all_roles: false,
                    role_ids: vec!["role-1".to_string()],
                }),
                expected_content_hash: None,
            },
            &skills_root,
            project.path(),
            home.path(),
        )
        .await
        .unwrap();

        let entry = result.entry.unwrap();
        assert_eq!(result.status, "imported");
        assert!(result.synced);
        assert_eq!(entry.source_type, "opencode");
        // managed_path 现指向 EgoSync 受控副本（skills_root 下），而非外部源文件，
        // 且副本内容已落盘，源文件删除后仍可用（AC4 持久性）。
        assert_ne!(entry.managed_path, source_path.to_string_lossy());
        assert!(Path::new(&entry.managed_path).starts_with(&skills_root));
        assert!(Path::new(&entry.managed_path).exists());
        std::fs::remove_file(&source_path).unwrap();
        assert!(Path::new(&entry.managed_path).exists());
        assert_eq!(list_for_role(&pool, "role-1").await.unwrap(), vec![entry.clone()]);
        let role = crate::db::roles::get_role(&pool, "role-1").await.unwrap();
        assert!(role.skills_config.contains(&entry.id));
    }

    #[tokio::test]
    async fn import_opencode_skill_rejects_when_source_changed_after_discover() {
        let pool = setup_test_db().await;
        let project = tempdir().unwrap();
        let home = tempdir().unwrap();
        let managed_root = tempdir().unwrap();
        let skills_root = managed_root.path().join(".opencode").join("skills");
        let skill_dir = project.path().join(".opencode").join("skills").join("daily-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let source_path = skill_dir.join("SKILL.md");
        std::fs::write(&source_path, skill_content("daily-review", "日复盘助手")).unwrap();

        // 模拟 discover→import 之间源文件被替换：携带旧 hash 导入应被拒绝。
        let result = import_opencode_skill(
            &pool,
            &ImportOpencodeSkillInput {
                source_path: source_path.to_string_lossy().to_string(),
                role_scope: None,
                expected_content_hash: Some("stale-hash".to_string()),
            },
            &skills_root,
            project.path(),
            home.path(),
        )
        .await;

        assert!(matches!(result, Err(AppError::ValidationError(_))));
        assert_eq!(skills::list_skills(&pool).await.unwrap().len(), 0);
    }

    // ----- Story 10.1: SkillAvailabilityService 作用域校验 -----

    /// WHY: AC-1 候选集合隔离 — 管家与角色必须各自读取自己的配置，
    /// 不合并对方的作用域。若 list_enabled 把管家与角色的 enabled 集合合并，
    /// 用户会在管家对话中看到角色专属 Skill，破坏可信授权边界。
    #[tokio::test]
    async fn list_enabled_isolates_butler_and_role_scopes() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();

        let butler_skill = skills::create_skill(
            &pool,
            "butler-only-skill",
            "管家专属",
            "managed/butler-only-skill/SKILL.md",
            "hash-butler",
        )
        .await
        .unwrap();
        let role_skill = skills::create_skill(
            &pool,
            "role-only-skill",
            "角色专属",
            "managed/role-only-skill/SKILL.md",
            "hash-role",
        )
        .await
        .unwrap();

        // 管家启用 butler_skill，不启用 role_skill
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![butler_skill.id.clone()],
            },
        )
        .await
        .unwrap();

        // 角色绑定并启用 role_skill，不绑定 butler_skill
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &role_skill.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();
        enable_skill_for_owner(&pool, &role_skill.id, "role-1")
            .await
            .unwrap();

        let butler_enabled = list_enabled(&pool, None).await.unwrap();
        let role_enabled = list_enabled(&pool, Some("role-1")).await.unwrap();

        // 管家候选只含 butler_skill，不含角色专属 Skill
        assert_eq!(butler_enabled.len(), 1);
        assert_eq!(butler_enabled[0].id, butler_skill.id);

        // 角色候选只含 role_skill，不含管家专属 Skill
        assert_eq!(role_enabled.len(), 1);
        assert_eq!(role_enabled[0].id, role_skill.id);
    }

    /// WHY: AC-4 失效竞态拒绝 — Skill 不存在时必须在进入 Runtime 前被拒绝，
    /// 否则会向 opencode 发送一个不存在的 command name，产生不可控错误。
    #[tokio::test]
    async fn resolve_enabled_returns_skill_not_found_for_missing_skill() {
        let pool = setup_test_db().await;
        let err = resolve_enabled(&pool, None, "nonexistent-skill")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::SkillNotFound(_)));
    }

    /// WHY: AC-4 — 角色未绑定该 Skill 时拒绝（Skill 存在但未添加到作用域）。
    /// 这防止用户通过手动构造请求调用角色未授权的 Skill。
    #[tokio::test]
    async fn resolve_enabled_returns_not_added_to_scope_for_unbound_role_skill() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let skill = skills::create_skill(
            &pool,
            "unbound-skill",
            "未绑定",
            "managed/unbound-skill/SKILL.md",
            "hash-unbound",
        )
        .await
        .unwrap();
        // 不调用 replace_bindings，角色未绑定该 Skill

        let err = resolve_enabled(&pool, Some("role-1"), &skill.id)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::SkillNotAddedToScope(_)));
    }

    /// WHY: AC-4 — Skill 已绑定到角色但未在 enabledSkillIds 中（被关闭）时拒绝。
    /// 这覆盖"选择后、发送前被关闭"的竞态：用户前端看到的是旧候选快照，
    /// 后端必须基于当前配置重新校验，拒绝已禁用的 Skill。
    #[tokio::test]
    async fn resolve_enabled_returns_disabled_for_bound_but_disabled_role_skill() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let skill = skills::create_skill(
            &pool,
            "bound-disabled-skill",
            "已绑定但禁用",
            "managed/bound-disabled-skill/SKILL.md",
            "hash-bound-disabled",
        )
        .await
        .unwrap();
        // 绑定但不启用
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &skill.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();

        let err = resolve_enabled(&pool, Some("role-1"), &skill.id)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::SkillDisabled(_)));
    }

    /// WHY: AC-4 — 管家作用域下 Skill 未在 enabled_skill_ids 中时拒绝。
    /// 管家没有单独的绑定表，enabled_skill_ids 即"已添加且启用"的单一来源。
    #[tokio::test]
    async fn resolve_enabled_returns_disabled_for_butler_unenabled_skill() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "butler-disabled",
            "管家未启用",
            "managed/butler-disabled/SKILL.md",
            "hash-butler-disabled",
        )
        .await
        .unwrap();
        // 管家 enabled_skill_ids 为空（默认）

        let err = resolve_enabled(&pool, None, &skill.id)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::SkillDisabled(_)));
    }

    /// WHY: AC-3 — 有效选择（存在、已添加、已启用）必须返回 SkillRegistryEntry，
    /// 供 agent_engine 取 name 调用 opencode command。这是唯一进入 Runtime 的路径。
    #[tokio::test]
    async fn resolve_enabled_returns_entry_for_valid_butler_skill() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "butler-active",
            "管家启用中",
            "managed/butler-active/SKILL.md",
            "hash-butler-active",
        )
        .await
        .unwrap();
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![skill.id.clone()],
            },
        )
        .await
        .unwrap();

        let resolved = resolve_enabled(&pool, None, &skill.id).await.unwrap();
        assert_eq!(resolved.id, skill.id);
        assert_eq!(resolved.name, "butler-active");
    }

    /// WHY: AC-3 — 角色有效选择（绑定 + 启用）必须返回 entry。
    #[tokio::test]
    async fn resolve_enabled_returns_entry_for_valid_role_skill() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let skill = skills::create_skill(
            &pool,
            "role-active",
            "角色启用中",
            "managed/role-active/SKILL.md",
            "hash-role-active",
        )
        .await
        .unwrap();
        enable_skill_for_owner(&pool, &skill.id, "role-1")
            .await
            .unwrap();

        let resolved = resolve_enabled(&pool, Some("role-1"), &skill.id)
            .await
            .unwrap();
        assert_eq!(resolved.id, skill.id);
        assert_eq!(resolved.name, "role-active");
    }

    /// WHY: AC-1 — list_enabled 对角色返回"已绑定且启用"的交集，
    /// 仅绑定未启用或仅启用未绑定的 Skill 都不应出现，避免前端展示不可用候选。
    #[tokio::test]
    async fn list_enabled_for_role_returns_intersection_of_bound_and_enabled() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();

        let bound_enabled = skills::create_skill(
            &pool,
            "bound-enabled",
            "绑定且启用",
            "managed/bound-enabled/SKILL.md",
            "hash-be",
        )
        .await
        .unwrap();
        let bound_only = skills::create_skill(
            &pool,
            "bound-only",
            "仅绑定",
            "managed/bound-only/SKILL.md",
            "hash-bo",
        )
        .await
        .unwrap();
        let enabled_only = skills::create_skill(
            &pool,
            "enabled-only",
            "仅启用",
            "managed/enabled-only/SKILL.md",
            "hash-eo",
        )
        .await
        .unwrap();

        // bound_enabled: 绑定 + 启用
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &bound_enabled.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();
        enable_skill_for_owner(&pool, &bound_enabled.id, "role-1")
            .await
            .unwrap();

        // bound_only: 仅绑定，不启用
        crate::db::skill_bindings::replace_bindings(
            &pool,
            &bound_only.id,
            false,
            &["role-1".to_string()],
        )
        .await
        .unwrap();

        // enabled_only: 仅启用（写入 enabledSkillIds），不绑定。
        // 注意：必须追加到现有 enabled_skill_ids，而非覆盖，否则
        // enable_skill_for_owner 之前写入的 bound_enabled 会被丢掉。
        let role = crate::db::roles::get_role(&pool, "role-1").await.unwrap();
        let mut config = crate::services::role_config::skills_from_config(&role.skills_config);
        let mut existing = config.enabled_skill_ids.unwrap_or_default();
        existing.push(enabled_only.id.clone());
        config.enabled_skill_ids = Some(existing);
        let raw = crate::services::role_config::normalize_skills_config_with_existing(
            &role.skills_config,
            &config,
        )
        .unwrap();
        crate::db::roles::set_role_skills_config_raw(&pool, "role-1", &raw)
            .await
            .unwrap();

        let enabled = list_enabled(&pool, Some("role-1")).await.unwrap();
        let ids: Vec<_> = enabled.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&bound_enabled.id.as_str()));
        assert!(!ids.contains(&bound_only.id.as_str()));
        assert!(!ids.contains(&enabled_only.id.as_str()));
    }

    /// WHY: AC-1 — 当前 Agent 无可用 Skill 时 list_enabled 返回空集合，
    /// 前端据此展示明确空状态（AC-2），而非错误。
    #[tokio::test]
    async fn list_enabled_returns_empty_when_no_skills_configured() {
        let pool = setup_test_db().await;
        let butler = list_enabled(&pool, None).await.unwrap();
        assert!(butler.is_empty());

        sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
            .bind("role-1")
            .bind("产品经理")
            .execute(&pool)
            .await
            .unwrap();
        let role = list_enabled(&pool, Some("role-1")).await.unwrap();
        assert!(role.is_empty());
    }

    /// WHY: `@` 候选必须由同一后端授权模型同时组合普通 registry Skill 与已启用 meta Skill，
    /// 否则前端即使显示 meta 项也会与发送时权限真源分叉。
    #[tokio::test]
    async fn list_selectable_combines_registry_and_enabled_meta_for_current_scope() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "butler-report",
            "管家报告",
            "managed/butler-report/SKILL.md",
            "hash-butler-report",
        )
        .await
        .unwrap();
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![skill.id.clone()],
            },
        )
        .await
        .unwrap();

        let items = list_selectable(&pool, None).await.unwrap();
        assert!(items.iter().any(|item| item.key == format!("registry:{}", skill.id)));
        assert!(items.iter().any(|item| item.key == META_FIND_SKILLS_KEY));
        assert!(!items.iter().any(|item| item.key == META_SKILL_CREATOR_KEY));
    }

    /// WHY: 候选列表不是授权边界；配置在选择后被关闭时，统一 key 必须在 Runtime 调用前重新拒绝。
    #[tokio::test]
    async fn resolve_selectable_authorizes_registry_and_meta_keys_against_current_scope() {
        let pool = setup_test_db().await;
        let skill = skills::create_skill(
            &pool,
            "butler-report",
            "管家报告",
            "managed/butler-report/SKILL.md",
            "hash-butler-report",
        )
        .await
        .unwrap();
        crate::services::butler_config::set_butler_skills_config(
            &pool,
            &crate::models::role::ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: vec![skill.id.clone()],
            },
        )
        .await
        .unwrap();

        let registry = resolve_selectable(&pool, None, &format!("registry:{}", skill.id))
            .await
            .unwrap();
        assert_eq!(registry.name, "butler-report");
        let meta = resolve_selectable(&pool, None, META_FIND_SKILLS_KEY).await.unwrap();
        assert_eq!(meta.name, "find-skills");
        assert!(matches!(
            resolve_selectable(&pool, None, META_SKILL_CREATOR_KEY).await.unwrap_err(),
            AppError::SkillDisabled(_)
        ));
        assert!(matches!(
            resolve_selectable(&pool, None, "meta:forged").await.unwrap_err(),
            AppError::SkillNotFound(_)
        ));
    }

}
