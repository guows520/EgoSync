package com.egosync.companion.ui.chat

import com.egosync.companion.command.CommandException
import com.egosync.companion.command.FakeCommandSender
import com.egosync.companion.command.StreamCoordinator
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.ChatOutbox
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
import kotlinx.coroutines.flow.MutableStateFlow
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
        outbox: ChatOutbox? = null,
        commandReady: MutableStateFlow<Boolean> = MutableStateFlow(true),
    ): ChatViewModel {
        // 流式聚合器与 VM 共享（测试直接注入 token 事件驱动状态机）
        val coordinator = StreamCoordinator { snapshot, raw -> store.applySnapshot(snapshot, raw) }
        return ChatViewModel(store, commands, coordinator, onError, outbox, commandReady, MutableStateFlow(true))
    }

    private fun ChatViewModel.streamField(): StreamCoordinator =
        ChatViewModel::class.java.getDeclaredField("stream").let {
            it.isAccessible = true
            @Suppress("UNCHECKED_CAST")
            it.get(this) as StreamCoordinator
        }

    /** STREAM_TOKEN payload（桌面 llm:stream StreamPayload 形状：正文默认 phase="answering"）。 */
    private fun tokenJson(
        conversationId: String = "conv-butler-current",
        token: String = "",
        done: Boolean = false,
        thinking: Boolean = false,
        messageId: String? = "m-a-d",
        // 生产正文形状（agent_engine.rs emit_stream_token）；历史 SSE 收口/错误
        // 帧按用例显式传 null（serde skip 无 phase 字段）
        phase: String? = "answering",
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

    // ── 场景 T-S10：离线待发箱（入队 / 串行 flush / 幂等 / 恢复）────

    @Test
    fun `离线发送入队待发态且不报桌面引擎不可达`() {
        // WHY（SPEC state-model §4.4 用户裁决：队列化）：!commandReady 发送不禁用
        // 也不报错——入待发箱、气泡待发态、恢复后自动发送；若仍弹「桌面引擎
        // 不可达」即旧遮罩阻断形态借发送路径复活。
        val errors = mutableListOf<String>()
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()
        val vm = newVm(onError = { errors += it }, outbox = outbox, commandReady = gate)

        vm.sendMessage("离线备忘")
        dispatcher.scheduler.runCurrent()

        val bubble = vm.uiState.value.messages.last { !it.fromButler }
        assertTrue("离线气泡必须呈待发态", bubble.pending)
        assertFalse("入队不进入流式（待发条目不置 responding，SPEC §4.4 并发守卫）", vm.uiState.value.thinking)
        assertFalse(vm.uiState.value.responding)
        assertEquals(1, outbox.entries.value.size)
        val entry = outbox.entries.value.single()
        assertEquals("离线备忘", entry.content)
        assertEquals(bubble.id, entry.localMessageId)
        assertEquals("对齐会话按 id 入队（恢复后发回原会话）", "conv-butler-current", entry.conversationId)
        assertTrue("无「桌面引擎不可达」错误路径触发", errors.isEmpty())
    }

    @Test
    fun `恢复后逐条串行重发且等本轮流式收口`() {
        // WHY（SPEC §4.4）：flush 一次一条、等本轮流式 done 后再发下一条（对齐
        // 桌面 busy 串行）——并行发出会交错流式渲染，两条回复互相覆盖。
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()
        val commands = FakeCommandSender { action, params ->
            if (action == "chat.send") {
                JSONObject()
                    .put("conversationId", "conv-butler-current")
                    .put("userMessageId", "desk-" + params.optString("content"))
            } else {
                JSONObject()
            }
        }
        val vm = newVm(commands = commands, outbox = outbox, commandReady = gate)
        val coordinator = vm.streamField()

        vm.sendMessage("第一条")
        vm.sendMessage("第二条")
        dispatcher.scheduler.runCurrent()
        assertEquals(listOf("第一条", "第二条"), outbox.entries.value.map { it.content })
        val firstEntryCommandId = outbox.entries.value.first().commandId

        gate.value = true
        dispatcher.scheduler.runCurrent()

        // 第一条已发出（携带条目 commandId），流式进行中——第二条必须等待
        assertEquals(1, commands.calls.count { it.first == "chat.send" })
        assertEquals(firstEntryCommandId, commands.passedCommandIds.first())
        assertTrue(vm.uiState.value.responding)

        coordinator.onStreamToken(tokenJson(token = "收到", done = true, phase = null))
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("流式收口后第二条才发出（串行）", 2, commands.calls.count { it.first == "chat.send" })
        assertEquals("成功条目删队", 0, outbox.entries.value.size)
        assertFalse("待发气泡全部转常态", vm.uiState.value.messages.any { it.pending })
    }

    @Test
    fun `待发箱重发ack锚点传入渲染id用锚点`() {
        // WHY（M1'）：重发路径的锚点契约与直发一致——ack 的 assistantMessageId
        // 若未传入 streamStarting，离线队列轮次回退合成 id（快照落地 key 翻转、
        // 溯源孤儿化、守卫降级末条代理），恰是本修复消灭的闪烁类缺陷。
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()
        val commands = FakeCommandSender { action, _ ->
            if (action == "chat.send") {
                JSONObject()
                    .put("conversationId", "conv-butler-current")
                    .put("userMessageId", "desk-u-1")
                    .put("assistantMessageId", "m-a-d")
            } else {
                JSONObject()
            }
        }
        val vm = newVm(commands = commands, outbox = outbox, commandReady = gate)
        val coordinator = vm.streamField()

        vm.sendMessage("离线备忘")
        dispatcher.scheduler.runCurrent()
        gate.value = true
        dispatcher.scheduler.runCurrent()
        assertEquals("重发已发出", 1, commands.calls.count { it.first == "chat.send" })

        coordinator.onStreamToken(tokenJson(token = "重发后的回复", messageId = null))
        dispatcher.scheduler.runCurrent()
        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("重发轮次渲染 id 用 ack 锚点", "m-a-d", streaming.id)
        assertTrue(streaming.streaming)
    }

    @Test
    fun `连接类失败留队静默且重发复用同一commandId`() {
        // WHY（SPEC §4.4 幂等）：连接类失败是常态而非错误——条目留队等恢复；
        // 重发复用同一 commandId，桌面 dispatcher 幂等缓存命中首次结果，
        // 网络重传/竞窗重发零重复执行。
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()
        val commands = FakeCommandSender()
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it }, outbox = outbox, commandReady = gate)

        vm.sendMessage("断网备忘")
        dispatcher.scheduler.runCurrent()

        commands.handler = { _, _ -> throw CommandException("ConnectionError", "连接已断开") }
        gate.value = true
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals("连接类失败条目留队", 1, outbox.entries.value.size)
        assertTrue("连接类失败不弹错（降级态由状态条呈现）", errors.isEmpty())
        assertFalse(vm.uiState.value.responding)
        val firstCommandId = commands.passedCommandIds.single()

        commands.handler = { _, _ -> JSONObject().put("conversationId", "conv-butler-current") }
        // 断连→重连跨调度拍（生产网络事件必然分拍；同拍翻转会被 StateFlow 聚合吞掉）
        gate.value = false
        dispatcher.scheduler.runCurrent()
        gate.value = true
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(2, commands.calls.count { it.first == "chat.send" })
        assertEquals("重发必须复用同一 commandId（幂等键稳定）", firstCommandId, commands.passedCommandIds.last())
        assertEquals(0, outbox.entries.value.size)
    }

    @Test
    fun `桌面业务错误删条目并显式提示`() {
        // WHY（SPEC §4.4）：坏消息（桌面校验拒绝等）留在队列会无限重试、每次
        // 恢复都重弹——删条目 + 既有 failSend 路径提示，让用户自行修改重发。
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()
        val commands = FakeCommandSender { _, _ -> throw CommandException("ValidationError", "内容不合法") }
        val errors = mutableListOf<String>()
        val vm = newVm(commands = commands, onError = { errors += it }, outbox = outbox, commandReady = gate)

        vm.sendMessage("坏消息")
        dispatcher.scheduler.runCurrent()
        gate.value = true
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("业务错误条目删队（不无限重试坏消息）", 0, outbox.entries.value.size)
        assertEquals(listOf("内容不合法"), errors)
        assertFalse("气泡退出待发态", vm.uiState.value.messages.any { it.pending })
    }

    @Test
    fun `进程重建后待发态还原含本地占位会话`() {
        // WHY（SPEC 裁决 B 落盘恢复）：进程被杀/冷启动后待发消息须在对应会话
        //（含本地占位会话）恢复为待发态气泡——否则用户以为消息丢失而重发，产生重复。
        val gate = MutableStateFlow(false)
        val outbox = ChatOutbox()

        val vm1 = newVm(outbox = outbox, commandReady = gate)
        vm1.newConversation() // 本地占位会话（ack 空 → 桌面 id 不回填）
        dispatcher.scheduler.advanceUntilIdle()
        vm1.sendMessage("占位会话里的离线消息")
        vm1.selectRole("role-pm")
        vm1.sendMessage("角色会话的离线消息")
        dispatcher.scheduler.runCurrent()

        assertEquals(2, outbox.entries.value.size)
        assertNull("本地占位会话条目 conversationId=null（桌面新建回填）", outbox.entries.value[0].conversationId)
        assertEquals("conv-pm", outbox.entries.value[1].conversationId)

        // 模拟进程重建：同一待发箱 + 全新 VM（内存会话历史清零）
        val vm2 = newVm(outbox = outbox, commandReady = gate)
        dispatcher.scheduler.runCurrent()

        // 本地占位条目 → 新建本地会话承载待发气泡（落最新位，用户可切看）
        val pendingConv = vm2.uiState.value.conversations.first { c -> c.messages.any { it.pending } }
        assertEquals("占位会话里的离线消息", pendingConv.messages.single().text)
        assertTrue(pendingConv.messages.single().pending)
        vm2.selectConversation(pendingConv.id)
        assertTrue(vm2.uiState.value.messages.single().pending)

        // 角色会话条目 → 原会话恢复待发气泡（按 localMessageId 幂等去重）
        vm2.selectRole("role-pm")
        val pmMsg = vm2.uiState.value.messages.last { !it.fromButler }
        assertEquals("角色会话的离线消息", pmMsg.text)
        assertTrue(pmMsg.pending)
        assertEquals("恢复不消费队列（仍待 flush）", 2, outbox.entries.value.size)
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
        // 空光标守卫：工具/思考段无可见文本，不产生 text 为空的气泡项
        assertTrue(vm.uiState.value.messages.none { it.fromButler && it.text.isBlank() })

        // 文本 token 逐字渲染
        coordinator.onStreamToken(tokenJson(token = "你"))
        coordinator.onStreamToken(tokenJson(token = "好，"))
        dispatcher.scheduler.runCurrent()
        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("你好，", streaming.text)
        assertTrue(streaming.streaming)

        // done 收口（Error/SSE 收口形状：done+文本、无 phase）
        coordinator.onStreamToken(tokenJson(token = "我在。", done = true, phase = null))
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
    fun `思考token无可见文本不产生空气泡首个正文token逐字浮现`() {
        // WHY：空光标守卫（SPEC streaming-protocol §3）——生产时序 thinking token
        // （phase=thinking）先于 answering token 到达，思考期由 ThinkingBubble 承载；
        // 消息列表出现 text 为空的流式项即用户报告的「空流式光标」形态。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("早上好")
        dispatcher.scheduler.runCurrent()

        coordinator.onStreamToken(
            tokenJson(token = "先分析一下", thinking = true, phase = "thinking", statusText = "思考中..."),
        )
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.thinking)
        assertTrue(vm.uiState.value.messages.none { it.fromButler && it.text.isBlank() })

        coordinator.onStreamToken(tokenJson(token = "早上"))
        dispatcher.scheduler.runCurrent()
        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("早上", streaming.text)
        assertTrue(streaming.streaming)
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

    // ── 场景 3b：锚点渲染 id + 流式溯源实时暴露（M1'/M2'，FR-29/FR-1）──

    @Test
    fun `普通流式渲染id用ack锚点`() {
        // WHY（M2'）：渲染用合成 id 而快照用真实 id——快照落地时按 id 重建消息
        // 列表，气泡换 key 闪烁且溯源块被孤儿化；ack 的 assistantMessageId
        // 最早可得，全 null 段（普通流）统一用锚点 id。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        // 生产形状：正文 token messageId=null（agent_engine.rs BubbleSlot::First → None）
        coordinator.onStreamToken(tokenJson(token = "部分回复", messageId = null))
        dispatcher.scheduler.runCurrent()

        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("m-a-d", streaming.id)
        assertTrue(streaming.streaming)
    }

    @Test
    fun `流式中溯源挂载宿主消息气泡上方`() {
        // WHY（FR-29）：桌面流式中执行过程实时可见（挂锚点消息上方、默认展开）；
        // 修复前溯源只在 done 后挂合成 id——快照落地即孤儿化消失。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "部分回复", messageId = null))
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(
            tokenJson(
                phase = "process",
                messageId = "m-a-d",
                processEvent = """{"eventType":"tool","toolName":"memory_search","status":"completed","summary":"检索记忆库"}""",
            ),
        )
        dispatcher.scheduler.runCurrent()

        // 有宿主（末段可见正文）：trace 挂宿主 id（锚点）；不占独立条目
        assertEquals(1, vm.uiState.value.traceByMessageId["m-a-d"]?.size)
        assertNull(vm.uiState.value.streamingTrace)
    }

    @Test
    fun `流式中无宿主溯源独立条目done收口清理`() {
        // WHY（FR-29）：思考/工具期无可见正文（空光标守卫不产消息项），溯源须
        // 以独立条目实时暴露（桌面同形——无锚 streamingTraceBlocks 独立渲染，
        // 置 thinking 前）；done 收口无正文可挂即清理。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(
            tokenJson(
                phase = "process",
                messageId = "m-a-d",
                processEvent = """{"eventType":"narration","summary":"开始检索记忆"}""",
            ),
        )
        dispatcher.scheduler.runCurrent()

        // 无可见正文 → 无宿主 → streamingTrace 独立条目
        assertEquals(1, vm.uiState.value.streamingTrace?.size)
        // 空光标守卫：无正文不产空气泡（种子问候仍在，只断言无新增空段）
        assertTrue(vm.uiState.value.messages.none { it.fromButler && it.text.isBlank() })

        // done 收口：无宿主溯源清理（不残留进行态条目）
        coordinator.onStreamToken(tokenJson(done = true, phase = null, messageId = null))
        dispatcher.scheduler.runCurrent()
        assertNull(vm.uiState.value.streamingTrace)
    }

    @Test
    fun `委派两段流式渲染两气泡溯源挂末段`() {
        // WHY（FR-1，M2'）：首段 token messageId=null、唤醒帧/二段 Some(占位id)+
        // phase=answering——null→Some 切换即分段，两段气泡；溯源挂末段（真实
        // id）；混合段 null 首段不替换 id——锚点==二段 id，替换会撞 LazyColumn key。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("让产品经理看看")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "稍等，我让产品经理看一下。", messageId = null))
        dispatcher.scheduler.runCurrent()
        // 唤醒帧（answering + 空 token + 非 done + Some id）→ null→Some 分段
        coordinator.onStreamToken(tokenJson(token = "", messageId = "m-a-d"))
        coordinator.onStreamToken(
            tokenJson(
                phase = "process",
                messageId = "m-a-d",
                processEvent = """{"eventType":"narration","summary":"产品经理整理反馈"}""",
            ),
        )
        // 先渲染分段态：此刻末段（Some）尚无可见正文，溯源临时挂首段兜底 id
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "来自产品经理的反馈：可以排期。", messageId = "m-a-d"))
        dispatcher.scheduler.runCurrent()

        // 本轮两段 = 用户气泡之后的全部消息（种子问候不计入）
        val lastUserIdx = vm.uiState.value.messages.indexOfLast { !it.fromButler }
        val bubbles = vm.uiState.value.messages.subList(lastUserIdx + 1, vm.uiState.value.messages.size)
        assertEquals(2, bubbles.size)
        assertEquals("稍等，我让产品经理看一下。", bubbles[0].text)
        assertEquals("来自产品经理的反馈：可以排期。", bubbles[1].text)
        assertEquals("混合段保留各自 id：二段用真实（Some）id", "m-a-d", bubbles[1].id)
        assertNotEquals("null 首段不替换为锚点（防 key 撞车）", "m-a-d", bubbles[0].id)
        // 溯源挂末段（真实 id）；宿主切换清旧条目（首段承载期不留幽灵溯源）
        assertEquals(1, vm.uiState.value.traceByMessageId["m-a-d"]?.size)
        assertNull(vm.uiState.value.traceByMessageId[bubbles[0].id])

        // 收敛（FR-1 桌面契约）：桌面只把 final_text 落库到占位行——首段文本
        // 仅前缀剥离用、不落任何行；done 后含完成行的快照落地，两气泡塌缩
        // 为一段（镜像桌面），溯源锚点条目存活。
        coordinator.onStreamToken(tokenJson(token = "", done = true, phase = null, messageId = "m-a-d"))
        dispatcher.scheduler.runCurrent()
        val converged = fixtureSnapshot().let { snap ->
            snap.copy(
                conversations = snap.conversations.map { conv ->
                    if (conv.id == "conv-butler-current") {
                        conv.copy(
                            messages = conv.messages + listOf(
                                SnapshotMessage("m-user-d", "user", "让产品经理看看", "", true, "2026-09-11T01:00:00Z"),
                                SnapshotMessage("m-a-d", "assistant", "来自产品经理的反馈：可以排期。", "", true, "2026-09-11T01:00:01Z"),
                            ),
                        )
                    } else conv
                },
            )
        }
        coordinator.deliverSnapshot(converged, null)
        dispatcher.scheduler.runCurrent()

        val after = vm.uiState.value.messages
        assertEquals("收敛后仅剩落库二段（首段桌面不落库）", 1, after.count { it.id == "m-a-d" })
        assertTrue("首段气泡随快照塌缩消失（镜像桌面）", after.none { it.text == "稍等，我让产品经理看一下。" })
        assertEquals("溯源锚点条目跨快照存活（不孤儿化）", 1, vm.uiState.value.traceByMessageId["m-a-d"]?.size)
    }

    @Test
    fun `ack缺assistantMessageId时渲染回退合成id`() {
        // WHY（M2' 锚点链兜底）：旧桌面/兜底 ack 无 assistantMessageId——全 null
        // 段回退合成兜底 id（既有行为不变更），不得因锚点缺失产生空 id 或崩溃。
        val commands = FakeCommandSender { _, _ ->
            JSONObject("""{"conversationId":"conv-butler-current","userMessageId":"m-user-d"}""")
        }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "部分回复", messageId = null))
        dispatcher.scheduler.runCurrent()

        val streaming = vm.uiState.value.messages.last { it.fromButler }
        assertTrue("无锚回退合成兜底 id（stream-序号-段位）", streaming.id.matches(Regex("stream-\\d+-\\d+")))
        assertEquals("部分回复", streaming.text)
    }

    @Test
    fun `停止后流式溯源独立条目被清理`() {
        // WHY（M2'）：独立溯源条目是流式进行态——停止/超时/断连后残留即幽灵
        // 「执行过程」（随会话切换跟随用户）。done 路径已有清理，本地收口
        // （stopStreaming → finalizeStreamLocally）路径同契约。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        // 思考/工具期（无可见正文）：溯源以独立条目暴露
        coordinator.onStreamToken(
            tokenJson(
                phase = "process",
                messageId = "m-a-d",
                processEvent = """{"eventType":"narration","summary":"开始检索记忆"}""",
            ),
        )
        dispatcher.scheduler.runCurrent()
        assertEquals(1, vm.uiState.value.streamingTrace?.size)

        vm.stopStreaming()
        dispatcher.scheduler.runCurrent()
        assertNull("停止后独立条目清理（不留幽灵溯源）", vm.uiState.value.streamingTrace)
        assertFalse(vm.uiState.value.responding)
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
        coordinator.onStreamToken(tokenJson(done = true, phase = "done"))
        dispatcher.scheduler.runCurrent()

        val messages = vm.uiState.value.messages
        assertEquals(1, messages.count { it.id == "m-user-d" })
        assertEquals(1, messages.count { it.id == "m-a-d" })
        assertFalse(vm.uiState.value.responding)
    }

    @Test
    fun `done时陈旧暂存被丢弃本地文本保留`() {
        // WHY（M1'）：快照门在流结束时刻回放流中段陈旧快照（含空 assistant
        // 占位行）——onSnapshotReplaced 掐掉已定文本，气泡消失约 2s 后随写信号
        // 新快照回归（闪烁）。守卫：陈旧暂存丢弃，本地已定文本保留。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val coordinator = StreamCoordinator { snapshot, raw -> store.applySnapshot(snapshot, raw) }
        val vm = ChatViewModel(store, commands, coordinator)

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "完整回复", messageId = null))
        dispatcher.scheduler.runCurrent()

        // 流中段陈旧快照（占位行未完成、content 空）暂存
        val midStream = fixtureSnapshot().let { snap ->
            snap.copy(
                generatedAt = "2026-08-25T08:10:00Z",
                conversations = snap.conversations.map { conv ->
                    if (conv.id == "conv-butler-current") {
                        conv.copy(
                            messages = conv.messages + listOf(
                                SnapshotMessage("m-user-d", "user", "你好", "", false, "2026-08-25T08:10:00Z"),
                                SnapshotMessage("m-a-d", "assistant", "", "", false, "2026-08-25T08:10:01Z"),
                            ),
                        )
                    } else conv
                },
            )
        }
        coordinator.deliverSnapshot(midStream, null)
        dispatcher.scheduler.runCurrent()
        assertTrue("流式中段暂存不掐气泡", vm.uiState.value.responding)

        coordinator.onStreamToken(tokenJson(token = "", done = true, phase = null, messageId = "m-a-d"))
        dispatcher.scheduler.runCurrent()

        // 陈旧暂存被丢弃：本地已定文本保留（不因回放占位快照而消失）
        val finalMsg = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("完整回复", finalMsg.text)
        assertEquals("m-a-d", finalMsg.id)
        assertFalse(vm.uiState.value.responding)
    }

    @Test
    fun `done后含完成行快照落地溯源存活且消息id不变`() {
        // WHY（AC，M2'）：done 触发 flush——含完成行的暂存照常应用，
        // onSnapshotReplaced 按快照重建；锚点 id == 快照行 id → 气泡不换 key
        //（无闪烁）、溯源锚点条目保留（不孤儿化）。
        val store = SnapshotStore().apply { applySnapshot(fixtureSnapshot()) }
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val coordinator = StreamCoordinator { snapshot, raw -> store.applySnapshot(snapshot, raw) }
        val vm = ChatViewModel(store, commands, coordinator)

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "你好，", messageId = null))
        coordinator.onStreamToken(
            tokenJson(
                phase = "process",
                messageId = "m-a-d",
                processEvent = """{"eventType":"narration","summary":"检索记忆"}""",
            ),
        )
        dispatcher.scheduler.runCurrent()
        assertEquals(1, vm.uiState.value.traceByMessageId["m-a-d"]?.size)

        // 含完成行的快照暂存（守卫通过）→ done flush 直通应用
        coordinator.deliverSnapshot(convergedSnapshot(), null)
        dispatcher.scheduler.runCurrent()
        assertTrue(vm.uiState.value.responding)

        coordinator.onStreamToken(tokenJson(token = "我在。", done = true, phase = null, messageId = "m-a-d"))
        dispatcher.scheduler.runCurrent()

        val finalMsg = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("m-a-d", finalMsg.id)
        assertEquals("你好，我在。", finalMsg.text)
        assertEquals("trace 锚点条目存活（不孤儿化）", 1, vm.uiState.value.traceByMessageId["m-a-d"]?.size)
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
        coordinator.onStreamToken(tokenJson(conversationId = historyId, token = "。", done = true, messageId = "m-x", phase = null))
        dispatcher.scheduler.runCurrent()

        // 段落已写入历史会话：切过去即可见（不凭空消失）
        vm.selectConversation(historyId)
        assertTrue(vm.uiState.value.messages.any { it.text == "桌面侧回复。" })
    }

    @Test
    fun `流式中切走后done落库段落id用ack锚点`() {
        // WHY（M2'）：非查看会话的 done 落库 id 不得回退合成兜底——切回 +
        // 快照落地时合成 id 会 key 翻转闪烁、溯源孤儿化；ack 锚点跨「停止→
        // 迟到 token 重启流」窗口须保留（foldNew 同会话不覆写）。
        val commands = FakeCommandSender { _, _ -> chatSendAck() }
        val vm = newVm(commands = commands)
        val coordinator = vm.streamField()
        val historyId = vm.uiState.value.conversations.first { it.title == butlerHistoryTitle }.id

        vm.sendMessage("你好")
        dispatcher.scheduler.runCurrent()
        // 流式中切走（触发 stop 收口——迟到 token 经 foldNew 重启流）
        vm.selectConversation(historyId)
        coordinator.onStreamToken(tokenJson(token = "切走后回复", messageId = null))
        dispatcher.scheduler.runCurrent()
        coordinator.onStreamToken(tokenJson(token = "。", done = true, phase = null, messageId = "m-a-d"))
        dispatcher.scheduler.runCurrent()

        vm.selectConversation("conv-butler-current")
        val persisted = vm.uiState.value.messages.last { it.fromButler }
        assertEquals("落库段落 id = ack 锚点（快照落地 key 稳定）", "m-a-d", persisted.id)
        assertEquals("切走后回复。", persisted.text)
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
        coordinator.onStreamToken(tokenJson(token = "回复", done = true, phase = null))
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
