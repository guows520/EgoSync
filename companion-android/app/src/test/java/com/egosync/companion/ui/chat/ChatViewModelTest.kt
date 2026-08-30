package com.egosync.companion.ui.chat

import com.egosync.companion.command.CommandException
import com.egosync.companion.command.FakeCommandSender
import com.egosync.companion.command.StreamCoordinator
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.SnapshotConversation
import com.egosync.companion.sync.SnapshotDashboard
import com.egosync.companion.sync.SnapshotMessage
import com.egosync.companion.sync.DashboardStatus
import com.egosync.companion.sync.SnapshotMetrics
import com.egosync.companion.sync.SnapshotRole
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 管家对话状态机（Story 13.3 换装：指令通道 + STREAM_TOKEN 驱动）。
 *
 * 发送经 `COMMAND(chat.send)`、回复经 token 流折叠渲染（镜像桌面 ChatStream
 * 消费方式）；多会话/角色隔离语义沿用 13.2 契约。VM 内所有 delay（看门狗）
 * 由测试调度器推进，可精确停在「流式进行中」验证互斥/超时守卫。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ChatViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // 测试夹具种子（镜像旧 mock 种子会话，内容稳定即可，无业务断言依赖其文案）
    private val butlerGreeting = "早上好 boss。今天最重要的一件事是下午 2 点的产品评审。"
    private val butlerHistoryTitle = "上季度 OKR 整理"
    private val butlerCurrentTitle = "今日概览"
    private val butlerHistoryMsgs = listOf(
        ChatMessage("h-1", fromButler = false, text = "帮我把上季度 OKR 整理一下。"),
        ChatMessage("h-2", fromButler = true, text = "好的，我按三个 O 分别列了完成度。"),
    )
    private val rolePmSeedMsgs = listOf(
        ChatMessage("r-pm-1", fromButler = true, senderRoleId = "role-pm", text = "boss，评审材料已备好。"),
    )

    /** 夹具快照：管家 2 会话（当前/历史）+ role-pm 1 会话 + 3 角色。 */
    private fun fixtureSnapshot(): DesktopSnapshot = DesktopSnapshot(
        schemaVersion = 1,
        generatedAt = "2026-08-25T08:00:00Z",
        dataCutoffAt = null,
        truncated = false,
        truncatedDomains = emptyList(),
        roles = listOf(
            SnapshotRole("role-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate"),
            SnapshotRole("role-father", "父亲", "home", "#16A34A", "", "", 56, "moderate"),
            SnapshotRole("role-learner", "学习者", "book-open", "#2563EB", "", "", 33, "moderate"),
        ),
        tasks = emptyList(),
        dashboard = SnapshotDashboard(
            statuses = listOf(
                DashboardStatus("role-pm", "产品经理", "target", "#4F46E5", 82, 3,
                    "2026-08-25T07:50:00Z", false),
                DashboardStatus("role-father", "父亲", "home", "#16A34A", 56, 1,
                    "2026-08-24T20:00:00Z", false),
                DashboardStatus("role-learner", "学习者", "book-open", "#2563EB", 33, 0,
                    "2026-08-21T22:00:00Z", false),
            ),
            metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-25T08:00:00Z"),
        ),
        conversations = listOf(
            SnapshotConversation(
                id = "conv-butler-current", roleId = null, title = butlerCurrentTitle,
                updatedAt = "2026-08-25T06:00:00Z",
                messages = listOf(
                    SnapshotMessage("m-1", "assistant", butlerGreeting, "", true, "2026-08-25T06:00:00Z"),
                ),
            ),
            SnapshotConversation(
                id = "conv-butler-history", roleId = null, title = butlerHistoryTitle,
                updatedAt = "2026-08-24T06:00:00Z",
                messages = butlerHistoryMsgs.mapIndexed { i, m ->
                    SnapshotMessage(m.id, if (m.fromButler) "assistant" else "user", m.text, "", true,
                        "2026-08-24T0${6 + i}:00:00Z")
                },
            ),
            SnapshotConversation(
                id = "conv-pm", roleId = "role-pm", title = "评审材料准备",
                updatedAt = "2026-08-25T07:00:00Z",
                messages = rolePmSeedMsgs.map { m ->
                    SnapshotMessage(m.id, "assistant", m.text, "", true, "2026-08-25T07:00:00Z")
                },
            ),
        ),
        briefings = emptyList(),
        weeklyReviews = emptyList(),
        notifications = emptyList(),
    )

    /** 流式收口后的收敛快照：当前会话含用户消息 + 完整助手回复（id 与 ack 对齐）。 */
    private fun convergedSnapshot(): DesktopSnapshot = fixtureSnapshot().let { snap ->
        snap.copy(
            generatedAt = "2026-08-25T09:00:00Z",
            conversations = snap.conversations.map { conv ->
                if (conv.id == "conv-butler-current") {
                    conv.copy(
                        updatedAt = "2026-08-25T08:30:00Z",
                        messages = conv.messages + listOf(
                            SnapshotMessage("m-user-d", "user", "你好", "", true, "2026-08-25T08:30:00Z"),
                            SnapshotMessage("m-a-d", "assistant", "你好，我在。", "", true, "2026-08-25T08:31:00Z"),
                        ),
                    )
                } else conv
            },
        )
    }

    private fun newVm(
        store: SnapshotStore = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) },
        commands: FakeCommandSender? = FakeCommandSender(),
        onError: (String) -> Unit = {},
    ): ChatViewModel {
        // 流式聚合器与 VM 共享（测试直接注入 token 事件驱动状态机）
        val coordinator = StreamCoordinator { snapshot, raw -> store.applySnapshot(snapshot, raw) }
        return ChatViewModel(store, commands, coordinator, onError)
    }

    private fun ChatViewModel.streamField(): StreamCoordinator =
        ChatViewModel::class.java.getDeclaredField("stream").let {
            it.isAccessible = true
            @Suppress("UNCHECKED_CAST")
            it.get(this) as StreamCoordinator
        }

    /** STREAM_TOKEN payload（桌面 llm:stream StreamPayload 形状）。 */
    private fun tokenJson(
        conversationId: String = "conv-butler-current",
        token: String = "",
        done: Boolean = false,
        thinking: Boolean = false,
        messageId: String? = "m-a-d",
        phase: String? = null,
        statusText: String? = null,
        processEvent: String? = null,
    ): String {
        val json = StringBuilder(
            """{"conversationId":"$conversationId","done":$done,"thinking":$thinking""",
        )
        messageId?.let { json.append(""", "messageId":"$it"""") }
        if (token.isNotEmpty()) json.append(""", "token":"$token"""")
        phase?.let { json.append(""", "phase":"$it"""") }
        statusText?.let { json.append(""", "statusText":"$it"""") }
        processEvent?.let { json.append(""", "processEvent":$it""") }
        json.append("}")
        return json.toString()
    }

    private fun chatSendAck(): JSONObject = JSONObject(
        """{"conversationId":"conv-butler-current","userMessageId":"m-user-d","assistantMessageId":"m-a-d"}""",
    )

    // ── 场景 1：多会话归属（13.2 契约沿用）─────────────────────────

    @Test
    fun `新建会话归属当前角色且不串到其他角色`() {
        // WHY：桌面端每个角色独立维护会话列表；若新建会话串到别的角色，
        // 用户切到角色视图会看到不属于自己的对话，属于数据越权泄漏。
        val vm = newVm()

        vm.newConversation()
        val newId = vm.uiState.value.currentConversationId
        assertEquals(3, vm.uiState.value.conversations.size) // 2 种子 + 1 新建
        assertTrue(vm.uiState.value.messages.isEmpty()) // 新会话为空消息流

        // 切到角色视图：新会话不得出现
        vm.selectRole("role-pm")
        assertFalse(vm.uiState.value.conversations.any { it.id == newId })
        assertEquals(1, vm.uiState.value.conversations.size) // 仅 pm 种子

        // 切回管家：新会话仍在（归属未丢）
        vm.selectRole(null)
        assertEquals(3, vm.uiState.value.conversations.size)
    }

    @Test
    fun `删除会话经指令通知桌面且本地立即移除`() {
        // WHY：手机删除仅移除本地视图时桌面残留——回执快照会把已删会话
        // 「复活」，用户操作被静默吞掉；删除必须同步桌面。
        val commands = FakeCommandSender()
        val vm = newVm(commands = commands)

        vm.deleteConversation("conv-butler-history") // 非当前会话：不触发回退
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.uiState.value.conversations.any { it.id == "conv-butler-history" })
        assertEquals("conversation.delete", commands.calls.map { it.first }.last())
        assertEquals("conv-butler-history", commands.paramsOf("conversation.delete")?.optString("conversationId"))
    }

    // ── 场景 2：发送经指令通道 + ack id 对齐（AC:3）───────────────

    @Test
    fun `发送经chat点send指令且回显气泡按ack对齐id`() {
        // WHY：本地临时 id 若不替换为桌面 id，流结束后的快照替换会以
        // 不同 key 重建消息列表——气泡闪烁且本地回写错位。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent() // 精确停在流式窗口内（advanceUntilIdle 会烧掉 120s 看门狗）

        // 指令已发出且参数完整（camelCase，桌面 deny_unknown_fields 校验）
        assertEquals("chat.send", commands.calls.first().first)
        val params = commands.paramsOf("chat.send")!!
        assertEquals("你好", params.optString("content"))
        assertEquals("conv-butler-current", params.optString("conversationId"))

        // 用户气泡即时回显且 id 已对齐桌面
        val userMsg = vm.uiState.value.messages.last { !it.fromButler }
        assertEquals("m-user-d", userMsg.id)
        assertEquals("你好", userMsg.text)

        // ack 后进入流式等待（thinking），不悬挂
        assertTrue(vm.uiState.value.thinking)
        assertTrue(vm.uiState.value.responding)
    }

    @Test
    fun `发送失败显式反馈且不悬挂`() {
        // WHY：指令失败若静默，用户消息发出后永远停在「思考中」——
        // 无反馈无出路，比报错更糟（NFR-M3 不伪造回执）。
        val commands = FakeCommandSender { _, _ ->
             throw CommandException("ConnectionError", "桌面引擎不可达")
         }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it })

        vm.sendMessage("你好")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf("桌面引擎不可达"), errors)
        assertFalse(vm.uiState.value.responding)
        assertFalse(vm.uiState.value.thinking)
        // 用户消息保留（重试语义），但不出现任何伪造回复
        //（种子会话自带 1 条管家问候气泡——只断言无新增）
        assertEquals(1, vm.uiState.value.messages.count { it.fromButler })
    }

    // ── 场景 3：STREAM_TOKEN 驱动流式状态机（AC:3）────────────────

    @Test
    fun `token流驱动渲染工具行与done收口并现查建议`() {
        // WHY：流式体验 = 桌面 ChatStream 的手机镜像；done 后快照域无
        // suggestions（Dev Notes §4 裁决），建议必须现查。
        val commands = FakeCommandSender { action, _ ->
            if (action == "chat.send") chatSendAck()
            else JSONObject("""{"suggestions":[{"id":"s-1","title":"排进明天上午","content":"明早有空","roleName":"产品经理"}]}""")
        }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()

        // 工具状态行（phase=tool）
        coordinator.onStreamToken(tokenJson(phase = "tool", statusText = "检索记忆"))
        dispatcher.scheduler.runCurrent()
        assertEquals("检索记忆", vm.uiState.value.streamingToolTitle)

        // 文本 token 逐字渲染
        coordinator.onStreamToken(tokenJson(token = "你"))
        coordinator.onStreamToken(tokenJson(token = "好，"))
        dispatcher.scheduler.runCurrent()
        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("你好，", streaming.text)
        assertTrue(streaming.streaming)

        // done 收口
        coordinator.onStreamToken(tokenJson(token = "我在。", done = true))
        dispatcher.scheduler.runCurrent()

        val finalMsg = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("你好，我在。", finalMsg.text)
        assertFalse(finalMsg.streaming)
        assertFalse(vm.uiState.value.responding)
        assertNull(vm.uiState.value.streamingToolTitle)

        // 流结束现查 pending 建议（快照无此域）
        assertNotNull(commands.paramsOf("suggestion.list"))
        assertEquals("conv-butler-current", commands.paramsOf("suggestion.list")?.optString("conversationId"))
        assertEquals(1, vm.uiState.value.actionCards.size)
        assertEquals("s-1", vm.uiState.value.actionCards.first().id)
    }

    @Test
    fun `停止发chat点stop指令且已浮现文本落定`() {
        // WHY：FR-33 停止 = 桌面真实终止生成；本地若只掐渲染不通知桌面，
        // 桌面继续产出并在快照中回流完整回复，停止形同虚设。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "已浮现的部"))
        dispatcher.scheduler.runCurrent()

        vm.stopStreaming()
        dispatcher.scheduler.runCurrent()

        assertEquals("chat.stop", commands.calls.map { it.first }.last())
        assertEquals("conv-butler-current", commands.paramsOf("chat.stop")?.optString("conversationId"))
        // 已浮现内容不回滚（镜像桌面：停止保留已生成部分）
        assertTrue(vm.uiState.value.messages.any { it.fromButler && it.text == "已浮现的部" && !it.streaming })
        assertFalse(vm.uiState.value.responding)
    }

    @Test
    fun `看门狗超时错误反馈可重试`() {
        // WHY：链路死亡（token 断流）时若不看门狗，气泡永远停在流式态。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it })
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "部分"))
        dispatcher.scheduler.runCurrent()

        // 120s 无 token 进展 → 超时收口
        dispatcher.scheduler.advanceTimeBy(121_000)
        dispatcher.scheduler.runCurrent()

        assertTrue(errors.any { it.contains("超时") })
        assertFalse(vm.uiState.value.responding)
        // 超时后可再发（看门狗/发送态已复位）
        vm.sendMessage("重试")
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.responding)
    }

    // ── 场景 4：快照-流式互斥（Dev Notes §4 裁决）────────────────

    @Test
    fun `流式期间快照暂存不掐气泡流结束补应用收敛`() {
        // WHY：chat 写入触发 2s debounce 快照重建，流式中段 STATE_DELTA
        // 必然到达；立即应用会触发 onSnapshotReplaced → stopStreaming
        // 掐掉正在流式的气泡（13.2 既有守卫）——暂存是最小正确解。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val coordinator = StreamCoordinator { snapshot, raw -> store.applySnapshot(snapshot, raw) }
        val vm = ChatViewModel(store, commands, coordinator)

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "部分回复"))
        dispatcher.scheduler.runCurrent()

        // 流式中段快照到达：走暂存门（frameConsumerFactory 经 deliverSnapshot）
        coordinator.deliverSnapshot(convergedSnapshot(), null)
        dispatcher.scheduler.runCurrent()

        // 气泡未被掐：流式态保持、消息不被快照替换清掉
        assertTrue(vm.uiState.value.responding)
        assertTrue(vm.uiState.value.messages.any { it.fromButler && it.text == "部分回复" && it.streaming })

        // done → 暂存快照补应用 → VM 按快照收敛（id 已对齐，无重复气泡）
        coordinator.onStreamToken(tokenJson(done = true))
        dispatcher.scheduler.runCurrent()

        val messages = vm.uiState.value.messages
        assertEquals(1, messages.count { it.id == "m-user-d" })
        assertEquals(1, messages.count { it.id == "m-a-d" })
        assertFalse(vm.uiState.value.responding)
    }

    // ── 场景 5：停止/断连/非查看会话收口（评审 C1/C3）─────────────

    @Test
    fun `停止掐灭ack等待中的发送协程不复活流`() {
        // WHY（评审 C1）：ack 等待窗口点停止后，存活的 chat.send 协程会在 ack
        // 返回时继续 streamStarting + 重臂看门狗——已停止的流「复活」，取消
        // 语义倒退。停止必须掐灭协程本身。
        val commands = FakeCommandSender { action, _ ->
            if (action == "chat.send") {
                kotlinx.coroutines.delay(60_000) // ack 等待窗口
                chatSendAck()
            } else JSONObject()
        }
        val vm = newVm(commands = commands)

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.responding)

        vm.stopStreaming()
        dispatcher.scheduler.runCurrent()
        assertFalse(vm.uiState.value.responding)

        // ack 时刻到达：被取消的协程不得复活流（不得 streamStarting/重臂看门狗）
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.uiState.value.responding)
        assertFalse(vm.uiState.value.thinking)
        assertNull(vm.streamField().state.value)
    }

    @Test
    fun `非查看会话的流done后段落落库不凭空消失`() {
        // WHY（评审 C3）：done 收口不得以「正在查看该会话」为前提——流式中切走
        // （或桌面自起流）后 done 到达，文本仍须写入该会话历史，否则切回时
        // 已流出文本凭空消失、streamEnded 不调用。
        val commands = FakeCommandSender { _, _ -> JSONObject() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()
        val historyId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id

        // 桌面在「历史会话」发起流（当前查看的是今日概览）
        coordinator.onStreamToken(tokenJson(conversationId = historyId, token = "桌面侧回复", messageId = "m-x"))
        dispatcher.scheduler.runCurrent()
        // 非查看会话不占全局流式态（不锁发送守卫）
        assertFalse(vm.uiState.value.responding)
        coordinator.onStreamToken(tokenJson(conversationId = historyId, token = "。", done = true, messageId = "m-x"))
        dispatcher.scheduler.runCurrent()

        // 段落已写入历史会话：切过去即可见（不凭空消失）
        vm.selectConversation(historyId)
        assertTrue(vm.uiState.value.messages.any { it.text == "桌面侧回复。" })
    }

    @Test
    fun `新建后立即发消息时孤儿桌面会话被回收`() {
        // WHY（评审 C8）：conversation.new 与首条 chat.send 并发时桌面产生两个
        // 会话——本地 id 已被 chat.send 采纳替换后，new 的 ack 落空须删除孤儿，
        // 否则桌面永久残留无入口的空会话。
        val commands = FakeCommandSender { action, _ ->
            when (action) {
                "conversation.new" -> {
                    kotlinx.coroutines.delay(2_000) // new 慢于 send：竞速窗口
                    JSONObject("""{"id":"conv-desktop-new"}""")
                }
                "chat.send" -> JSONObject(
                    """{"conversationId":"conv-desktop-send","userMessageId":"u-1","assistantMessageId":"a-1"}""",
                )
                else -> JSONObject()
            }
        }
        val vm = newVm(commands = commands)

        vm.newConversation()
        dispatcher.scheduler.runCurrent() // new 已发出（ack 在途）
        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent() // send ack 先到：本地 id 被采纳替换

        dispatcher.scheduler.advanceUntilIdle() // new ack 到达：落空 → 回收孤儿

        assertEquals("conv-desktop-new", commands.paramsOf("conversation.delete")?.optString("conversationId"))
    }

    // ── 场景 6：13.2 快照/会话管理守护（评审 D2 补回，零回归测试面）──

    @Test
    fun `新建会话设为当前且列表最新在前`() {
        // WHY：新建后必须立即激活，否则用户发的消息会写进旧会话，
        // 「开新话题」的心智模型直接破裂。
        val vm = newVm()
        val before = vm.uiState.value.currentConversationId

        vm.newConversation()

        assertNotEquals(before, vm.uiState.value.currentConversationId)
        assertEquals(
            vm.uiState.value.currentConversationId,
            vm.uiState.value.conversations.first().id,
        )
    }

    @Test
    fun `切换会话换消息流且可切回`() {
        // WHY：历史下拉的唯一价值就是「换到哪个会话就看到哪个会话的完整消息流」；
        // 切换后消息流不随动 = 功能失效。
        val vm = newVm()
        val historyConv = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }
        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })

        vm.selectConversation(historyConv.id)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertEquals(historyConv.id, vm.uiState.value.currentConversationId)

        // 切回当前种子会话，消息流恢复
        val currentConv = vm.uiState.value.conversations.first { it.title == butlerCurrentTitle }
        vm.selectConversation(currentConv.id)
        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })
    }

    @Test
    fun `删除当前会话回退到剩余最新会话`() {
        // WHY：删的正是正在看的会话时必须有兜底落点，否则 currentConversationId
        // 悬空，后续发消息会写进不存在的会话（persist 静默丢失）。
        val vm = newVm()
        val okrConv = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }

        vm.deleteConversation(vm.uiState.value.currentConversationId)

        assertEquals(okrConv.id, vm.uiState.value.currentConversationId)
        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertEquals(1, vm.uiState.value.conversations.size)
    }

    @Test
    fun `删光所有会话自动新建空会话兜底`() {
        // WHY：会话列表为空时界面不能进入「无当前会话」死态——
        // 输入框永远无处可发，用户只能重启 App，这是体验断裂。
        val vm = newVm()
        vm.deleteConversation(vm.uiState.value.currentConversationId) // 剩 OKR 种子
        dispatcher.scheduler.advanceUntilIdle()
        vm.deleteConversation(vm.uiState.value.currentConversationId) // 全删

        assertEquals(1, vm.uiState.value.conversations.size)
        assertTrue(vm.uiState.value.currentConversationId.isNotEmpty())
        assertTrue(vm.uiState.value.messages.isEmpty())
        assertTrue(vm.uiState.value.conversations.first().title.isEmpty())
    }

    @Test
    fun `空会话首条消息生成标题且超长截断加省略号`() {
        // WHY：历史下拉靠标题辨认会话；空标题会话永远显示「新对话」，
        // 多条之后用户将无法找回任何一段对话（镜像桌面会话命名规则）。
        val vm = newVm()
        vm.newConversation()

        vm.sendMessage("你好") // 同步落标题，无需推进调度器
        assertEquals(
            "你好",
            vm.uiState.value.conversations.first { it.id == vm.uiState.value.currentConversationId }.title,
        )

        vm.newConversation()
        vm.sendMessage("abcdefghijklmnopqrst") // 20 字符 > 16
        assertEquals(
            "abcdefghijklmnop…",
            vm.uiState.value.conversations.first { it.id == vm.uiState.value.currentConversationId }.title,
        )
    }

    @Test
    fun `同对象首发不重建_本地暂存不被首帧抹掉`() {
        // WHY（13.2 评审 P14 守护）：构造时已加载的快照会作为 collect 首帧再次
        // 投递；无同一性守卫时首帧重建会 seedFromStore 抹掉首帧之前的本地暂存。
        val vm = newVm()

        vm.newConversation()
        dispatcher.scheduler.advanceUntilIdle() // 驱动 collect 首帧

        assertEquals(3, vm.uiState.value.conversations.size) // 2 种子 + 1 本地新建
    }

    @Test
    fun `二次applySnapshot后消息与会话随新快照刷新`() {
        // WHY（AC2 守护）：STATE_DELTA 全量替换后，消息流与会话列表必须随新快照
        // 刷新（含当前会话内桌面侧新增的消息），否则换装名存实亡；同时当前会话
        // 保持不跳（快照替换不重置用户视图）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store)
        dispatcher.scheduler.advanceUntilIdle()

        val second = fixtureSnapshot().copy(
            conversations = fixtureSnapshot().conversations.map { conv ->
                if (conv.id == "conv-butler-current") {
                    conv.copy(
                        messages = conv.messages + SnapshotMessage(
                            "m-2", "assistant", "桌面侧新增的回复", "", true, "2026-08-25T09:00:00Z",
                        ),
                    )
                } else {
                    conv
                }
            } + SnapshotConversation(
                id = "conv-butler-new", roleId = null, title = "桌面新建会话",
                updatedAt = "2026-08-25T09:30:00Z", messages = emptyList(),
            ),
        )
        store.applySnapshot(second)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            listOf(butlerGreeting, "桌面侧新增的回复"),
            vm.uiState.value.messages.map { it.text },
        )
        assertEquals("conv-butler-current", vm.uiState.value.currentConversationId)
        // 管家视角：2 个种子会话 + 桌面新建会话（role-pm 会话在角色视图，不计入）
        assertEquals(3, vm.uiState.value.conversations.size)
    }

    @Test
    fun `冷启动空快照兜底建会话_快照到达后重建`() {
        // WHY（守护）：离线冷启动（无缓存）时 Chat 不能进入「无当前会话」死态；
        // 快照随后到达时必须自动重建，会话列表与消息流切换到快照口径。
        val store = SnapshotStore()
        val vm = newVm(store)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, vm.uiState.value.conversations.size) // 兜底空会话
        assertTrue(vm.uiState.value.messages.isEmpty())

        store.applySnapshot(fixtureSnapshot())
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf(butlerGreeting), vm.uiState.value.messages.map { it.text })
        assertEquals(3, vm.uiState.value.roles.size)
    }

    @Test
    fun `快照删除当前会话回落时卡片态清空`() {
        // WHY（13.2 评审 P4 守护）：回落换会话必须遵守 selectConversation 的同一
        // 不变量——卡片态属触发它的那段对话；桌面删除当前会话导致的回落若不清
        // 卡片，用户会在不相干的会话里确认旧建议。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val commands = FakeCommandSender { action, _ ->
            if (action == "chat.send") chatSendAck()
            else JSONObject(
                """{"suggestions":[{"id":"s-1","title":"排进明天上午","content":"明早有空","roleName":"产品经理"}]}""",
            )
        }
        val vm = newVm(store, commands)
        val coordinator = vm.streamField()

        vm.sendMessage("第一轮")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "回复", done = true))
        dispatcher.scheduler.runCurrent()
        assertFalse(vm.uiState.value.actionCards.isEmpty())

        val reduced = fixtureSnapshot().copy(
            conversations = fixtureSnapshot().conversations.filterNot { it.id == "conv-butler-current" },
        )
        store.applySnapshot(reduced)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(butlerHistoryMsgs.map { it.text }, vm.uiState.value.messages.map { it.text })
        assertTrue(vm.uiState.value.actionCards.isEmpty())
        assertNull(vm.uiState.value.decomposition)
        assertTrue(vm.uiState.value.traceByMessageId.isEmpty())
    }

    @Test
    fun `当前查看角色被快照删除后回退管家视图`() {
        // WHY（13.2 评审 P5 守护）：角色被桌面删除后，切换器已无该角色入口；
        // 若手机仍停留在已删角色的空视图，用户没有直观路径回到管家（幽灵视图）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val vm = newVm(store)
        // 先启动 collect 协程（同对象首帧跳过）：lastSnapshot 捕获发生在协程体首跑时
        dispatcher.scheduler.runCurrent()
        vm.selectRole("role-pm")

        // store.roles 门面从 dashboard.statuses 派生——模拟删除角色须同步删状态行
        val reduced = fixtureSnapshot().copy(
            roles = fixtureSnapshot().roles.filterNot { it.id == "role-pm" },
            dashboard = fixtureSnapshot().dashboard.copy(
                statuses = fixtureSnapshot().dashboard.statuses.filterNot { it.roleId == "role-pm" },
            ),
            conversations = fixtureSnapshot().conversations.filterNot { it.roleId == "role-pm" },
        )
        store.applySnapshot(reduced)
        dispatcher.scheduler.advanceUntilIdle()

        assertNull(vm.uiState.value.activeRoleId)
        assertTrue(vm.uiState.value.roles.none { it.id == "role-pm" })
    }
}
