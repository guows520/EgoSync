use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db::{
    app_settings, briefings, conversations, mcp_servers, memories, mission, notifications,
    roles, settings, suggestions, tasks, weekly_reviews,
};
use crate::error::AppError;
use crate::models::briefing::Briefing;
use crate::models::chat::{Conversation, Message};
use crate::models::mcp::McpServer;
use crate::models::memory::Memory;
use crate::models::mission::Mission;
use crate::models::notification::NotificationWithRole;
use crate::models::role::Role;
use crate::models::settings::LlmConfig;
use crate::models::suggestion::Suggestion;
use crate::models::task::CrossRoleTask;
use crate::models::weekly_review::WeeklyReview;

const EXPORT_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Sqlite,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub roles: Vec<Role>,
    pub tasks: Vec<CrossRoleTask>,
    pub memories: Vec<Memory>,
    pub suggestions: Vec<Suggestion>,
    pub notifications: Vec<NotificationWithRole>,
    pub mission: Option<Mission>,
    pub conflicts: Vec<serde_json::Value>,
    pub briefings: Vec<Briefing>,
    pub weekly_reviews: Vec<WeeklyReview>,
    pub llm_configs: Vec<LlmConfig>,
    pub app_settings: Vec<(String, Option<String>)>,
    pub mcp_servers: Vec<McpServer>,
    pub skills: Vec<serde_json::Value>,
    pub skill_bindings: Vec<serde_json::Value>,
    pub q2_reminders: Vec<serde_json::Value>,
    pub big_rock_protection_reminders: Vec<serde_json::Value>,
    pub conversations: Vec<Conversation>,
    pub messages: Vec<Message>,
    pub exported_at: String,
    pub export_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub files: Vec<String>,
    pub sqlite_path: Option<String>,
    pub json_path: Option<String>,
    pub markdown_path: Option<String>,
}

fn export_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn io_error(context: &str, e: std::io::Error) -> AppError {
    AppError::ValidationError(format!("{}: {}", context, e))
}

pub fn export_sqlite(app_data_dir: &Path, dir_path: &Path) -> Result<Vec<String>, AppError> {
    let date = export_date();
    let mut files = Vec::new();

    let main_src = app_data_dir.join("egosync.db");
    let main_dst = dir_path.join(format!("egosync-export-{}.db", date));
    if main_src.exists() {
        std::fs::copy(&main_src, &main_dst)
            .map_err(|e| io_error("导出 egosync.db 失败", e))?;
        files.push(main_dst.to_string_lossy().to_string());
    }

    let conv_src = app_data_dir.join("conversations.db");
    let conv_dst = dir_path.join(format!("egosync-export-{}-conversations.db", date));
    if conv_src.exists() {
        std::fs::copy(&conv_src, &conv_dst)
            .map_err(|e| io_error("导出 conversations.db 失败", e))?;
        files.push(conv_dst.to_string_lossy().to_string());
    }

    if files.is_empty() {
        return Err(AppError::ValidationError(
            "未找到可导出的数据库文件".to_string(),
        ));
    }

    Ok(files)
}

async fn query_conflicts(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, task_id_a, task_id_b, role_id_a, role_id_b, conflict_time, status, resolution, created_at
         FROM conflicts ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询冲突记录失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "taskIdA": row.get::<String, _>("task_id_a"),
                "taskIdB": row.get::<String, _>("task_id_b"),
                "roleIdA": row.get::<String, _>("role_id_a"),
                "roleIdB": row.get::<String, _>("role_id_b"),
                "conflictTime": row.get::<String, _>("conflict_time"),
                "status": row.get::<String, _>("status"),
                "resolution": row.get::<Option<String>, _>("resolution"),
                "createdAt": row.get::<String, _>("created_at"),
            })
        })
        .collect())
}

async fn query_skills(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, description, source_type, managed_path, content_hash, created_at, updated_at
         FROM skills ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Skill 注册表失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<String, _>("description"),
                "sourceType": row.get::<String, _>("source_type"),
                "managedPath": row.get::<String, _>("managed_path"),
                "contentHash": row.get::<String, _>("content_hash"),
                "createdAt": row.get::<String, _>("created_at"),
                "updatedAt": row.get::<String, _>("updated_at"),
            })
        })
        .collect())
}

async fn query_skill_bindings(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT skill_id, role_id, created_at FROM skill_role_bindings ORDER BY skill_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Skill 绑定失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "skillId": row.get::<String, _>("skill_id"),
                "roleId": row.get::<String, _>("role_id"),
                "createdAt": row.get::<String, _>("created_at"),
            })
        })
        .collect())
}

async fn query_q2_reminders(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, task_id, reminded_count, last_reminded_at, created_at
         FROM q2_reminders ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Q2 提醒记录失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "taskId": row.get::<String, _>("task_id"),
                "remindedCount": row.get::<i64, _>("reminded_count"),
                "lastRemindedAt": row.get::<String, _>("last_reminded_at"),
                "createdAt": row.get::<String, _>("created_at"),
            })
        })
        .collect())
}

async fn query_big_rock_protection_reminders(
    pool: &DbPool,
) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, task_id, reminded_count, last_reminded_at, created_at
         FROM big_rock_protection_reminders ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询大石头保护提醒记录失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "taskId": row.get::<String, _>("task_id"),
                "remindedCount": row.get::<i64, _>("reminded_count"),
                "lastRemindedAt": row.get::<String, _>("last_reminded_at"),
                "createdAt": row.get::<String, _>("created_at"),
            })
        })
        .collect())
}

pub async fn gather_export_data(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
) -> Result<ExportData, AppError> {
    let roles = roles::list_all_roles(pool).await?;
    let tasks = tasks::list_all_tasks(pool, None, None).await?;
    let memories = memories::list_all_memories(pool).await?;
    let suggestions = suggestions::list_all_suggestions(pool).await?;
    let notifications = notifications::list_notifications(pool).await?;
    let mission = mission::get_mission(pool).await?;
    let conflicts = query_conflicts(pool).await?;
    let briefings = briefings::list_all_briefings(pool).await?;
    let weekly_reviews = weekly_reviews::list_all_weekly_reviews(pool).await?;
    let llm_configs = settings::list_llm_configs(pool).await?;
    let app_settings = app_settings::get_all_settings(pool).await?;
    let mcp_servers = mcp_servers::list_mcp_servers(pool).await?;
    let skills = query_skills(pool).await?;
    let skill_bindings = query_skill_bindings(pool).await?;
    let q2_reminders = query_q2_reminders(pool).await?;
    let big_rock_protection_reminders = query_big_rock_protection_reminders(pool).await?;
    let conversations = conversations::list_all_conversations(conv_pool).await?;
    let messages = conversations::list_all_messages(conv_pool).await?;

    Ok(ExportData {
        roles,
        tasks,
        memories,
        suggestions,
        notifications,
        mission,
        conflicts,
        briefings,
        weekly_reviews,
        llm_configs,
        app_settings,
        mcp_servers,
        skills,
        skill_bindings,
        q2_reminders,
        big_rock_protection_reminders,
        conversations,
        messages,
        exported_at: crate::db::settings::chrono_now_pub(),
        export_version: EXPORT_VERSION.to_string(),
    })
}

pub async fn export_json(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    dir_path: &Path,
) -> Result<String, AppError> {
    let data = gather_export_data(pool, conv_pool).await?;
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| AppError::ValidationError(format!("JSON 序列化失败: {}", e)))?;

    let file_path = dir_path.join(format!("egosync-export-{}.json", export_date()));
    std::fs::write(&file_path, json)
        .map_err(|e| io_error("写入 JSON 导出文件失败", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

pub fn generate_markdown(export_data: &ExportData) -> String {
    let mut md = String::new();

    md.push_str("# EgoSync 数据导出\n\n");
    md.push_str(&format!(
        "- 导出时间: {}\n- 导出版本: {}\n\n",
        export_data.exported_at, export_data.export_version
    ));

    md.push_str("## 统计摘要\n\n");
    md.push_str(&format!(
        "| 数据 | 数量 |\n|------|------|\n| 角色 | {} |\n| 任务 | {} |\n| 记忆 | {} |\n| 建议 | {} |\n| 通知 | {} |\n| 冲突记录 | {} |\n| 简报 | {} |\n| 周复盘 | {} |\n| LLM 配置 | {} |\n| MCP 服务器 | {} |\n| Skill | {} |\n| 对话 | {} |\n| 消息 | {} |\n\n",
        export_data.roles.len(),
        export_data.tasks.len(),
        export_data.memories.len(),
        export_data.suggestions.len(),
        export_data.notifications.len(),
        export_data.conflicts.len(),
        export_data.briefings.len(),
        export_data.weekly_reviews.len(),
        export_data.llm_configs.len(),
        export_data.mcp_servers.len(),
        export_data.skills.len(),
        export_data.conversations.len(),
        export_data.messages.len(),
    ));

    let messages_by_conv: std::collections::HashMap<&str, Vec<&Message>> = {
        let mut map: std::collections::HashMap<&str, Vec<&Message>> =
            std::collections::HashMap::new();
        for msg in &export_data.messages {
            map.entry(&msg.conversation_id).or_default().push(msg);
        }
        map
    };

    // 管家部分（与角色格式一致，放在所有角色之前）
    md.push_str("## 管家\n\n");

    let butler_tasks: Vec<&CrossRoleTask> = export_data
        .tasks
        .iter()
        .filter(|t| t.owner_type == "butler")
        .collect();
    md.push_str("### 任务列表\n\n");
    if butler_tasks.is_empty() {
        md.push_str("（暂无任务）\n\n");
    } else {
        for task in butler_tasks {
            let status = if task.is_completed { "已完成" } else { "未完成" };
            let big_rock = if task.is_big_rock { " [大石头]" } else { "" };
            let deadline = task.deadline.as_deref().unwrap_or("无");
            md.push_str(&format!(
                "- **{}** — 象限: {} | 状态: {}{} | 截止: {}\n",
                task.title, task.quadrant, status, big_rock, deadline,
            ));
        }
        md.push('\n');
    }

    let butler_memories: Vec<&Memory> = export_data
        .memories
        .iter()
        .filter(|m| m.role_id.is_none())
        .collect();
    md.push_str("### 记忆条目\n\n");
    if butler_memories.is_empty() {
        md.push_str("（暂无记忆）\n\n");
    } else {
        for mem in butler_memories {
            md.push_str(&format!(
                "- **[{}]** {} _({})_\n",
                mem.category, mem.content, mem.created_at,
            ));
        }
        md.push('\n');
    }

    let butler_conversations: Vec<&Conversation> = export_data
        .conversations
        .iter()
        .filter(|c| c.role_id.is_none())
        .collect();
    md.push_str("### 对话记录\n\n");
    if butler_conversations.is_empty() {
        md.push_str("（暂无对话）\n\n");
    } else {
        for conv in butler_conversations {
            let conv_msgs = messages_by_conv.get(conv.id.as_str());
            let msg_count = conv_msgs.map(|v| v.len()).unwrap_or(0);
            let title = if conv.title.is_empty() {
                "无标题"
            } else {
                conv.title.as_str()
            };
            md.push_str(&format!(
                "#### {} — 消息数: {} | 最近更新: {}\n\n",
                title, msg_count, conv.updated_at,
            ));
            if let Some(msgs) = conv_msgs {
                for msg in msgs {
                    let speaker = match msg.role.as_str() {
                        "user" => "用户",
                        "assistant" => "助手",
                        other => other,
                    };
                    md.push_str(&format!(
                        "**{}** ({})\n\n{}\n\n",
                        speaker, msg.created_at, msg.content,
                    ));
                    if !msg.thinking_content.is_empty() {
                        md.push_str(&format!(
                            "<details><summary>思考过程</summary>\n\n{}\n\n</details>\n\n",
                            msg.thinking_content,
                        ));
                    }
                }
            }
        }
        md.push('\n');
    }

    // 各角色部分
    for role in &export_data.roles {
        md.push_str(&format!("## {}\n\n", role.name));

        md.push_str("### 角色信息\n\n");
        md.push_str(&format!(
            "- 图标: {}\n- 颜色: {}\n- 目标: {}\n- 能量: {}\n- 主动性级别: {}\n- 状态: {}\n\n",
            role.icon,
            role.color,
            role.goal,
            role.energy,
            role.proactivity_level,
            role.status,
        ));

        let role_tasks: Vec<&CrossRoleTask> = export_data
            .tasks
            .iter()
            .filter(|t| t.role_id.as_deref() == Some(&role.id))
            .collect();
        md.push_str("### 任务列表\n\n");
        if role_tasks.is_empty() {
            md.push_str("（暂无任务）\n\n");
        } else {
            for task in role_tasks {
                let status = if task.is_completed { "已完成" } else { "未完成" };
                let big_rock = if task.is_big_rock { " [大石头]" } else { "" };
                let deadline = task.deadline.as_deref().unwrap_or("无");
                md.push_str(&format!(
                    "- **{}** — 象限: {} | 状态: {}{} | 截止: {}\n",
                    task.title, task.quadrant, status, big_rock, deadline,
                ));
            }
            md.push('\n');
        }

        let role_memories: Vec<&Memory> = export_data
            .memories
            .iter()
            .filter(|m| m.role_id.as_deref() == Some(&role.id))
            .collect();
        md.push_str("### 记忆条目\n\n");
        if role_memories.is_empty() {
            md.push_str("（暂无记忆）\n\n");
        } else {
            for mem in role_memories {
                md.push_str(&format!(
                    "- **[{}]** {} _({})_\n",
                    mem.category, mem.content, mem.created_at,
                ));
            }
            md.push('\n');
        }

        let role_conversations: Vec<&Conversation> = export_data
            .conversations
            .iter()
            .filter(|c| c.role_id.as_deref() == Some(&role.id))
            .collect();
        md.push_str("### 对话记录\n\n");
        if role_conversations.is_empty() {
            md.push_str("（暂无对话）\n\n");
        } else {
            for conv in role_conversations {
                let conv_msgs = messages_by_conv.get(conv.id.as_str());
                let msg_count = conv_msgs.map(|v| v.len()).unwrap_or(0);
                let title = if conv.title.is_empty() {
                    "无标题"
                } else {
                    conv.title.as_str()
                };
                md.push_str(&format!(
                    "#### {} — 消息数: {} | 最近更新: {}\n\n",
                    title, msg_count, conv.updated_at,
                ));
                if let Some(msgs) = conv_msgs {
                    for msg in msgs {
                        let speaker = match msg.role.as_str() {
                            "user" => "用户",
                            "assistant" => "助手",
                            other => other,
                        };
                        md.push_str(&format!(
                            "**{}** ({})\n\n{}\n\n",
                            speaker, msg.created_at, msg.content,
                        ));
                        if !msg.thinking_content.is_empty() {
                            md.push_str(&format!(
                                "<details><summary>思考过程</summary>\n\n{}\n\n</details>\n\n",
                                msg.thinking_content,
                            ));
                        }
                    }
                }
            }
            md.push('\n');
        }
    }

    md.push_str("## 全局数据\n\n");

    if let Some(mission) = &export_data.mission {
        md.push_str("### 使命宣言\n\n");
        if let Some(content) = &mission.content {
            md.push_str(content);
        } else {
            md.push_str("（未设置）");
        }
        md.push_str("\n\n");
    }

    if !export_data.briefings.is_empty() {
        md.push_str("### 简报\n\n");
        for b in &export_data.briefings {
            md.push_str(&format!("- **{}**: {}\n", b.date, b.content));
        }
        md.push('\n');
    }

    if !export_data.weekly_reviews.is_empty() {
        md.push_str("### 周复盘\n\n");
        for wr in &export_data.weekly_reviews {
            md.push_str(&format!(
                "- **{} ~ {}**: {}\n",
                wr.week_start, wr.week_end, wr.summary
            ));
        }
        md.push('\n');
    }

    if !export_data.llm_configs.is_empty() {
        md.push_str("### LLM 配置\n\n");
        for cfg in &export_data.llm_configs {
            let default = if cfg.is_default { " (默认)" } else { "" };
            md.push_str(&format!(
                "- **{}** — {} / {}{}\n",
                cfg.name, cfg.provider, cfg.model, default
            ));
        }
        md.push('\n');
    }

    if !export_data.mcp_servers.is_empty() {
        md.push_str("### MCP 服务器\n\n");
        for srv in &export_data.mcp_servers {
            let enabled = if srv.enabled { "已启用" } else { "已禁用" };
            md.push_str(&format!(
                "- **{}** — {} ({})\n",
                srv.name, srv.server_type, enabled
            ));
        }
        md.push('\n');
    }

    if !export_data.app_settings.is_empty() {
        md.push_str("### 应用设置\n\n");
        for (key, value) in &export_data.app_settings {
            let val = value.as_deref().unwrap_or("（空）");
            md.push_str(&format!("- `{}`: {}\n", key, val));
        }
        md.push('\n');
    }

    md
}

pub async fn export_markdown(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    dir_path: &Path,
) -> Result<String, AppError> {
    let data = gather_export_data(pool, conv_pool).await?;
    let markdown = generate_markdown(&data);

    let file_path = dir_path.join(format!("egosync-export-{}.md", export_date()));
    std::fs::write(&file_path, markdown)
        .map_err(|e| io_error("写入 Markdown 导出文件失败", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

pub async fn export_all(
    app_data_dir: &Path,
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    dir_path: &Path,
    formats: Vec<ExportFormat>,
) -> Result<ExportResult, AppError> {
    let mut files = Vec::new();
    let mut sqlite_path = None;
    let mut json_path = None;
    let mut markdown_path = None;

    for fmt in &formats {
        match fmt {
            ExportFormat::Sqlite => {
                if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                    .execute(pool)
                    .await
                {
                    tracing::warn!("主数据库 WAL checkpoint 失败: {}", e);
                }
                if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                    .execute(&**conv_pool)
                    .await
                {
                    tracing::warn!("对话数据库 WAL checkpoint 失败: {}", e);
                }
                let sqlite_files = export_sqlite(app_data_dir, dir_path)?;
                if let Some(first) = sqlite_files.first() {
                    sqlite_path = Some(first.clone());
                }
                files.extend(sqlite_files);
            }
            ExportFormat::Json => {
                let path = export_json(pool, conv_pool, dir_path).await?;
                json_path = Some(path.clone());
                files.push(path);
            }
            ExportFormat::Markdown => {
                let path = export_markdown(pool, conv_pool, dir_path).await?;
                markdown_path = Some(path.clone());
                files.push(path);
            }
        }
    }

    Ok(ExportResult {
        files,
        sqlite_path,
        json_path,
        markdown_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_markdown_with_empty_data_produces_valid_structure() {
        let data = ExportData {
            roles: vec![],
            tasks: vec![],
            memories: vec![],
            suggestions: vec![],
            notifications: vec![],
            mission: None,
            conflicts: vec![],
            briefings: vec![],
            weekly_reviews: vec![],
            llm_configs: vec![],
            app_settings: vec![],
            mcp_servers: vec![],
            skills: vec![],
            skill_bindings: vec![],
            q2_reminders: vec![],
            big_rock_protection_reminders: vec![],
            conversations: vec![],
            messages: vec![],
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            export_version: "1.0".to_string(),
        };

        let md = generate_markdown(&data);
        assert!(md.contains("# EgoSync 数据导出"));
        assert!(md.contains("## 统计摘要"));
        assert!(md.contains("导出时间: 2026-01-01T00:00:00Z"));
        assert!(md.contains("导出版本: 1.0"));
    }

    #[test]
    fn generate_markdown_includes_role_sections() {
        let role = Role {
            id: "role-1".to_string(),
            name: "产品经理".to_string(),
            icon: "🎯".to_string(),
            color: "#FF0000".to_string(),
            goal: "打造好产品".to_string(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy: 80,
            energy_updated_at: None,
            skills_config: "{}".to_string(),
            proactivity_level: "medium".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let task = CrossRoleTask {
            id: "task-1".to_string(),
            owner_type: "role".to_string(),
            role_id: Some("role-1".to_string()),
            title: "写PRD".to_string(),
            deadline: Some("2026-06-30".to_string()),
            quadrant: "q1".to_string(),
            is_big_rock: true,
            is_completed: false,
            completed_at: None,
            sort_order: 0,
            protection_status: "normal".to_string(),
            confidence: None,
            manual_override: false,
            classification_reason: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            deleted_at: None,
            role_name: Some("产品经理".to_string()),
            role_color: Some("#FF0000".to_string()),
        };

        let memory = Memory {
            id: "mem-1".to_string(),
            role_id: Some("role-1".to_string()),
            category: "偏好".to_string(),
            content: "喜欢简洁的设计".to_string(),
            source_conversation_id: "conv-1".to_string(),
            source_message_ids: "msg-1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let data = ExportData {
            roles: vec![role],
            tasks: vec![task],
            memories: vec![memory],
            suggestions: vec![],
            notifications: vec![],
            mission: None,
            conflicts: vec![],
            briefings: vec![],
            weekly_reviews: vec![],
            llm_configs: vec![],
            app_settings: vec![],
            mcp_servers: vec![],
            skills: vec![],
            skill_bindings: vec![],
            q2_reminders: vec![],
            big_rock_protection_reminders: vec![],
            conversations: vec![],
            messages: vec![],
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            export_version: "1.0".to_string(),
        };

        let md = generate_markdown(&data);
        assert!(md.contains("## 管家"));
        assert!(md.contains("### 任务列表"));
        assert!(md.contains("（暂无任务）"));
        assert!(md.contains("### 记忆条目"));
        assert!(md.contains("（暂无记忆）"));
        assert!(md.contains("### 对话记录"));
        assert!(md.contains("（暂无对话）"));
        assert!(md.contains("## 产品经理"));
        assert!(md.contains("### 角色信息"));
        assert!(md.contains("打造好产品"));
        assert!(md.contains("### 任务列表"));
        assert!(md.contains("**写PRD**"));
        assert!(md.contains("[大石头]"));
        assert!(md.contains("### 记忆条目"));
        assert!(md.contains("喜欢简洁的设计"));
    }

    #[test]
    fn generate_markdown_includes_conversation_summary() {
        let role = Role {
            id: "role-1".to_string(),
            name: "产品经理".to_string(),
            icon: "🎯".to_string(),
            color: "#FF0000".to_string(),
            goal: String::new(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy: 50,
            energy_updated_at: None,
            skills_config: "{}".to_string(),
            proactivity_level: "low".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let conv = Conversation {
            id: "conv-1".to_string(),
            role_id: Some("role-1".to_string()),
            title: "讨论需求".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T12:00:00Z".to_string(),
        };

        let msg1 = Message {
            id: "msg-1".to_string(),
            conversation_id: "conv-1".to_string(),
            role: "user".to_string(),
            content: "你好".to_string(),
            thinking_content: String::new(),
            is_complete: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            routing_metadata: None,
        };
        let msg2 = Message {
            id: "msg-2".to_string(),
            conversation_id: "conv-1".to_string(),
            role: "assistant".to_string(),
            content: "你好！".to_string(),
            thinking_content: String::new(),
            is_complete: true,
            created_at: "2026-01-01T00:01:00Z".to_string(),
            routing_metadata: None,
        };

        let data = ExportData {
            roles: vec![role],
            tasks: vec![],
            memories: vec![],
            suggestions: vec![],
            notifications: vec![],
            mission: None,
            conflicts: vec![],
            briefings: vec![],
            weekly_reviews: vec![],
            llm_configs: vec![],
            app_settings: vec![],
            mcp_servers: vec![],
            skills: vec![],
            skill_bindings: vec![],
            q2_reminders: vec![],
            big_rock_protection_reminders: vec![],
            conversations: vec![conv],
            messages: vec![msg1, msg2],
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            export_version: "1.0".to_string(),
        };

        let md = generate_markdown(&data);
        assert!(md.contains("### 对话记录"));
        assert!(md.contains("讨论需求"));
        assert!(md.contains("消息数: 2"));
        assert!(md.contains("你好"));
        assert!(md.contains("你好！"));
    }

    #[test]
    fn export_data_serialization_roundtrip() {
        let data = ExportData {
            roles: vec![],
            tasks: vec![],
            memories: vec![],
            suggestions: vec![],
            notifications: vec![],
            mission: None,
            conflicts: vec![serde_json::json!({"id": "c1", "status": "detected"})],
            briefings: vec![],
            weekly_reviews: vec![],
            llm_configs: vec![],
            app_settings: vec![("key1".to_string(), Some("value1".to_string()))],
            mcp_servers: vec![],
            skills: vec![],
            skill_bindings: vec![],
            q2_reminders: vec![],
            big_rock_protection_reminders: vec![],
            conversations: vec![],
            messages: vec![],
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            export_version: "1.0".to_string(),
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let deserialized: ExportData = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.export_version, "1.0");
        assert_eq!(deserialized.exported_at, "2026-01-01T00:00:00Z");
        assert_eq!(deserialized.conflicts.len(), 1);
        assert_eq!(deserialized.app_settings.len(), 1);
        assert_eq!(deserialized.app_settings[0].0, "key1");
    }

    #[test]
    fn export_format_serializes_to_lowercase() {
        let json = serde_json::to_string(&ExportFormat::Sqlite).expect("serialize");
        assert_eq!(json, "\"sqlite\"");

        let json = serde_json::to_string(&ExportFormat::Json).expect("serialize");
        assert_eq!(json, "\"json\"");

        let json = serde_json::to_string(&ExportFormat::Markdown).expect("serialize");
        assert_eq!(json, "\"markdown\"");
    }

    #[test]
    fn export_format_deserializes_from_lowercase() {
        let fmt: ExportFormat = serde_json::from_str("\"sqlite\"").expect("deserialize");
        assert_eq!(fmt, ExportFormat::Sqlite);

        let fmt: ExportFormat = serde_json::from_str("\"json\"").expect("deserialize");
        assert_eq!(fmt, ExportFormat::Json);

        let fmt: ExportFormat = serde_json::from_str("\"markdown\"").expect("deserialize");
        assert_eq!(fmt, ExportFormat::Markdown);
    }

    #[test]
    fn export_result_serializes_with_camel_case() {
        let result = ExportResult {
            files: vec!["/tmp/export.json".to_string()],
            sqlite_path: None,
            json_path: Some("/tmp/export.json".to_string()),
            markdown_path: None,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"files\""));
        assert!(json.contains("\"sqlitePath\""));
        assert!(json.contains("\"jsonPath\""));
        assert!(json.contains("\"markdownPath\""));
    }

    #[tokio::test]
    async fn export_sqlite_copies_db_files() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let app_data_dir = dir.path().join("app_data");
        let export_dir = dir.path().join("export");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");
        std::fs::create_dir_all(&export_dir).expect("create export dir");

        let db_content = b"fake sqlite db content";
        std::fs::write(app_data_dir.join("egosync.db"), db_content).expect("write db");
        std::fs::write(
            app_data_dir.join("conversations.db"),
            b"fake conversations content",
        )
        .expect("write conv db");

        let files = export_sqlite(&app_data_dir, &export_dir).expect("export sqlite");
        assert_eq!(files.len(), 2);

        let copied_main = std::fs::read(&files[0]).expect("read copied db");
        assert_eq!(copied_main, db_content);
    }

    #[tokio::test]
    async fn export_sqlite_returns_error_when_no_db_files() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let app_data_dir = dir.path().join("app_data");
        let export_dir = dir.path().join("export");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");
        std::fs::create_dir_all(&export_dir).expect("create export dir");

        let result = export_sqlite(&app_data_dir, &export_dir);
        assert!(result.is_err());
    }
}
