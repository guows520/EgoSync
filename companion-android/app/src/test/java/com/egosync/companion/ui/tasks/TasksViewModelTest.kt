package com.egosync.companion.ui.tasks

import com.egosync.companion.command.CommandException
import com.egosync.companion.command.FakeCommandSender
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
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 任务屏指令通道换装语义验证（Story 13.3 T9，AC:4/7）。
 *
 * - 勾选/新建经 `COMMAND(task.toggle/task.create)` 由桌面真实执行（乐观更新 +
 *   失败回滚；临时 id 替换为桌面 id；智能分类经 STATE_DELTA 收敛）；
 * - 快照同一性守卫（评审 P1）：state 重发不回滚乐观态；unpair 自愈清空。
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

    private fun newVm(
        store: SnapshotStore = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) },
        commands: FakeCommandSender? = FakeCommandSender(),
        onError: (String) -> Unit = {},
    ): TasksViewModel = TasksViewModel(store, commands, onError)

    // ── 勾选：乐观翻转 + 失败回滚（AC:4）──────────────────────────

    @Test
    fun `勾选乐观翻转成功保持`() {
        // WHY：本地翻转即时反馈；成功后 STATE_DELTA 会再次收敛同一终态——
        // 这里验证乐观翻转在成功路径不被回滚。
        val vm = newVm()
        vm.toggleTask("t-1")
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done)
    }

    @Test
    fun `勾选失败回滚并提示`() {
        // WHY：失败若不回滚，UI 显示勾选但桌面未变更——下次快照替换
        // 会「弹回」未勾选态，用户操作被静默吞掉。
        val commands = FakeCommandSender { _, _ ->
             throw CommandException("ValidationError", "任务已删除")
         }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it })

        vm.toggleTask("t-1")
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.uiState.value.tasks.first { it.id == "t-1" }.done)
        assertTrue(errors.any { it.contains("任务操作失败") })
    }

    @Test
    fun `指令不可达时勾选回滚且不悬挂`() {
        // WHY：离线/fake 态勾选必须显式失败——不能假装勾选成功（NFR-M3）。
        val errors = mutableListOf<String>()
        val vm = newVm(commands = null, onError = { errors += it })

        vm.toggleTask("t-1")
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.uiState.value.tasks.first { it.id == "t-1" }.done)
        assertTrue(errors.any { it.contains("桌面引擎不可达") })
    }

    // ── 新建：临时 id 替换 + 分类收敛（AC:4）──────────────────────

    @Test
    fun `新建经task点create且临时id替换为桌面id`() {
        // WHY：临时 id 若不替换，快照替换会把乐观卡片与桌面任务并列为
        // 重复卡片；懒列表 key 不稳会闪烁。
        val commands = FakeCommandSender { _, _ ->
            JSONObject("""{"id":"t-desktop","quadrant":"Q2","isCompleted":false}""")
        }
        val vm = newVm(commands = commands)

        vm.createTask(
            CreateTaskInput(title = "整理客户反馈", ownerType = TaskOwner.BUTLER, quadrant = Quadrant.Q2),
        )
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("task.create", commands.calls.first().first)
        assertEquals("整理客户反馈", commands.paramsOf("task.create")?.optString("title"))
        assertEquals("Q2", commands.paramsOf("task.create")?.optString("quadrant"))
        assertTrue(vm.uiState.value.tasks.any { it.id == "t-desktop" })
        assertFalse(vm.uiState.value.tasks.any { it.id.startsWith("local-") })
        assertFalse(vm.uiState.value.classifyingIds.contains("t-desktop"))
    }

    @Test
    fun `未指定象限占位Q2分类中标记仅超时收口`() {
        // WHY（评审 D1-A）：桌面对未指定象限默认落库 Q2（normalized_quadrant
        // unwrap_or("Q2")）——若占位 Q3 且以「象限脱离 Q3」判收敛，首个
        // STATE_DELTA（约 2s）即误清「分类中」，而 LLM 分类最长 12s 才终态，
        // 改判期间无过渡提示。占位对齐 Q2 后象限无假跳变，标记由 15s 超时兜底。
        val commands = FakeCommandSender { _, _ ->
            JSONObject("""{"id":"t-desktop","quadrant":"Q2","isCompleted":false}""")
        }
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store, commands)

        vm.createTask(
            CreateTaskInput(title = "待分类任务", ownerType = TaskOwner.BUTLER, quadrant = null),
        )
        dispatcher.scheduler.runCurrent() // 精确停在分类窗口内（15s 超时兜底不触发）

        // 占位象限 = 桌面默认 Q2（ack 象限一致，无 Q3→Q2 假跳变）
        assertEquals(Quadrant.Q2, vm.uiState.value.tasks.first { it.id == "t-desktop" }.quadrant)
        assertTrue(vm.uiState.value.classifyingIds.contains("t-desktop"))

        // 分类窗口内 STATE_DELTA 到达（象限仍 Q2，LLM 尚未改判）：
        // 「分类中」标记不得被误清（收敛只认 15s 超时）
        val midClassify = fixtureSnapshot().copy(
            tasks = fixtureSnapshot().tasks + listOf(
                SnapshotTask(
                    "t-desktop", "butler", null, "待分类任务", null, "Q2", false, false,
                    "none", "管家", null,
                ),
            ),
            generatedAt = "2026-08-25T09:00:00Z",
        )
        store.applySnapshot(midClassify)
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.classifyingIds.contains("t-desktop"))

        // 分类终态改判 Q1：STATE_DELTA 收敛象限（LLM 已裁决）
        val classified = fixtureSnapshot().copy(
            tasks = fixtureSnapshot().tasks + listOf(
                SnapshotTask(
                    "t-desktop", "butler", null, "待分类任务", null, "Q1", false, false,
                    "none", "管家", null,
                ),
            ),
            generatedAt = "2026-08-25T09:00:30Z",
        )
        store.applySnapshot(classified)
        dispatcher.scheduler.runCurrent()
        assertEquals(Quadrant.Q1, vm.uiState.value.tasks.first { it.id == "t-desktop" }.quadrant)

        // 15s 超时兜底清标记（task_classifier 固定截止）
        dispatcher.scheduler.advanceTimeBy(15_000)
        dispatcher.scheduler.runCurrent()
        assertFalse(vm.uiState.value.classifyingIds.contains("t-desktop"))
    }

    @Test
    fun `临时id序列唯一`() {
        // WHY：临时 id 重复会导致 LazyList key 冲突——重复渲染/错位。
        // 断言对象是乐观临时 id（同步上屏即读），advanceUntilIdle 后读到的
        // 已是 fake 返回的桌面 id——断言 fake 自身产物是恒真测试（评审 C20）。
        val commands = FakeCommandSender { _, _ ->
            JSONObject("""{"id":"t-d","quadrant":"Q1","isCompleted":false}""")
        }
        val vm = newVm(commands = commands)

        vm.createTask(CreateTaskInput(title = "一", ownerType = TaskOwner.BUTLER, quadrant = Quadrant.Q1))
        val firstId = vm.uiState.value.tasks.first { it.title == "一" }.id
        assertTrue(firstId.startsWith("local-"))
        vm.createTask(CreateTaskInput(title = "二", ownerType = TaskOwner.BUTLER, quadrant = Quadrant.Q1))
        val secondId = vm.uiState.value.tasks.first { it.title == "二" }.id
        assertTrue(secondId.startsWith("local-"))

        assertNotEquals(firstId, secondId)
    }

    @Test
    fun `在途勾选不被无关快照回滚`() {
        // WHY（评审 C14）：toggle 写信号快照（约 2s）到达前，无关 STATE_DELTA
        // 会用桌面旧态覆盖乐观翻转——视觉回滚后再跳变，像「点了没反应」。
        val commands = FakeCommandSender { _, _ ->
            kotlinx.coroutines.delay(10_000) // ack 在途
            JSONObject()
        }
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store, commands)

        vm.toggleTask("t-1")
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done)

        // 无关快照（新对象）携带桌面旧态 done=false：在途翻转保持
        store.applySnapshot(fixtureSnapshot().copy(generatedAt = "2026-08-25T09:00:00Z"))
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done)
    }

    @Test
    fun `同任务在途期间连点仅受理首次`() {
        // WHY（评审 C14）：多次乐观翻转与回滚交错会让本地态偏离桌面且无收敛
        // 信号——在途守卫（同任务 ack 未回不受理）是最小正确解。
        var sent = 0
        val commands = FakeCommandSender { action, _ ->
            if (action == "task.toggle") sent += 1
            kotlinx.coroutines.delay(10_000)
            JSONObject()
        }
        val vm = newVm(commands = commands)

        vm.toggleTask("t-1")
        dispatcher.scheduler.runCurrent()
        vm.toggleTask("t-1") // 在途二次点击：忽略
        dispatcher.scheduler.runCurrent()
        assertEquals(1, sent)

        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done) // 首次翻转成功保持
    }

    @Test
    fun `快照先含桌面任务时ack去重不双卡`() {
        // WHY（评审 C15）：写信号快照先于 ack 到达时，乐观卡「快照不含即追加」
        // 与「ack 换 id」叠加会出现两张同 id 卡片（LazyColumn key 冲突）。
        val commands = FakeCommandSender { _, _ ->
            kotlinx.coroutines.delay(5_000) // 让快照先行
            JSONObject("""{"id":"t-desktop","quadrant":"Q2","isCompleted":false}""")
        }
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = TasksViewModel(store, commands)

        vm.createTask(
            CreateTaskInput(title = "整理客户反馈", ownerType = TaskOwner.BUTLER, quadrant = Quadrant.Q2),
        )
        dispatcher.scheduler.runCurrent()

        // 快照携带桌面任务（ack 未回，乐观 local- 卡仍在）
        store.applySnapshot(fixtureSnapshot().copy(
            tasks = fixtureSnapshot().tasks + listOf(
                SnapshotTask(
                    "t-desktop", "butler", null, "整理客户反馈", null, "Q2", false, false,
                    "none", "管家", null,
                ),
            ),
            generatedAt = "2026-08-25T09:00:00Z",
        ))
        dispatcher.scheduler.runCurrent()

        // ack 到达：去重乐观卡，不替换出第二张同 id 卡
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, vm.uiState.value.tasks.count { it.id == "t-desktop" })
        assertFalse(vm.uiState.value.tasks.any { it.id.startsWith("local-") })
    }

    @Test
    fun `ack缺id移除乐观卡并显式报错`() {
        // WHY（评审 C15）：ack 成功但缺 id 时乐观卡永不与快照对齐（永久滞留）——
        // 契约破坏必须显式失败，不留孤儿卡片。
        val commands = FakeCommandSender { _, _ -> JSONObject() }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it })

        vm.createTask(
            CreateTaskInput(title = "整理客户反馈", ownerType = TaskOwner.BUTLER, quadrant = Quadrant.Q2),
        )
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.uiState.value.tasks.any { it.title == "整理客户反馈" })
        assertTrue(errors.any { it.contains("任务 id") })
    }

    // ── 快照同一性守卫（评审 P1/P2，13.2 契约沿用）────────────────

    @Test
    fun `快照重发不回滚乐观状态`() {
        // WHY：collect 无同一性守卫时，任何 state 重发都执行 store.tasks
        // 重建——已勾选的乐观态被快照旧态回滚（评审 P1）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store = store)
        vm.toggleTask("t-1")
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done)

        // 同一快照对象重发：不得触发重建
        store.applySnapshot(fixtureSnapshot())
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.tasks.first { it.id == "t-1" }.done)
    }

    @Test
    fun `解除配对后任务列表与分类态清空`() {
        // WHY：密钥失效自愈路径调 store.clear() 后不导航，任务屏若不响应
        // loaded=false，用户会停留在已解配桌面的陈旧任务数据上（评审 P2）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store = store)
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.tasks.isNotEmpty())

        store.clear()
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.uiState.value.tasks.isEmpty())
        assertTrue(vm.uiState.value.classifyingIds.isEmpty())
    }
}
