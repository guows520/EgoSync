//! Story 15.2: 角色上下文构建 —— 自桌面壳 agent_engine 提取的共享件。
//!
//! 依赖闭包解锁件：suggestion_generator（泛域）依赖本模块的
//! `build_role_task_summary` / `build_role_memory_summary`；agent_engine
//! 留壳侧经顶部 `use` 回引共享 helper（`format_task_context_line` 等），
//! 其余函数体零改动。内容与提取源逐字节等价。

use crate::db::pool::DbPool;
use crate::db::{memories, tasks};
use crate::error::AppError;

/// 每角色注入管家记忆上下文的条数上限。
pub const BUTLER_MEMORY_PER_ROLE: usize = 6;
const BUTLER_MEMORY_PER_LINE_CHARS: usize = 90;
pub const BUTLER_MEMORY_TOTAL_CHARS: usize = 3000;
pub const TASK_CONTEXT_VISIBLE_LIMIT: usize = 50;
const TASK_CONTEXT_TITLE_CHARS: usize = 80;

pub fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn task_status_label(task: &crate::models::task::Task) -> &'static str {
    if task.is_completed {
        "已完成"
    } else {
        "未完成"
    }
}

pub fn format_task_context_line(task: &crate::models::task::Task) -> String {
    let mut badges = vec![format!("[{}]", task_status_label(task)), format!("[{}]", task.quadrant)];
    if task.is_big_rock {
        badges.push("[大石头]".to_string());
    }
    if task.protection_status != "normal" {
        badges.push(format!("[{}]", task.protection_status));
    }
    let deadline = task
        .deadline
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!(" 截止:{}", value))
        .unwrap_or_default();
    format!(
        "- id={} {}{} {}",
        task.id,
        badges.join(""),
        deadline,
        truncate_chars(task.title.trim(), TASK_CONTEXT_TITLE_CHARS)
    )
}

pub fn select_task_context_items(tasks: &[crate::models::task::Task]) -> (Vec<&crate::models::task::Task>, usize) {
    let mut selected = tasks
        .iter()
        .filter(|task| !task.is_completed)
        .collect::<Vec<_>>();
    let remaining_slots = TASK_CONTEXT_VISIBLE_LIMIT.saturating_sub(selected.len());
    let completed = tasks
        .iter()
        .filter(|task| task.is_completed)
        .collect::<Vec<_>>();
    let omitted_completed_count = completed.len().saturating_sub(remaining_slots);
    selected.extend(completed.into_iter().take(remaining_slots));
    (selected, omitted_completed_count)
}

pub async fn build_role_task_summary(
    main_pool: &DbPool,
    role_id: &str,
) -> Result<String, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;
    let role_tasks = tasks::list_tasks_by_role(main_pool, role_id).await?;
    if role_tasks.is_empty() {
        return Ok(String::new());
    }

    let (selected, omitted_completed_count) = select_task_context_items(&role_tasks);
    let mut lines = selected
        .into_iter()
        .map(format_task_context_line)
        .collect::<Vec<_>>();
    if omitted_completed_count > 0 {
        lines.push(format!("- 另有 {} 条已完成任务未注入。", omitted_completed_count));
    }

    Ok(format!(
        "[当前角色任务]\n{}：\n{}\n规则：这是 EgoSync 内部任务列表；当用户询问任务、待办、安排或下一步时，优先依据本段回答，不要去工作目录寻找任务文件。未完成任务必须全部覆盖；已完成任务只在总量不超过 {} 条时补充。任务 ID 仅用于 complete_task / delete_task 工具参数，不得在自然语言回复中展示。",
        role.name,
        lines.join("\n"),
        TASK_CONTEXT_VISIBLE_LIMIT
    ))
}

pub fn format_memory_reference_label(memory: &crate::models::memory::Memory) -> String {
    chrono::DateTime::parse_from_rfc3339(&memory.created_at)
        .map(|created_at| {
            created_at
                .with_timezone(&chrono::Local)
                .format("%Y/%m/%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| memory.created_at.clone())
}

pub fn format_memory_reference_line(memory: &crate::models::memory::Memory) -> String {
    format!(
        "- [[记忆#{}]](egosync-memory://{}) [{}] {}",
        format_memory_reference_label(memory),
        memory.id,
        memory.category,
        truncate_chars(memory.content.trim(), BUTLER_MEMORY_PER_LINE_CHARS)
    )
}

pub async fn build_role_memory_summary(
    main_pool: &DbPool,
    role_id: &str,
) -> Result<String, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;
    let memories = memories::list_memories(main_pool, Some(role_id), None, None, None).await?;
    if memories.is_empty() {
        return Ok(String::new());
    }

    let lines = memories
        .into_iter()
        .take(BUTLER_MEMORY_PER_ROLE)
        .map(|memory| format_memory_reference_line(&memory))
        .collect::<Vec<_>>();
    let mut summary = format!("[当前角色记忆]\n{}：\n{}", role.name, lines.join("\n"));
    if summary.chars().count() > BUTLER_MEMORY_TOTAL_CHARS {
        summary = truncate_chars(&summary, BUTLER_MEMORY_TOTAL_CHARS) + "…";
    }
    Ok(summary)
}
