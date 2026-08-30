package com.egosync.companion.ui.memory

import com.egosync.companion.command.CommandException
import com.egosync.companion.command.FakeCommandSender
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.DashboardStatus
import com.egosync.companion.sync.MemoryCategory
import com.egosync.companion.sync.SnapshotDashboard
import com.egosync.companion.sync.SnapshotMetrics
import com.egosync.companion.sync.SnapshotRole
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 记忆页现查语义验证（Story 13.3 T9，AC:5/7）。
 *
 * WHY：记忆内容永不进入快照（AC5 防快照膨胀）——列表/来源/遗忘全部经
 * `COMMAND(memory.*)` 现查现显；失败必须显式（错误态 + 重试），不得用
 * 伪数据或静默空态掩盖（NFR-M3）。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MemoryViewModelTest {

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
        tasks = emptyList(),
        dashboard = SnapshotDashboard(
            statuses = listOf(
                DashboardStatus("role-pm", "产品经理", "target", "#4F46E5", 82, 3,
                    "2026-08-25T07:50:00Z", false),
            ),
            metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-25T08:00:00Z"),
        ),
        conversations = emptyList(),
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    private val memoryListResult = JSONObject(
        """{"memories":[
            {"id":"mem-1","roleId":"role-pm","category":"fact","content":"周报在每周五提交","createdAt":"2026-08-20T10:00:00Z"},
            {"id":"mem-2","roleId":"role-pm","category":"preference","content":"偏好简洁汇报","createdAt":"2026-08-21T10:00:00Z"},
            {"id":"mem-3","roleId":"role-pm","category":"task_status","content":"任务 X 进行中","createdAt":"2026-08-22T10:00:00Z"}
        ]}""",
    )

    private fun newVm(
        commands: FakeCommandSender? = FakeCommandSender { _, _ -> memoryListResult },
        onError: (String) -> Unit = {},
    ): MemoryViewModel {
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        return MemoryViewModel(store, "role-pm", commands, onError)
    }

    @Test
    fun `进屏现查记忆列表且task_status不显示`() {
        // WHY：AC5 契约——记忆永不随快照下发；task_status 是任务域的
        // 派生记忆，显示会造成两屏数据双源漂移。
        val commands = FakeCommandSender { _, _ -> memoryListResult }
        val vm = MemoryViewModel(
            SnapshotStore().apply { applySnapshot(fixtureSnapshot()) },
            "role-pm",
            commands,
        )
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("memory.list", commands.calls.first().first)
        assertEquals("role-pm", commands.paramsOf("memory.list")?.optString("roleId"))
        assertNull(vm.uiState.value.error)
        // task_status 过滤：memories 全量含 mem-3，visibleMemories 不含（双源防呆）
        assertEquals(2, vm.uiState.value.visibleMemories.size)
        assertTrue(vm.uiState.value.memories.any { it.id == "mem-3" })
        assertTrue(vm.uiState.value.visibleMemories.none { it.id == "mem-3" })
    }

    @Test
    fun `类别筛选现查且task_status永不可见`() {
        val commands = FakeCommandSender { _, _ -> memoryListResult }
        val vm = MemoryViewModel(
            SnapshotStore().apply { applySnapshot(fixtureSnapshot()) },
            "role-pm",
            commands,
        )
        dispatcher.scheduler.advanceUntilIdle()

        vm.setCategory(MemoryCategory.FACT)
        dispatcher.scheduler.advanceUntilIdle()

        // 筛选参数下发桌面（桌面过滤）+ 本地防呆双保险
        assertEquals("fact", commands.paramsOf("memory.list")?.optString("category"))
        assertEquals(1, vm.uiState.value.visibleMemories.size)
        assertEquals("mem-1", vm.uiState.value.visibleMemories.single().id)
    }

    @Test
    fun `加载失败错误态重试成功`() {
        // WHY：现查失败若静默，用户看到空列表会误以为角色无记忆——
        // 错误态 + 重试入口是唯一诚实呈现。
        var fail = true
        val commands = FakeCommandSender { _, _ ->
            if (fail) throw CommandException("ConnectionError", "链路中断") else memoryListResult
        }
        val vm = newVm(commands = commands)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("链路中断", vm.uiState.value.error)
        assertTrue(vm.uiState.value.visibleMemories.isEmpty())

        fail = false
        vm.reload()
        dispatcher.scheduler.advanceUntilIdle()

        assertNull(vm.uiState.value.error)
        assertEquals(2, vm.uiState.value.visibleMemories.size)
    }

    @Test
    fun `指令不可达显式失败`() {
        val vm = newVm(commands = null)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("桌面引擎不可达", vm.uiState.value.error)
        assertTrue(vm.uiState.value.visibleMemories.isEmpty())
    }

    @Test
    fun `展开来源懒加载memory点sources`() {
        // WHY：来源消息仅展开时才查（低频深链路），未展开的记忆不产生
        // 任何查询流量。
        val commands = FakeCommandSender { action, _ ->
            if (action == "memory.sources") {
                JSONObject(
                    """{"messages":[
                        {"id":"s-1","role":"user","content":"我周五交周报","createdAt":"2026-08-20T09:00:00Z","isSource":true}
                    ]}""",
                )
            } else memoryListResult
        }
        val vm = newVm(commands = commands)
        dispatcher.scheduler.advanceUntilIdle()

        vm.toggleSource("mem-1")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("memory.sources", commands.calls.map { it.first }.last())
        assertEquals("mem-1", commands.paramsOf("memory.sources")?.optString("memoryId"))
        assertEquals(1, vm.uiState.value.sourcesByMemoryId["mem-1"]?.size)

        // 再次展开同一记忆：已有缓存不重查
        vm.toggleSource("mem-1")
        vm.toggleSource("mem-1")
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, commands.calls.count { it.first == "memory.sources" })
    }

    @Test
    fun `确认遗忘经memory点forget且失败保留`() {
        // WHY：遗忘是破坏性操作——桌面失败时卡片必须保留可重试，
        // 静默移除会让用户以为已遗忘而实际桌面仍在用该记忆。
        var fail = true
        val commands = FakeCommandSender { action, _ ->
            if (action == "memory.forget") {
                if (fail) throw CommandException("NotFound", "记忆不存在") else JSONObject()
            } else memoryListResult
        }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it })
        dispatcher.scheduler.advanceUntilIdle()

        vm.confirmForget("mem-1")
        dispatcher.scheduler.advanceUntilIdle()
        // 失败：卡片保留 + 错误反馈
        assertTrue(vm.uiState.value.memories.any { it.id == "mem-1" })
        assertTrue(errors.any { it.contains("遗忘失败") })

        fail = false
        vm.confirmForget("mem-1")
        dispatcher.scheduler.advanceUntilIdle()
        // 成功：ack 后移出列表
        assertTrue(vm.uiState.value.memories.none { it.id == "mem-1" })
        assertEquals("mem-1", commands.paramsOf("memory.forget")?.optString("memoryId"))
    }

    @Test
    fun `快速切换类别旧回执不得覆盖新结果`() {
        // WHY（评审 C17）：memory.list 无取消防护时，慢的旧查询回执晚于新查询
        // 到达会覆盖最新列表——显示的记亿与选中的类别 chip 不符（数据错位）。
        val factResult = JSONObject(
            """{"memories":[{"id":"mem-fact","roleId":"role-pm","category":"fact","content":"事实","createdAt":"2026-08-20T10:00:00Z"}]}""",
        )
        val prefResult = JSONObject(
            """{"memories":[{"id":"mem-pref","roleId":"role-pm","category":"preference","content":"偏好","createdAt":"2026-08-21T10:00:00Z"}]}""",
        )
        val commands = FakeCommandSender { action, params ->
            if (action != "memory.list") JSONObject()
            else if (params.optString("category") == "fact") {
                kotlinx.coroutines.delay(10_000) // 旧查询慢
                factResult
            } else prefResult
        }
        val vm = newVm(commands = commands)
        dispatcher.scheduler.advanceUntilIdle() // 首查（无类别）

        vm.setCategory(MemoryCategory.FACT) // 慢查询发出
        dispatcher.scheduler.runCurrent()
        vm.setCategory(MemoryCategory.PREFERENCE) // 快查询发出，取消旧的
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, vm.uiState.value.memories.size)
        assertEquals("mem-pref", vm.uiState.value.memories.single().id)
    }

    @Test
    fun `来源加载失败后可显式重试`() {
        // WHY（评审 C18）：失败文案称「请重试」却没有可点入口是断裂交互；
        // 失败不缓存，重试即重发。
        var fail = true
        val commands = FakeCommandSender { action, _ ->
            if (action == "memory.sources") {
                if (fail) throw CommandException("ConnectionError", "链路中断")
                JSONObject(
                    """{"messages":[
                        {"id":"s-1","role":"user","content":"我周五交周报","createdAt":"2026-08-20T09:00:00Z","isSource":true}
                    ]}""",
                )
            } else memoryListResult
        }
        val vm = newVm(commands = commands)
        dispatcher.scheduler.advanceUntilIdle()

        vm.toggleSource("mem-1") // 展开即懒加载：失败
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.uiState.value.sourcesByMemoryId["mem-1"] == null)
        assertFalse(vm.uiState.value.loadingSourceIds.contains("mem-1"))

        fail = false
        vm.retrySources("mem-1")
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, vm.uiState.value.sourcesByMemoryId["mem-1"]?.size)
    }
}
