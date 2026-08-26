package com.egosync.companion.sync

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * FR-38 活动统计聚合契约单测：scope 过滤 + 时间窗与天数桶的相交判定。
 *
 * WHY：桶重叠判定曾是真实缺陷发生点——自定义区间的两端点曾被比较反，
 * 跨桶区间（如「最近两周」）四项指标静默归零。本测试锁定
 * 「桶与窗口相交即整桶计入」语义与各预设前缀和，防止接真实数据层时契约回退。
 */
class ActivityMetricsTest {

    @Test
    fun `全部日期返回四桶总和并与角色卡统计对齐`() {
        val pm = SnapshotStore.activityMetrics("role-pm", ActivityWindow.All)
        assertEquals(14, pm[MetricType.TASK_TOTAL])
        assertEquals(38, pm[MetricType.MEMORY_COUNT])
        assertEquals(3, pm[MetricType.PENDING_TASKS])
        assertEquals(26, pm[MetricType.CONVERSATION_COUNT])
    }

    @Test
    fun `预设窗口取天数桶前缀和`() {
        val recent3 = SnapshotStore.activityMetrics("role-pm", ActivityWindow.Recent(2))
        assertEquals(4, recent3[MetricType.TASK_TOTAL])
        val recent7 = SnapshotStore.activityMetrics("role-pm", ActivityWindow.Recent(6))
        assertEquals(7, recent7[MetricType.TASK_TOTAL]) // 近3天桶+近7天段 = 4+3
        val month = SnapshotStore.activityMetrics("role-pm", ActivityWindow.Recent(29))
        assertEquals(12, month[MetricType.TASK_TOTAL]) // 再加近1月段 = 7+5
    }

    @Test
    fun `自定义跨桶区间按相交桶整桶计入`() {
        // 窗口 daysAgo ∈ [5..20]：与 3–6 桶相交、完整覆盖 7–29 桶；0–2 与 30+ 不相交
        val window = ActivityWindow.Custom(oldestDaysAgo = 20, newestDaysAgo = 5)
        val metrics = SnapshotStore.activityMetrics("role-pm", window)
        assertEquals(3 + 5, metrics[MetricType.TASK_TOTAL])
    }

    @Test
    fun `自定义同桶内区间记整桶`() {
        // 窗口 [23..25] 完全落在 7–29 天桶内 → 整桶计入（任务桶值 5）
        val window = ActivityWindow.Custom(oldestDaysAgo = 25, newestDaysAgo = 23)
        assertEquals(
            5,
            SnapshotStore.activityMetrics("role-pm", window)[MetricType.TASK_TOTAL],
        )
    }

    @Test
    fun `全部 scope 为含管家的所有账本合计`() {
        val all = SnapshotStore.activityMetrics(MetricScope.ALL, ActivityWindow.All)
        val ledgerSum = SnapshotStore.activityLedger.values.sumOf { ledger ->
            ledger.getValue(MetricType.TASK_TOTAL).sum()
        }
        assertEquals(ledgerSum, all[MetricType.TASK_TOTAL])
    }

    @Test
    fun `管家 scope 只聚合管家账本`() {
        val butler = SnapshotStore.activityMetrics(MetricScope.BUTLER, ActivityWindow.All)
        assertEquals(12, butler[MetricType.CONVERSATION_COUNT]) // 5+4+2+1
    }

    @Test
    fun `未知 scope 返回全零且不抛错`() {
        val unknown = SnapshotStore.activityMetrics("role-not-exist", ActivityWindow.All)
        assertEquals(MetricType.entries.size, unknown.size)
        unknown.forEach { (_, value) -> assertEquals(0, value) }
    }
}
