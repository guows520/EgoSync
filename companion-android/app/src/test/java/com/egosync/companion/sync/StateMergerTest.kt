package com.egosync.companion.sync

import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * StateMerger 全量替换语义（镜像桌面裁决 2：SNAPSHOT 与 STATE_DELTA 载荷同为
 * 全量快照，StateMerger 实现为全量替换，严禁字段级 diff）。
 *
 * WHY：域级 diff 是明确 deferred 项；若误做字段合并，新旧域交错会让手机
 * 渲染出「半新半旧」的混合态（如新任务挂在旧角色名下）。全量替换是唯一
 * 与桌面「整序列持锁入队」语义自洽的形态。
 */
class StateMergerTest {

    private fun snapshotWith(roles: List<SnapshotRole>): DesktopSnapshot = DesktopSnapshot(
        schemaVersion = 1,
        generatedAt = "2026-08-29T12:00:00Z",
        dataCutoffAt = null,
        truncated = false,
        truncatedDomains = emptyList(),
        roles = roles,
        tasks = emptyList(),
        dashboard = SnapshotDashboard(
            statuses = emptyList(),
            metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-29T12:00:00Z"),
        ),
        conversations = emptyList(),
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    private fun role(id: String, energy: Int = 50) = SnapshotRole(
        id = id, name = id, icon = "target", color = "#6366F1", goal = "",
        status = "active", energy = energy, proactivityLevel = "moderate",
    )

    @Test
    fun `applySnapshot 全量替换旧快照不合并`() = runTest {
        // 镜像桌面 write_signal_pushes_state_delta：STATE_DELTA 全量替换
        val merger = StateMerger()
        merger.applySnapshot(snapshotWith(listOf(role("role-a", 10))))

        val after = merger.applySnapshot(snapshotWith(listOf(role("role-b", 90))))

        // 旧 role-a 不得残留——全量替换，非合并
        assertEquals(listOf("role-b"), after.snapshot!!.roles.map { it.id })
        assertEquals(90, after.snapshot.roles.single().energy)
    }

    @Test
    fun `重连全量替换拿到最新快照`() = runTest {
        // 镜像桌面 reconnect_receives_latest_snapshot_after_gap：断线重连后
        // 旧断线期间的增量全部被最新全量快照覆盖
        val merger = StateMerger()
        merger.applySnapshot(snapshotWith(listOf(role("role-a", 10))))
        // 模拟断线期间多次推送未到达；重连后只收到最新全量
        merger.applySnapshot(snapshotWith(listOf(role("role-a", 55), role("role-b", 70))))

        val state = merger.state.first()
        assertEquals(setOf("role-a", "role-b"), state.snapshot!!.roles.map { it.id }.toSet())
        assertEquals(55, state.snapshot.roles.single { it.id == "role-a" }.energy)
    }

    @Test
    fun `STATE_DELTA 先于 SNAPSHOT 到达任意先到皆正确`() = runTest {
        // 评审 deferred 收口：建连全量(SNAPSHOT)与写信号全量(STATE_DELTA)同载荷，
        // 任意先到皆正确（StateMerger 不区分帧来源，只做全量替换）
        val deltaPayload = snapshotWith(listOf(role("role-x", 33)))
        val snapshotPayload = snapshotWith(listOf(role("role-x", 33)))
        val merger = StateMerger()

        // delta 先到
        merger.applySnapshot(deltaPayload)
        assertEquals("role-x", merger.state.first().snapshot!!.roles.single().id)

        // 反向：snapshot 先到，delta 后到——后到者覆盖
        val merger2 = StateMerger()
        merger2.applySnapshot(snapshotWith(listOf(role("role-y", 1))))
        merger2.applySnapshot(deltaPayload)
        assertEquals("role-x", merger2.state.first().snapshot!!.roles.single().id)
    }

    @Test
    fun `初始状态为空未加载`() = runTest {
        val merger = StateMerger()
        val state = merger.state.first()

        assertFalse(state.loaded)
        assertNull(state.snapshot)
    }

    @Test
    fun `metadata 元数据随状态暴露`() = runTest {
        val merger = StateMerger()
        val truncated = snapshotWith(emptyList()).copy(
            generatedAt = "2026-08-29T13:00:00Z",
            dataCutoffAt = "2026-08-01",
            truncated = true,
            truncatedDomains = listOf("conversations", "briefings"),
        )
        merger.applySnapshot(truncated)

        val meta = merger.state.first().metadata!!
        assertEquals("2026-08-29T13:00:00Z", meta.generatedAt)
        assertEquals("2026-08-01", meta.dataCutoffAt)
        assertEquals(true, meta.truncated)
        assertEquals(listOf("conversations", "briefings"), meta.truncatedDomains)
    }
}
