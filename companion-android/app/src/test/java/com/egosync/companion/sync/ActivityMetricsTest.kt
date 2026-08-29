package com.egosync.companion.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * FR-38 活动统计聚合契约单测（13.2 快照全量口径）。
 *
 * WHY：scope 过滤曾是真实缺陷发生点——角色/管家归属判定反了会把别的角色
 * 的任务算进来（数据越权呈现）。本测试锁定：
 * - all=桌面全局指标直读（含真实 memoryCount）；
 * - butler/角色=按归属聚合，记忆数不可得 → null（UI 显示「—」而非冒充 0）；
 * - 时间窗参数 13.2 呈快照全量口径（不筛选），schemaVersion 演进再收口。
 */
class ActivityMetricsTest {

    private fun task(
        id: String,
        ownerType: String = "role",
        roleId: String? = "role-pm",
        completed: Boolean = false,
    ) = SnapshotTask(
        id = id, ownerType = ownerType, roleId = roleId, title = id,
        deadline = null, quadrant = "q1", isBigRock = false, isCompleted = completed,
        protectionStatus = "none", roleName = null, roleColor = null,
    )

    private fun conversation(id: String, roleId: String?) =
        SnapshotConversation(
            id = id, roleId = roleId, title = id, updatedAt = "2026-08-25T08:00:00Z",
            messages = emptyList(),
        )

    private fun snapshot(
        tasks: List<SnapshotTask>,
        conversations: List<SnapshotConversation>,
        metrics: SnapshotMetrics = SnapshotMetrics(
            taskCount = 10, memoryCount = 76, conversationCount = 20,
            pendingTaskCount = 4, generatedAt = "2026-08-25T08:00:00Z",
        ),
    ) = DesktopSnapshot(
        schemaVersion = 1,
        generatedAt = "2026-08-25T08:00:00Z",
        dataCutoffAt = null,
        truncated = false,
        truncatedDomains = emptyList(),
        roles = listOf(SnapshotRole("role-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate")),
        tasks = tasks,
        dashboard = SnapshotDashboard(statuses = emptyList(), metrics = metrics),
        conversations = conversations,
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    @Test
    fun `全部 scope 直读桌面全局指标含真实记忆数`() {
        // all 是唯一能拿到真 memoryCount 的 scope（桌面全局指标）；
        // 若误按角色聚合，全局数会与桌面仪表盘对不上
        val metrics = SnapshotMapper.activityMetrics(snapshot(emptyList(), emptyList()), MetricScope.ALL, ActivityWindow.All)
        assertEquals(10, metrics[MetricType.TASK_TOTAL])
        assertEquals(76, metrics[MetricType.MEMORY_COUNT])
        assertEquals(20, metrics[MetricType.CONVERSATION_COUNT])
        assertEquals(4, metrics[MetricType.PENDING_TASKS])
    }

    @Test
    fun `角色 scope 按归属聚合并区分待办`() {
        // WHY：跨角色泄漏（把管家/他角色任务算给当前角色）= 数据越权呈现
        val snap = snapshot(
            tasks = listOf(
                task("t-pm-1"),                        // pm 待办
                task("t-pm-2", completed = true),      // pm 已完成
                task("t-fa-1", roleId = "role-father"), // 他角色，不得计入
                task("t-bu-1", ownerType = "butler", roleId = null), // 管家，不得计入
            ),
            conversations = listOf(
                conversation("c-pm", "role-pm"),
                conversation("c-fa", "role-father"),
            ),
        )
        val pm = SnapshotMapper.activityMetrics(snap, "role-pm", ActivityWindow.All)
        assertEquals(2, pm[MetricType.TASK_TOTAL])
        assertEquals(1, pm[MetricType.PENDING_TASKS])
        assertEquals(1, pm[MetricType.CONVERSATION_COUNT])
        // 快照无 per-role 记忆数：null → UI「—」，禁止冒充 0
        assertNull(pm[MetricType.MEMORY_COUNT])
    }

    @Test
    fun `管家 scope 聚合管家归属任务与管家会话`() {
        // 桌面 ownerKey 恒为 "butler"、管家会话 roleId=null——两条归属判定都不得反
        val snap = snapshot(
            tasks = listOf(
                task("t-bu-1", ownerType = "butler", roleId = null),
                task("t-pm-1"),
            ),
            conversations = listOf(
                conversation("c-butler", null),
                conversation("c-pm", "role-pm"),
            ),
        )
        val butler = SnapshotMapper.activityMetrics(snap, MetricScope.BUTLER, ActivityWindow.All)
        assertEquals(1, butler[MetricType.TASK_TOTAL])
        assertEquals(1, butler[MetricType.PENDING_TASKS])
        assertEquals(1, butler[MetricType.CONVERSATION_COUNT])
        assertNull(butler[MetricType.MEMORY_COUNT])
    }

    @Test
    fun `未知 scope 返回全空不抛错`() {
        // 下拉理论不产生未知 id，但快照 role 变更瞬态可能命中：静默空集而非崩溃
        val snap = snapshot(listOf(task("t-pm-1")), listOf(conversation("c-pm", "role-pm")))
        val unknown = SnapshotMapper.activityMetrics(snap, "role-not-exist", ActivityWindow.All)
        assertEquals(0, unknown[MetricType.TASK_TOTAL])
        assertEquals(0, unknown[MetricType.CONVERSATION_COUNT])
        assertEquals(0, unknown[MetricType.PENDING_TASKS])
        assertNull(unknown[MetricType.MEMORY_COUNT])
    }

    @Test
    fun `时间窗参数呈现快照全量口径不筛选`() {
        // 13.2 契约：window 仅作 UI 状态呈现，聚合恒为全量（schemaVersion 演进再收口）——
        // 若有人提前实现时间筛选，本断言立即报错，防止半成品口径静默上线
        val snap = snapshot(
            tasks = listOf(task("t-pm-1"), task("t-pm-2")),
            conversations = listOf(conversation("c-pm", "role-pm")),
        )
        val all = SnapshotMapper.activityMetrics(snap, "role-pm", ActivityWindow.All)
        val recent = SnapshotMapper.activityMetrics(snap, "role-pm", ActivityWindow.Recent(2))
        val custom = SnapshotMapper.activityMetrics(
            snap, "role-pm",
            ActivityWindow.Custom(oldestDate = java.time.LocalDate.parse("2026-08-10"),
                newestDate = java.time.LocalDate.parse("2026-08-20")),
        )
        assertEquals(all, recent)
        assertEquals(all, custom)
    }
}
