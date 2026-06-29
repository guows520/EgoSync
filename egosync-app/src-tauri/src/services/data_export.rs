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
    pub forgotten_memory_sources: Vec<serde_json::Value>,
    pub role_mcp_server_bindings: Vec<serde_json::Value>,
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

async fn query_forgotten_memory_sources(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT id, role_id, category, content, normalized_content, source_conversation_id, source_message_ids, forgotten_at
         FROM forgotten_memory_sources ORDER BY forgotten_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询已遗忘记忆来源失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "roleId": row.get::<Option<String>, _>("role_id"),
                "category": row.get::<String, _>("category"),
                "content": row.get::<String, _>("content"),
                "normalizedContent": row.get::<String, _>("normalized_content"),
                "sourceConversationId": row.get::<String, _>("source_conversation_id"),
                "sourceMessageIds": row.get::<String, _>("source_message_ids"),
                "forgottenAt": row.get::<String, _>("forgotten_at"),
            })
        })
        .collect())
}

async fn query_role_mcp_server_bindings(pool: &DbPool) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = sqlx::query(
        "SELECT server_id, role_id, created_at FROM role_mcp_server_bindings ORDER BY server_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色 MCP 绑定失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "serverId": row.get::<String, _>("server_id"),
                "roleId": row.get::<String, _>("role_id"),
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
    let forgotten_memory_sources = query_forgotten_memory_sources(pool).await?;
    let role_mcp_server_bindings = query_role_mcp_server_bindings(pool).await?;
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
        forgotten_memory_sources,
        role_mcp_server_bindings,
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

/// 主库中需要清空的数据表（硬编码，保留 _sqlx_migrations）
const MAIN_DB_TABLES: &[&str] = &[
    "roles",
    "tasks",
    "memories",
    "forgotten_memory_sources",
    "suggestions",
    "notifications",
    "q2_reminders",
    "big_rock_protection_reminders",
    "mission",
    "conflicts",
    "briefings",
    "weekly_reviews",
    "llm_configs",
    "app_settings",
    "mcp_servers",
    "role_mcp_server_bindings",
    "skills",
    "skill_role_bindings",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub roles_count: usize,
    pub tasks_count: usize,
    pub memories_count: usize,
    pub conversations_count: usize,
    pub messages_count: usize,
}

/// 对话库表列表
const CONV_DB_TABLES: &[&str] = &["conversations", "messages"];

/// 从导出 JSON 的对象字段中提取必填字符串，缺失则返回 ValidationError（避免静默写入空串）
fn json_req_str<'a>(v: &'a serde_json::Value, key: &str, table: &str) -> Result<&'a str, AppError> {
    v.get(key)
        .and_then(|x| x.as_str())
        .ok_or_else(|| AppError::ValidationError(format!("{} 记录缺少必填字段 {}", table, key)))
}

/// 从导出 JSON 的对象字段中提取必填整数，缺失则返回 ValidationError
fn json_req_i64(v: &serde_json::Value, key: &str, table: &str) -> Result<i64, AppError> {
    v.get(key)
        .and_then(|x| x.as_i64())
        .ok_or_else(|| AppError::ValidationError(format!("{} 记录缺少必填字段 {}", table, key)))
}

/// 导入 JSON 数据：反序列化 → 版本校验 → 事务内清空+写入所有表
pub async fn import_json_data(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    file_path: &Path,
) -> Result<ImportResult, AppError> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| AppError::ValidationError(format!("读取导入文件失败: {}", e)))?;
    let data: ExportData = serde_json::from_str(&content)
        .map_err(|e| AppError::ValidationError(format!("JSON 解析失败: {}", e)))?;

    if data.export_version != EXPORT_VERSION {
        return Err(AppError::ValidationError(format!(
            "不兼容的导出版本: {}，当前支持: {}",
            data.export_version, EXPORT_VERSION
        )));
    }

    // 主库事务：清空 → 逐表写入
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启主库事务失败: {}", e)))?;

    for table in MAIN_DB_TABLES {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("清空表 {} 失败: {}", table, e)))?;
    }

    // roles
    for r in &data.roles {
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, personality_prompt, status, energy, energy_updated_at, skills_config, proactivity_level, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(&r.id).bind(&r.name).bind(&r.icon).bind(&r.color).bind(&r.goal)
        .bind(&r.personality_prompt).bind(&r.status).bind(r.energy)
        .bind(&r.energy_updated_at).bind(&r.skills_config).bind(&r.proactivity_level)
        .bind(&r.archived_at).bind(&r.created_at).bind(&r.updated_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 roles 失败: {}", e)))?;
    }

    // tasks
    for t in &data.tasks {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed, completed_at, sort_order, protection_status, confidence, manual_override, classification_reason, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        )
        .bind(&t.id).bind(&t.owner_type).bind(&t.role_id).bind(&t.title)
        .bind(&t.deadline).bind(&t.quadrant).bind(t.is_big_rock).bind(t.is_completed)
        .bind(&t.completed_at).bind(t.sort_order).bind(&t.protection_status)
        .bind(t.confidence).bind(t.manual_override).bind(&t.classification_reason)
        .bind(&t.created_at).bind(&t.updated_at).bind(&t.deleted_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 tasks 失败: {}", e)))?;
    }

    // memories
    for m in &data.memories {
        sqlx::query(
            "INSERT INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&m.id).bind(&m.role_id).bind(&m.category).bind(&m.content)
        .bind(&m.source_conversation_id).bind(&m.source_message_ids).bind(&m.created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 memories 失败: {}", e)))?;
    }

    // suggestions
    for s in &data.suggestions {
        sqlx::query(
            "INSERT INTO suggestions (id, role_id, title, content, priority, status, rejection_reason, converted_task_id, conversation_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&s.id).bind(&s.role_id).bind(&s.title).bind(&s.content)
        .bind(&s.priority).bind(&s.status).bind(&s.rejection_reason)
        .bind(&s.converted_task_id).bind(&s.conversation_id).bind(&s.created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 suggestions 失败: {}", e)))?;
    }

    // notifications
    for n in &data.notifications {
        sqlx::query(
            "INSERT INTO notifications (id, role_id, level, content, is_read, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&n.id).bind(&n.role_id).bind(&n.level).bind(&n.content)
        .bind(n.is_read).bind(&n.created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 notifications 失败: {}", e)))?;
    }

    // mission
    if let Some(mission) = &data.mission {
        sqlx::query(
            "INSERT INTO mission (id, content, format, updated_at) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&mission.id).bind(&mission.content).bind(&mission.format).bind(&mission.updated_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 mission 失败: {}", e)))?;
    }

    // conflicts (serde_json::Value)
    for c in &data.conflicts {
        let id = json_req_str(c, "id", "conflicts")?;
        let task_id_a = json_req_str(c, "taskIdA", "conflicts")?;
        let task_id_b = json_req_str(c, "taskIdB", "conflicts")?;
        let role_id_a = json_req_str(c, "roleIdA", "conflicts")?;
        let role_id_b = json_req_str(c, "roleIdB", "conflicts")?;
        let conflict_time = json_req_str(c, "conflictTime", "conflicts")?;
        let status = json_req_str(c, "status", "conflicts")?;
        let resolution = c["resolution"].as_str();
        let created_at = json_req_str(c, "createdAt", "conflicts")?;
        sqlx::query(
            "INSERT INTO conflicts (id, task_id_a, task_id_b, role_id_a, role_id_b, conflict_time, status, resolution, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(id).bind(task_id_a).bind(task_id_b).bind(role_id_a).bind(role_id_b)
        .bind(conflict_time).bind(status).bind(resolution).bind(created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 conflicts 失败: {}", e)))?;
    }

    // briefings
    for b in &data.briefings {
        sqlx::query(
            "INSERT INTO briefings (id, content, date, created_at) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&b.id).bind(&b.content).bind(&b.date).bind(&b.created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 briefings 失败: {}", e)))?;
    }

    // weekly_reviews
    for wr in &data.weekly_reviews {
        sqlx::query(
            "INSERT INTO weekly_reviews (id, week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&wr.id).bind(&wr.week_start).bind(&wr.week_end).bind(&wr.summary)
        .bind(&wr.energy_trends).bind(&wr.bigrock_status).bind(wr.new_memories_count)
        .bind(&wr.created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 weekly_reviews 失败: {}", e)))?;
    }

    // llm_configs
    for cfg in &data.llm_configs {
        sqlx::query(
            "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&cfg.id).bind(&cfg.name).bind(&cfg.provider).bind(&cfg.base_url)
        .bind(&cfg.model).bind(&cfg.api_key_ref).bind(cfg.is_default)
        .bind(&cfg.created_at).bind(&cfg.updated_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 llm_configs 失败: {}", e)))?;
    }

    // app_settings
    for (key, value) in &data.app_settings {
        sqlx::query("INSERT INTO app_settings (key, value) VALUES (?1, ?2)")
            .bind(key).bind(value)
            .execute(&mut *tx).await
            .map_err(|e| AppError::DbError(format!("插入 app_settings 失败: {}", e)))?;
    }

    // mcp_servers
    for srv in &data.mcp_servers {
        sqlx::query(
            "INSERT INTO mcp_servers (id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&srv.id).bind(&srv.name).bind(&srv.server_type).bind(&srv.command_or_url)
        .bind(&srv.env_refs).bind(&srv.description).bind(srv.enabled)
        .bind(&srv.created_at).bind(&srv.updated_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 mcp_servers 失败: {}", e)))?;
    }

    // role_mcp_server_bindings
    for b in &data.role_mcp_server_bindings {
        let server_id = json_req_str(b, "serverId", "role_mcp_server_bindings")?;
        let role_id = json_req_str(b, "roleId", "role_mcp_server_bindings")?;
        let created_at = json_req_str(b, "createdAt", "role_mcp_server_bindings")?;
        sqlx::query(
            "INSERT INTO role_mcp_server_bindings (server_id, role_id, created_at) VALUES (?1, ?2, ?3)",
        )
        .bind(server_id).bind(role_id).bind(created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 role_mcp_server_bindings 失败: {}", e)))?;
    }

    // skills
    for s in &data.skills {
        let id = json_req_str(s, "id", "skills")?;
        let name = json_req_str(s, "name", "skills")?;
        let description = json_req_str(s, "description", "skills")?;
        let source_type = json_req_str(s, "sourceType", "skills")?;
        let managed_path = json_req_str(s, "managedPath", "skills")?;
        let content_hash = json_req_str(s, "contentHash", "skills")?;
        let created_at = json_req_str(s, "createdAt", "skills")?;
        let updated_at = json_req_str(s, "updatedAt", "skills")?;
        sqlx::query(
            "INSERT INTO skills (id, name, description, source_type, managed_path, content_hash, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(id).bind(name).bind(description).bind(source_type)
        .bind(managed_path).bind(content_hash).bind(created_at).bind(updated_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 skills 失败: {}", e)))?;
    }

    // skill_role_bindings
    for b in &data.skill_bindings {
        let skill_id = json_req_str(b, "skillId", "skill_role_bindings")?;
        let role_id = json_req_str(b, "roleId", "skill_role_bindings")?;
        let created_at = json_req_str(b, "createdAt", "skill_role_bindings")?;
        sqlx::query(
            "INSERT INTO skill_role_bindings (skill_id, role_id, created_at) VALUES (?1, ?2, ?3)",
        )
        .bind(skill_id).bind(role_id).bind(created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 skill_role_bindings 失败: {}", e)))?;
    }

    // q2_reminders
    for r in &data.q2_reminders {
        let id = json_req_str(r, "id", "q2_reminders")?;
        let task_id = json_req_str(r, "taskId", "q2_reminders")?;
        let reminded_count = json_req_i64(r, "remindedCount", "q2_reminders")?;
        let last_reminded_at = json_req_str(r, "lastRemindedAt", "q2_reminders")?;
        let created_at = json_req_str(r, "createdAt", "q2_reminders")?;
        sqlx::query(
            "INSERT INTO q2_reminders (id, task_id, reminded_count, last_reminded_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(id).bind(task_id).bind(reminded_count).bind(last_reminded_at).bind(created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 q2_reminders 失败: {}", e)))?;
    }

    // big_rock_protection_reminders
    for r in &data.big_rock_protection_reminders {
        let id = json_req_str(r, "id", "big_rock_protection_reminders")?;
        let task_id = json_req_str(r, "taskId", "big_rock_protection_reminders")?;
        let reminded_count = json_req_i64(r, "remindedCount", "big_rock_protection_reminders")?;
        let last_reminded_at = json_req_str(r, "lastRemindedAt", "big_rock_protection_reminders")?;
        let created_at = json_req_str(r, "createdAt", "big_rock_protection_reminders")?;
        sqlx::query(
            "INSERT INTO big_rock_protection_reminders (id, task_id, reminded_count, last_reminded_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(id).bind(task_id).bind(reminded_count).bind(last_reminded_at).bind(created_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 big_rock_protection_reminders 失败: {}", e)))?;
    }

    // forgotten_memory_sources
    for f in &data.forgotten_memory_sources {
        let id = json_req_str(f, "id", "forgotten_memory_sources")?;
        let role_id = f["roleId"].as_str();
        let category = json_req_str(f, "category", "forgotten_memory_sources")?;
        let content = json_req_str(f, "content", "forgotten_memory_sources")?;
        let normalized_content = json_req_str(f, "normalizedContent", "forgotten_memory_sources")?;
        let source_conversation_id = json_req_str(f, "sourceConversationId", "forgotten_memory_sources")?;
        let source_message_ids = json_req_str(f, "sourceMessageIds", "forgotten_memory_sources")?;
        let forgotten_at = json_req_str(f, "forgottenAt", "forgotten_memory_sources")?;
        sqlx::query(
            "INSERT INTO forgotten_memory_sources (id, role_id, category, content, normalized_content, source_conversation_id, source_message_ids, forgotten_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(id).bind(role_id).bind(category).bind(content)
        .bind(normalized_content).bind(source_conversation_id)
        .bind(source_message_ids).bind(forgotten_at)
        .execute(&mut *tx).await
        .map_err(|e| AppError::DbError(format!("插入 forgotten_memory_sources 失败: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交主库事务失败: {}", e)))?;

    // 对话库事务
    let mut conv_tx = conv_pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启对话库事务失败: {}", e)))?;

    sqlx::query("DELETE FROM messages")
        .execute(&mut *conv_tx).await
        .map_err(|e| AppError::DbError(format!("清空 messages 失败: {}", e)))?;
    sqlx::query("DELETE FROM conversations")
        .execute(&mut *conv_tx).await
        .map_err(|e| AppError::DbError(format!("清空 conversations 失败: {}", e)))?;

    for c in &data.conversations {
        sqlx::query(
            "INSERT INTO conversations (id, role_id, title, started_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&c.id).bind(&c.role_id).bind(&c.title)
        .bind(&c.started_at).bind(&c.updated_at)
        .execute(&mut *conv_tx).await
        .map_err(|e| AppError::DbError(format!("插入 conversations 失败: {}", e)))?;
    }

    for msg in &data.messages {
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content, thinking_content, is_complete, created_at, routing_metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&msg.id).bind(&msg.conversation_id).bind(&msg.role)
        .bind(&msg.content).bind(&msg.thinking_content).bind(msg.is_complete)
        .bind(&msg.created_at).bind(&msg.routing_metadata)
        .execute(&mut *conv_tx).await
        .map_err(|e| AppError::DbError(format!("插入 messages 失败: {}", e)))?;
    }

    conv_tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交对话库事务失败: {}", e)))?;

    let result = ImportResult {
        roles_count: data.roles.len(),
        tasks_count: data.tasks.len(),
        memories_count: data.memories.len(),
        conversations_count: data.conversations.len(),
        messages_count: data.messages.len(),
    };
    tracing::info!(?result, "JSON 导入完成");
    Ok(result)
}

/// 导入 SQLite 备份：在单一连接上 ATTACH → 校验结构 → 事务内逐表复制 → DETACH
///
/// 连接池可能持有多条连接，而 `ATTACH DATABASE` 仅作用于执行它的那一条连接，
/// 因此必须 `acquire()` 取出单一连接，让 ATTACH/复制/DETACH 全部落在同一连接上；
/// 复制阶段包裹在事务中，任何失败都会回滚，保证导入失败时当前数据不变。
pub async fn import_sqlite_data(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    file_path: &Path,
) -> Result<ImportResult, AppError> {
    // ---- 自动识别主库/对话库文件 ----
    // 导出时产生两个文件：egosync-export-{date}.db 和 egosync-export-{date}-conversations.db
    // 用户可能选择其中任意一个，这里根据文件名自动推导另一个的路径。
    let (main_file, conv_file) = {
        let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if file_name.ends_with("-conversations.db") {
            // 用户选择的是对话库文件，主库路径去掉 "-conversations" 后缀
            let stem = file_name.strip_suffix("-conversations.db").unwrap_or(file_name);
            (parent.join(format!("{}.db", stem)), file_path.to_path_buf())
        } else {
            // 用户选择的是主库文件，对话库路径加 "-conversations" 后缀
            let stem = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            (file_path.to_path_buf(), parent.join(format!("{}-conversations.db", stem)))
        }
    };

    // ---- 主库：单一连接上 ATTACH → 校验 → 事务复制 → DETACH ----
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| AppError::DbError(format!("获取主库连接失败: {}", e)))?;

    sqlx::query("ATTACH DATABASE ?1 AS imported")
        .bind(main_file.to_string_lossy().to_string())
        .execute(&mut *conn)
        .await
        .map_err(|e| AppError::DbError(format!("ATTACH 主库失败: {}", e)))?;

    let copy_result: Result<(i64, i64, i64), AppError> = async {
        // 结构校验：导入库必须包含所有目标表，否则视为不兼容存档
        for table in MAIN_DB_TABLES {
            let exists: Option<String> = sqlx::query_scalar(
                "SELECT name FROM imported.sqlite_master WHERE type = 'table' AND name = ?1",
            )
            .bind(table)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| AppError::DbError(format!("校验导入表 {} 失败: {}", table, e)))?;
            if exists.is_none() {
                return Err(AppError::ValidationError(format!(
                    "存档文件结构不兼容：缺少数据表 {}",
                    table
                )));
            }
        }

        sqlx::query("BEGIN")
            .execute(&mut *conn)
            .await
            .map_err(|e| AppError::DbError(format!("开启主库事务失败: {}", e)))?;
        for table in MAIN_DB_TABLES {
            sqlx::query(&format!("DELETE FROM {}", table))
                .execute(&mut *conn)
                .await
                .map_err(|e| AppError::DbError(format!("清空表 {} 失败: {}", table, e)))?;
            sqlx::query(&format!("INSERT INTO {} SELECT * FROM imported.{}", table, table))
                .execute(&mut *conn)
                .await
                .map_err(|e| AppError::DbError(format!("复制表 {} 失败: {}", table, e)))?;
        }
        sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(|e| AppError::DbError(format!("提交主库事务失败: {}", e)))?;

        let roles_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&mut *conn).await.unwrap_or(0);
        let tasks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&mut *conn).await.unwrap_or(0);
        let memories_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&mut *conn).await.unwrap_or(0);
        Ok((roles_count, tasks_count, memories_count))
    }
    .await;

    // 无论成功失败，都必须回滚未提交事务并 DETACH，否则连接归还连接池时仍挂载 imported
    if copy_result.is_err() {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
    }
    let _ = sqlx::query("DETACH DATABASE imported").execute(&mut *conn).await;
    drop(conn);

    let (roles_count, tasks_count, memories_count) = copy_result?;

    let (conversations_count, messages_count) = if conv_file.exists() {
        let mut conv_conn = conv_pool
            .acquire()
            .await
            .map_err(|e| AppError::DbError(format!("获取对话库连接失败: {}", e)))?;

        sqlx::query("ATTACH DATABASE ?1 AS imported_conv")
            .bind(conv_file.to_string_lossy().to_string())
            .execute(&mut *conv_conn)
            .await
            .map_err(|e| AppError::DbError(format!("ATTACH 对话库失败: {}", e)))?;

        let conv_copy: Result<(i64, i64), AppError> = async {
            for table in CONV_DB_TABLES {
                let exists: Option<String> = sqlx::query_scalar(
                    "SELECT name FROM imported_conv.sqlite_master WHERE type = 'table' AND name = ?1",
                )
                .bind(table)
                .fetch_optional(&mut *conv_conn)
                .await
                .map_err(|e| AppError::DbError(format!("校验导入对话表 {} 失败: {}", table, e)))?;
                if exists.is_none() {
                    return Err(AppError::ValidationError(format!(
                        "对话存档文件结构不兼容：缺少数据表 {}",
                        table
                    )));
                }
            }

            sqlx::query("BEGIN")
                .execute(&mut *conv_conn)
                .await
                .map_err(|e| AppError::DbError(format!("开启对话库事务失败: {}", e)))?;
            for table in CONV_DB_TABLES {
                sqlx::query(&format!("DELETE FROM {}", table))
                    .execute(&mut *conv_conn)
                    .await
                    .map_err(|e| AppError::DbError(format!("清空对话表 {} 失败: {}", table, e)))?;
                sqlx::query(&format!("INSERT INTO {} SELECT * FROM imported_conv.{}", table, table))
                    .execute(&mut *conv_conn)
                    .await
                    .map_err(|e| AppError::DbError(format!("复制对话表 {} 失败: {}", table, e)))?;
            }
            sqlx::query("COMMIT")
                .execute(&mut *conv_conn)
                .await
                .map_err(|e| AppError::DbError(format!("提交对话库事务失败: {}", e)))?;

            let conv_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
                .fetch_one(&mut *conv_conn).await.unwrap_or(0);
            let msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
                .fetch_one(&mut *conv_conn).await.unwrap_or(0);
            Ok((conv_count, msg_count))
        }
        .await;

        if conv_copy.is_err() {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conv_conn).await;
        }
        let _ = sqlx::query("DETACH DATABASE imported_conv").execute(&mut *conv_conn).await;
        drop(conv_conn);

        conv_copy?
    } else {
        tracing::info!("对话库文件不存在，仅导入主库数据");
        (0i64, 0i64)
    };

    let result = ImportResult {
        roles_count: roles_count as usize,
        tasks_count: tasks_count as usize,
        memories_count: memories_count as usize,
        conversations_count: conversations_count as usize,
        messages_count: messages_count as usize,
    };
    tracing::info!(?result, "SQLite 导入完成");
    Ok(result)
}

/// 统一导入入口：备份 → 清理过期备份 → 识别格式 → 分发
pub async fn import_all(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    file_path: &Path,
) -> Result<ImportResult, AppError> {
    // 导入前备份当前数据
    if let Err(e) = create_backup(pool, conv_pool).await {
        return Err(AppError::ValidationError(format!(
            "导入前备份失败，已中止导入: {}", e
        )));
    }

    // 清理过期备份
    if let Err(e) = cleanup_old_backups() {
        tracing::warn!("清理过期备份文件失败: {}", e);
    }

    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "db" => import_sqlite_data(pool, conv_pool, file_path).await,
        "json" => import_json_data(pool, conv_pool, file_path).await,
        _ => Err(AppError::ValidationError("不支持的文件格式".to_string())),
    }
}

/// 销毁所有数据 — 事务内清空所有数据表，保留 schema 和 _sqlx_migrations
pub async fn destroy_all_data(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
    _app_data_dir: &Path,
) -> Result<(), AppError> {
    // 1. 创建隐藏备份（失败则中止销毁）
    if let Err(e) = create_backup(pool, conv_pool).await {
        return Err(AppError::ValidationError(format!(
            "销毁前备份失败，已中止销毁: {}",
            e
        )));
    }

    // 2. 清理过期备份（7天前），失败只记 warn 不阻塞
    if let Err(e) = cleanup_old_backups() {
        tracing::warn!("清理过期备份文件失败: {}", e);
    }

    // 3. 读取 LLM 配置的 api_key_ref（必须在 DELETE FROM llm_configs 之前读取）。
    //    keyring 删除推迟到 DB 事务全部提交成功之后，避免销毁失败却已误删 API Key。
    let api_key_refs: Vec<String> = settings::list_llm_configs(pool)
        .await?
        .into_iter()
        .map(|config| config.api_key_ref)
        .collect();

    // 4. 主库事务：DELETE FROM 所有数据表
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启主库事务失败: {}", e)))?;
    for table in MAIN_DB_TABLES {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("清空表 {} 失败: {}", table, e)))?;
    }
    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交主库事务失败: {}", e)))?;

    // 5. 对话库事务：DELETE FROM messages, conversations
    let mut conv_tx = conv_pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启对话库事务失败: {}", e)))?;
    sqlx::query("DELETE FROM messages")
        .execute(&mut *conv_tx)
        .await
        .map_err(|e| AppError::DbError(format!("清空 messages 表失败: {}", e)))?;
    sqlx::query("DELETE FROM conversations")
        .execute(&mut *conv_tx)
        .await
        .map_err(|e| AppError::DbError(format!("清空 conversations 表失败: {}", e)))?;
    conv_tx
        .commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交对话库事务失败: {}", e)))?;

    // 6. 删除 Keyring 密钥（DB 事务全部提交成功后才执行，失败只记 warn 不阻塞）
    for key_ref in &api_key_refs {
        if let Err(e) = crate::services::secret_store::delete_secret(key_ref) {
            tracing::warn!(key = %key_ref, "删除 keyring 密钥失败: {}", e);
        }
    }

    // 7. WAL checkpoint — 截断 WAL 文件，确保已删除数据不可从 WAL 恢复
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

    tracing::info!("所有数据已销毁，数据库已回到初始状态");
    Ok(())
}

/// 创建隐藏备份到临时目录
async fn create_backup(pool: &DbPool, conv_pool: &ConversationsPool) -> Result<(), AppError> {
    let data = gather_export_data(pool, conv_pool).await?;
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| AppError::ValidationError(format!("备份 JSON 序列化失败: {}", e)))?;
    let backup_path = std::env::temp_dir().join(format!(
        "egosync-backup-{}.json",
        chrono::Local::now().format("%Y%m%dT%H%M%S")
    ));
    std::fs::write(&backup_path, json).map_err(|e| io_error("写入备份文件失败", e))?;
    tracing::info!(path = %backup_path.display(), "销毁前备份已创建");
    Ok(())
}

/// 清理过期备份文件（超过7天）
pub fn cleanup_old_backups() -> Result<(), AppError> {
    let temp_dir = std::env::temp_dir();
    let entries =
        std::fs::read_dir(&temp_dir).map_err(|e| io_error("读取临时目录失败", e))?;

    let now = std::time::SystemTime::now();
    let seven_days = std::time::Duration::from_secs(7 * 24 * 60 * 60);

    for entry in entries {
        let entry = entry.map_err(|e| io_error("读取目录条目失败", e))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("egosync-backup-") && name_str.ends_with(".json") {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if now
                        .duration_since(modified)
                        .map(|d| d > seven_days)
                        .unwrap_or(false)
                    {
                        if let Err(e) = std::fs::remove_file(entry.path()) {
                            tracing::warn!(
                                path = %entry.path().display(),
                                "删除过期备份文件失败: {}",
                                e
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
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
            forgotten_memory_sources: vec![],
            role_mcp_server_bindings: vec![],
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
            forgotten_memory_sources: vec![],
            role_mcp_server_bindings: vec![],
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
            forgotten_memory_sources: vec![],
            role_mcp_server_bindings: vec![],
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
            forgotten_memory_sources: vec![],
            role_mcp_server_bindings: vec![],
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

    async fn setup_destroy_test_db() -> (tempfile::TempDir, crate::db::pool::DbPool, crate::db::pool::ConversationsPool) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let main_db_path = dir.path().join("egosync.db");
        let conv_db_path = dir.path().join("conversations.db");

        let pool = crate::db::pool::init_db(&main_db_path)
            .await
            .expect("init main db");
        let conv_pool = crate::db::pool::init_conversations_db(&conv_db_path)
            .await
            .expect("init conversations db");

        // 插入测试数据到多张表
        sqlx::query("INSERT INTO roles (id, name, icon, color, goal, personality_prompt, status, energy, skills_config, proactivity_level, created_at, updated_at) VALUES ('r1', '测试角色', '🎯', '#FF0000', '测试', '', 'active', 80, '{}', 'moderate', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert role");

        sqlx::query("INSERT INTO app_settings (key, value) VALUES ('onboarding_completed', 'true')")
            .execute(&pool)
            .await
            .expect("insert app_setting");

        sqlx::query("INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at) VALUES ('cfg1', '测试配置', 'openai_compatible', 'https://api.openai.com/v1', 'gpt-4o', 'test_key_ref_1', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert llm_config");

        sqlx::query("INSERT INTO conversations (id, role_id, title, started_at, updated_at) VALUES ('c1', NULL, '测试对话', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&*conv_pool)
            .await
            .expect("insert conversation");

        sqlx::query("INSERT INTO messages (id, conversation_id, role, content, thinking_content, is_complete, created_at) VALUES ('m1', 'c1', 'user', '你好', '', 1, '2026-01-01T00:00:00Z')")
            .execute(&*conv_pool)
            .await
            .expect("insert message");

        // 插入引用 r1 的子表行，验证父表先删时的 ON DELETE CASCADE 级联清空路径。
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, sort_order, created_at, updated_at) VALUES ('t1', 'role', 'r1', '测试任务', 'Q1', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert task");

        sqlx::query("INSERT INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids, created_at) VALUES ('mem1', 'r1', 'fact', '测试记忆', 'c1', '[\"m1\"]', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert memory");

        sqlx::query("INSERT INTO suggestions (id, role_id, title, content, priority, status, created_at) VALUES ('s1', 'r1', '测试建议', '内容', 'medium', 'pending', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert suggestion");

        sqlx::query("INSERT INTO notifications (id, role_id, level, content, is_read, created_at) VALUES ('n1', 'r1', 'whisper', '测试通知', 0, '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert notification");

        sqlx::query("INSERT INTO q2_reminders (id, task_id, reminded_count, last_reminded_at, created_at) VALUES ('q2_1', 't1', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert q2_reminder");

        sqlx::query("INSERT INTO big_rock_protection_reminders (id, task_id, reminded_count, last_reminded_at, created_at) VALUES ('br1', 't1', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert big_rock_protection_reminder");

        sqlx::query("INSERT INTO forgotten_memory_sources (id, role_id, category, content, normalized_content, source_conversation_id, source_message_ids, forgotten_at) VALUES ('f1', 'r1', 'fact', '已遗忘', '已遗忘', 'c1', '[\"m1\"]', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .expect("insert forgotten_memory_source");

        (dir, pool, conv_pool)
    }

    #[tokio::test]
    async fn destroy_all_data_clears_all_tables() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");

        destroy_all_data(&pool, &conv_pool, &app_data_dir)
            .await
            .expect("destroy should succeed");

        // 验证主库表为空
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool)
            .await
            .expect("count roles");
        assert_eq!(role_count, 0);

        let setting_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM app_settings")
            .fetch_one(&pool)
            .await
            .expect("count app_settings");
        assert_eq!(setting_count, 0);

        let llm_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM llm_configs")
            .fetch_one(&pool)
            .await
            .expect("count llm_configs");
        assert_eq!(llm_count, 0);

        // 验证引用 role/task 的子表也被清空（父表先删 + ON DELETE CASCADE 路径）
        for table in [
            "tasks",
            "memories",
            "suggestions",
            "notifications",
            "q2_reminders",
            "big_rock_protection_reminders",
            "forgotten_memory_sources",
        ] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("count {}: {}", table, e));
            assert_eq!(count, 0, "表 {} 应被清空", table);
        }

        // 验证对话库表为空
        let conv_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&*conv_pool)
            .await
            .expect("count conversations");
        assert_eq!(conv_count, 0);

        let msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&*conv_pool)
            .await
            .expect("count messages");
        assert_eq!(msg_count, 0);
    }

    #[tokio::test]
    async fn destroy_all_data_preserves_migrations_table() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");

        // 记录销毁前的 migration 数量
        let migration_count_before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .expect("count migrations before");

        destroy_all_data(&pool, &conv_pool, &app_data_dir)
            .await
            .expect("destroy should succeed");

        // 验证 _sqlx_migrations 数据未被删除
        let migration_count_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .expect("count migrations after");
        assert_eq!(migration_count_before, migration_count_after);
        assert!(migration_count_after > 0);
    }

    #[tokio::test]
    async fn destroy_all_data_creates_backup_json() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");

        destroy_all_data(&pool, &conv_pool, &app_data_dir)
            .await
            .expect("destroy should succeed");

        // 验证 temp_dir 中生成了备份文件
        let temp_dir = std::env::temp_dir();
        let entries = std::fs::read_dir(&temp_dir).expect("read temp dir");
        let mut found_backup = false;
        for entry in entries {
            let entry = entry.expect("read entry");
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("egosync-backup-") && name_str.ends_with(".json") {
                // 验证文件内容是有效 JSON
                let content = std::fs::read_to_string(entry.path()).expect("read backup file");
                assert!(serde_json::from_str::<serde_json::Value>(&content).is_ok());
                found_backup = true;
                // 清理测试产生的备份文件
                let _ = std::fs::remove_file(entry.path());
                break;
            }
        }
        assert!(found_backup, "应在临时目录中生成 egosync-backup-*.json 备份文件");
    }

    #[tokio::test]
    async fn destroy_all_data_preserves_db_schema() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");

        destroy_all_data(&pool, &conv_pool, &app_data_dir)
            .await
            .expect("destroy should succeed");

        // 验证表结构仍然存在（可以查询，只是数据为空）
        let table_exists: Option<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'roles'")
                .fetch_optional(&pool)
                .await
                .expect("query roles table");
        assert_eq!(table_exists.as_deref(), Some("roles"));
    }

    #[test]
    fn cleanup_old_backups_removes_expired_files() {
        let temp_dir = std::env::temp_dir();
        let backup_path = temp_dir.join("egosync-backup-test-expired.json");
        std::fs::write(&backup_path, "{}").expect("write test backup file");

        // 将文件修改时间设置为 8 天前
        let eight_days_ago = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 60 * 60),
        );
        filetime::set_file_mtime(&backup_path, eight_days_ago).expect("set file mtime");

        assert!(backup_path.exists(), "备份文件应存在");

        cleanup_old_backups().expect("cleanup should succeed");

        assert!(!backup_path.exists(), "8天前的备份文件应被删除");
    }

    #[test]
    fn cleanup_old_backups_preserves_recent_files() {
        let temp_dir = std::env::temp_dir();
        let backup_path = temp_dir.join("egosync-backup-test-recent.json");
        std::fs::write(&backup_path, "{}").expect("write test backup file");

        // 文件修改时间为当前时间（刚创建），不应被删除
        assert!(backup_path.exists(), "备份文件应存在");

        cleanup_old_backups().expect("cleanup should succeed");

        assert!(backup_path.exists(), "刚创建的备份文件不应被删除");

        // 清理测试文件
        let _ = std::fs::remove_file(&backup_path);
    }

    #[tokio::test]
    async fn import_json_data_restores_all_tables() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        // 导出当前数据为 JSON
        let export_data = gather_export_data(&pool, &conv_pool).await.expect("gather export data");
        let json = serde_json::to_string_pretty(&export_data).expect("serialize to json");
        let json_path = dir.path().join("export.json");
        std::fs::write(&json_path, json).expect("write json file");

        // 销毁所有数据
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");
        destroy_all_data(&pool, &conv_pool, &app_data_dir).await.expect("destroy data");

        // 验证数据已清空
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count");
        assert_eq!(role_count, 0);

        // 导入 JSON
        let result = import_json_data(&pool, &conv_pool, &json_path).await.expect("import json");

        // 验证数据恢复
        assert_eq!(result.roles_count, 1);
        assert_eq!(result.tasks_count, 1);
        assert_eq!(result.memories_count, 1);
        assert_eq!(result.conversations_count, 1);
        assert_eq!(result.messages_count, 1);

        let restored_role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count roles after import");
        assert_eq!(restored_role_count, 1);

        let restored_task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool).await.expect("count tasks after import");
        assert_eq!(restored_task_count, 1);

        let restored_conv_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&*conv_pool).await.expect("count conversations after import");
        assert_eq!(restored_conv_count, 1);

        let restored_msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&*conv_pool).await.expect("count messages after import");
        assert_eq!(restored_msg_count, 1);
    }

    #[tokio::test]
    async fn import_json_data_rejects_incompatible_version() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        let bad_data = ExportData {
            roles: vec![], tasks: vec![], memories: vec![], suggestions: vec![],
            notifications: vec![], mission: None, conflicts: vec![], briefings: vec![],
            weekly_reviews: vec![], llm_configs: vec![], app_settings: vec![],
            mcp_servers: vec![], skills: vec![], skill_bindings: vec![],
            q2_reminders: vec![], big_rock_protection_reminders: vec![],
            forgotten_memory_sources: vec![], role_mcp_server_bindings: vec![],
            conversations: vec![], messages: vec![],
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            export_version: "99.0".to_string(),
        };
        let json = serde_json::to_string(&bad_data).expect("serialize");
        let json_path = dir.path().join("bad_export.json");
        std::fs::write(&json_path, json).expect("write json file");

        let result = import_json_data(&pool, &conv_pool, &json_path).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            AppError::ValidationError(msg) => assert!(msg.contains("99.0")),
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn import_json_data_rejects_corrupt_file() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        // 损坏 JSON 在反序列化阶段即失败（事务尚未开启）
        let json_path = dir.path().join("corrupt.json");
        std::fs::write(&json_path, "not valid json").expect("write corrupt file");

        let result = import_json_data(&pool, &conv_pool, &json_path).await;
        assert!(result.is_err());

        // 验证原数据不受影响
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count roles");
        assert_eq!(role_count, 1, "原数据应不受解析失败影响");
    }

    #[tokio::test]
    async fn import_json_data_rolls_back_on_failure() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        // 构造合法但含重复主键的导出数据：清空后第二次插入同一 role 触发约束冲突，
        // 从而在事务中途（DELETE 已执行）失败，验证事务回滚保留原数据。
        let mut export_data = gather_export_data(&pool, &conv_pool).await.expect("gather export data");
        let dup_role = export_data.roles[0].clone();
        export_data.roles.push(dup_role);
        let json = serde_json::to_string(&export_data).expect("serialize");
        let json_path = dir.path().join("dup.json");
        std::fs::write(&json_path, json).expect("write json file");

        let result = import_json_data(&pool, &conv_pool, &json_path).await;
        assert!(result.is_err(), "重复主键应导致导入失败");

        // 验证事务回滚：DELETE 已执行但应整体回滚，原数据完整保留
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count roles");
        assert_eq!(role_count, 1, "事务回滚后原角色数据应完整保留");

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool).await.expect("count tasks");
        assert_eq!(task_count, 1, "事务回滚后任务数据应完整保留");
    }

    #[tokio::test]
    async fn import_sqlite_data_restores_all_tables() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        // 用 VACUUM INTO 生成主库与对话库 .db 快照
        // （命名遵循导出约定：{stem}.db + {stem}-conversations.db）
        let backup_main = dir.path().join("backup.db");
        let backup_conv = dir.path().join("backup-conversations.db");
        sqlx::query(&format!("VACUUM INTO '{}'", backup_main.to_string_lossy()))
            .execute(&pool).await.expect("vacuum main db");
        sqlx::query(&format!("VACUUM INTO '{}'", backup_conv.to_string_lossy()))
            .execute(&*conv_pool).await.expect("vacuum conv db");

        // 销毁现有数据
        let app_data_dir = dir.path().join("app_data");
        std::fs::create_dir_all(&app_data_dir).expect("create app_data dir");
        destroy_all_data(&pool, &conv_pool, &app_data_dir).await.expect("destroy data");
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count");
        assert_eq!(role_count, 0);

        // 从 .db 快照导入
        let result = import_sqlite_data(&pool, &conv_pool, &backup_main)
            .await.expect("import sqlite");

        assert_eq!(result.roles_count, 1);
        assert_eq!(result.tasks_count, 1);
        assert_eq!(result.memories_count, 1);
        assert_eq!(result.conversations_count, 1);
        assert_eq!(result.messages_count, 1);

        let restored_roles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count roles after import");
        assert_eq!(restored_roles, 1);

        let restored_tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool).await.expect("count tasks after import");
        assert_eq!(restored_tasks, 1);

        let restored_msgs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&*conv_pool).await.expect("count messages after import");
        assert_eq!(restored_msgs, 1);
    }

    #[tokio::test]
    async fn import_sqlite_data_rejects_incompatible_schema() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        // 构造一个缺少目标表的 .db（仅含无关表），应被结构校验拒绝
        let bad_db = dir.path().join("incompatible.db");
        let bad_url = format!("sqlite:{}?mode=rwc", bad_db.display());
        let bad_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&bad_url)
            .await
            .expect("create incompatible db");
        sqlx::query("CREATE TABLE unrelated (id TEXT PRIMARY KEY)")
            .execute(&bad_pool).await.expect("create unrelated table");
        bad_pool.close().await;

        let result = import_sqlite_data(&pool, &conv_pool, &bad_db).await;
        assert!(result.is_err(), "结构不兼容的 .db 应被拒绝");
        match result.unwrap_err() {
            AppError::ValidationError(msg) => assert!(msg.contains("结构不兼容")),
            other => panic!("expected ValidationError, got {:?}", other),
        }

        // 校验在事务外完成，原数据不受影响
        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(&pool).await.expect("count roles");
        assert_eq!(role_count, 1, "结构校验失败时原数据应保留");
    }

    #[tokio::test]
    async fn import_all_rejects_unsupported_format() {
        let (dir, pool, conv_pool) = setup_destroy_test_db().await;

        let txt_path = dir.path().join("archive.txt");
        std::fs::write(&txt_path, "some content").expect("write txt file");

        let result = import_all(&pool, &conv_pool, &txt_path).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ValidationError(msg) => assert!(msg.contains("不支持的文件格式")),
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[test]
    fn import_result_serializes_with_camel_case() {
        let result = ImportResult {
            roles_count: 1,
            tasks_count: 2,
            memories_count: 3,
            conversations_count: 4,
            messages_count: 5,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("\"rolesCount\""));
        assert!(json.contains("\"tasksCount\""));
        assert!(json.contains("\"memoriesCount\""));
        assert!(json.contains("\"conversationsCount\""));
        assert!(json.contains("\"messagesCount\""));
    }
}
