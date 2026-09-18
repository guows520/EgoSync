//! Story 15.2: 事件名常量源 —— 引擎侧事件发射的唯一事实源。
//!
//! 收编 A 组迁移服务发射的 8 个事件名 + 壳监听侧 `llm:stream`，共 9 名。
//! 常量值与迁移前各站点字面量逐一相同（跨进程契约：桌面行为零变化）。
//!
//! 纯壳侧事件名（`role:*` / `task:created` 等）不在本源范围（Story 15.3/15.5 收编）。

/// `bigrock:protection` — 大石头保护提醒（bigrock_protection 两处发射）
pub const BIGROCK_PROTECTION_EVENT: &str = "bigrock:protection";

/// `bigrock:reminder` — 大石头规划提醒（bigrock_reminder 发射）
pub const BIGROCK_REMINDER_EVENT: &str = "bigrock:reminder";

/// `briefing:generated` — 晨间简报已生成（briefing_generator 发射）
pub const BRIEFING_GENERATED_EVENT: &str = "briefing:generated";

/// `review:generated` — 周复盘已生成（review_generator 发射）
pub const REVIEW_GENERATED_EVENT: &str = "review:generated";

/// `q2:reminder` — Q2 保护提醒（q2_protection_reminder 发射）
pub const Q2_REMINDER_EVENT: &str = "q2:reminder";

/// `notification:new` — 新通知（scheduler 工作循环发射）
pub const NOTIFICATION_NEW_EVENT: &str = "notification:new";

/// `task:classified` — 任务自动分类完成（或降级）时发射，前端据此刷新任务卡片并清除「分类中」标记
pub const TASK_CLASSIFIED_EVENT: &str = "task:classified";

/// `skill-registry-updated` — Skill 注册表已更新（delegate_bridge 创建 Skill 后发射）
pub const SKILL_REGISTRY_UPDATED_EVENT: &str = "skill-registry-updated";

/// `llm:stream` — LLM 流式事件（agent_engine 发射；companion_dispatch/companion_snapshot 监听）
pub const LLM_STREAM_EVENT: &str = "llm:stream";

#[cfg(test)]
mod tests {
    use super::*;

    /// Story 15.2 评审轮补丁：三个无服务文件钉子的常量在此集中值钉
    /// （与其余 6 个服务内钉子同款守卫）。
    /// WHY: 常量值一旦错拼/漂移，监听侧（companion 快照/镜像）与发射侧
    /// 静默断链且全部门禁保持绿色——值钉是唯一能在 CI 内捕获的层。
    #[test]
    fn event_constants_pin_values() {
        assert_eq!(NOTIFICATION_NEW_EVENT, "notification:new");
        assert_eq!(SKILL_REGISTRY_UPDATED_EVENT, "skill-registry-updated");
        assert_eq!(LLM_STREAM_EVENT, "llm:stream");
    }
}
