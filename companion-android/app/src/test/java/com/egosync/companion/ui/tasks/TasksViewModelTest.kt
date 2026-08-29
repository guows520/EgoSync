package com.egosync.companion.ui.tasks

import com.egosync.companion.sync.CreateTaskInput
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.DashboardStatus
import com.egosync.companion.sync.SnapshotDashboard
import com.egosync.companion.sync.SnapshotMetrics
import com.egosync.companion.sync.SnapshotRole
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.SnapshotTask
import com.egosync.companion.sync.TaskOwner
import com.egosync.companion.ui.theme.Quadrant
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 任务屏快照换装语义验证（规格 AC2 + 评审 P1/P2）：
 * 快照同一性守卫下，state 重发不得回滚已分类的 seedTask 或新建任务 id 序列；
 * unpair/密钥失效自愈（store.clear）后任务列表一并清空。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class TasksViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun fixtureSnapshot(): DesktopSnapshot = DesktopSnapshot(
        schemaVersion = 1,
        generatedAt = "2026-08-25T08:00:00Z",
        dataCutoffAt = null,
        truncated = false,
        truncatedDomains = emptyList(),
        roles = listOf(
            SnapshotRole("role-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate"),
        ),
        conversations = emptyList(),
        tasks = listOf(
            SnapshotTask("t-1", "butler", null, "写周报", null, "Q1", false, false, "none", "管家", null),
            SnapshotTask(
                "t-2", "role", "role-pm", "整理竞品要点", null, "Q2", true, false,
                "none", "产品经理", "#4F46E5",
            ),
        ),
        dashboard = SnapshotDashboard(
            statuses = listOf(
                DashboardStatus("role-pm", "产品经理", "target", "#4F46E5", 82, 3,
                    "2026-08-25T07:50:00Z", false),
            ),
            metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-25T08:00:00Z"),
        ),
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    private fun TasksUiState.seedCard() = tasks.firstOrNull { it.id == "t-9" }

    @Test
    fun `快照重发不回滚已分类的seedTask`() {
        // WHY：collect 无快照同一性守卫时，任何 state 重发都会执行 store.tasks + seedTask
        // 把已归类 Q2 的演示种子重置为 Q3 占位，且 4 秒分类协程已耗尽不再补分类——
        // 卡片永久错置在第三象限（评审 P1）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store)
        dispatcher.scheduler.advanceUntilIdle()

        // 4 秒 mock 分类完成：seedTask 已归类 Q2
        assertEquals(Quadrant.Q2, vm.uiState.value.seedCard()?.quadrant)

        // 同内容新快照对象重发（STATE_DELTA 的实际形态）：守卫放行重建，但已分类态不回退
        store.applySnapshot(fixtureSnapshot())
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(Quadrant.Q2, vm.uiState.value.seedCard()?.quadrant)
        assertTrue(vm.uiState.value.classifyingIds.isEmpty())
    }

    @Test
    fun `快照重发后新建任务id不回退`() {
        // WHY：id 序列若随重建回退，重建后再新建会与既有任务 id 冲突，
        // applyClassified 按 id 替换时改错卡片（评审 P1 的次生故障面）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store)

        vm.createTask(CreateTaskInput(title = "本地新建一", ownerType = TaskOwner.BUTLER))
        val firstId = vm.uiState.value.tasks.first { it.title == "本地新建一" }.id

        store.applySnapshot(fixtureSnapshot())
        dispatcher.scheduler.advanceUntilIdle()

        vm.createTask(CreateTaskInput(title = "本地新建二", ownerType = TaskOwner.BUTLER))
        val secondId = vm.uiState.value.tasks.first { it.title == "本地新建二" }.id

        assertNotEquals(firstId, secondId)
    }

    @Test
    fun `解除配对后任务列表与分类态清空`() {
        // WHY：密钥失效自愈路径调 store.clear() 后不导航，任务屏若不响应
        // loaded=false，用户会停留在已解配桌面的陈旧任务数据上（评审 P2）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store)
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.tasks.isNotEmpty())

        store.clear()
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.uiState.value.tasks.isEmpty())
        assertTrue(vm.uiState.value.classifyingIds.isEmpty())
    }
}
