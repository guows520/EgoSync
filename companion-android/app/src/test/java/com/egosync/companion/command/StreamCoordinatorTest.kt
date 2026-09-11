package com.egosync.companion.command

import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.ExecutionTraceBlock
import com.egosync.companion.sync.SnapshotConversation
import com.egosync.companion.sync.SnapshotMessage
import com.egosync.companion.sync.ToolStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 流式聚合状态机契约测试（Story 13.3 T9，AC:3/7）。
 *
 * WHY：token 折叠语义镜像桌面 ChatStream.tsx（thinking→文本→工具行→溯源→done），
 * 任何漂移都会让手机端流式气泡与桌面呈现不一致；快照延后门（Dev Notes §4）是
 * 「流式中段 STATE_DELTA 不掐气泡」的唯一防线。
 */
class StreamCoordinatorTest {

    private fun snapshot(id: Int): DesktopSnapshot = SnapshotFixture.snapshot(id)

    private fun snapshotWithConversation(
        id: Int,
        conversationId: String = "conv-1",
        assistantMessageId: String = "m-a-1",
        assistantComplete: Boolean = true,
        assistantContent: String = "已完成回复",
    ): DesktopSnapshot = SnapshotFixture.snapshotWithConversation(
        id, conversationId, assistantMessageId, assistantComplete, assistantContent,
    )

    private fun token(
        conversationId: String = "conv-1",
        token: String = "",
        done: Boolean = false,
        thinking: Boolean = false,
        messageId: String? = "m-1",
        // 生产正文 token 形状（桌面 agent_engine.rs emit_stream_token 硬编码
        // phase="answering"）；历史 SSE 路径/收口帧按用例显式传 null
        phase: String? = "answering",
        statusText: String? = null,
        toolName: String? = null,
        processEvent: String? = null,
    ): String {
        val json = StringBuilder("""{"conversationId":"$conversationId","done":$done,"thinking":$thinking""")
        messageId?.let { json.append(""", "messageId":"$it"""") }
        if (token.isNotEmpty()) json.append(""", "token":"$token"""")
        phase?.let { json.append(""", "phase":"$it"""") }
        statusText?.let { json.append(""", "statusText":"$it"""") }
        toolName?.let { json.append(""", "toolName":"$it"""") }
        processEvent?.let { json.append(""", "processEvent":$it""") }
        json.append("}")
        return json.toString()
    }

    // ── 折叠状态机 ────────────────────────────────────────────────

    @Test
    fun `思考态到文本累加到done收口`() {
        val appliedSnapshots = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> appliedSnapshots += (s.generatedAt.toInt()) }

        coordinator.streamStarting("conv-1")
        assertTrue(coordinator.state.value!!.thinking)

        coordinator.onStreamToken(token(token = "你", messageId = "m-1"))
        coordinator.onStreamToken(token(token = "好", messageId = "m-1"))
        val s = coordinator.state.value!!
        // 文本 token 逐字累加进活跃段
        assertEquals("你好", s.segments.single().text)
        assertTrue(!s.thinking)
        assertTrue(!s.done)

        coordinator.onStreamToken(token(done = true, messageId = "m-1", phase = null))
        val done = coordinator.state.value!!
        assertTrue(done.done)
        assertEquals("你好", done.segments.single().text)
    }

    @Test
    fun `messageId切换开新段且上一段sealed`() {
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(token(token = "稍等，我让产品经理看一下。", messageId = "m-1"))
        coordinator.onStreamToken(token(token = "来自产品经理的反馈。", messageId = "m-2"))

        val s = coordinator.state.value!!
        // 两段委派语义（桌面 streamBubbles 分桶）：管家声明段 + 角色反馈段
        assertEquals(2, s.segments.size)
        assertTrue(s.segments[0].sealed)
        assertEquals("稍等，我让产品经理看一下。", s.segments[0].text)
        assertEquals("来自产品经理的反馈。", s.segments[1].text)
        assertTrue(!s.segments[1].sealed)
    }

    @Test
    fun `工具阶段状态行与processEvent溯源`() {
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(
            token(
                phase = "tool",
                statusText = "检索记忆库",
                toolName = "memory_search",
                messageId = "m-1",
            ),
        )
        var s = coordinator.state.value!!
        assertEquals("检索记忆库", s.toolTitle)

        coordinator.onStreamToken(
            token(
                phase = "process",
                messageId = "m-1",
                processEvent = """{"eventType":"tool","toolName":"memory_search","status":"completed","summary":"检索记忆库"}""",
            ),
        )
        s = coordinator.state.value!!
        // processEvent 不进文本段，累计为溯源块
        assertEquals(1, s.traceBlocks.size)
        val action = s.traceBlocks.single() as ExecutionTraceBlock.Action
        assertEquals(ToolStatus.COMPLETED, action.status)
        assertEquals("检索记忆库", action.title)
        assertEquals("", s.segments.single().text)
    }

    @Test
    fun `不同会话的流到达即重置聚合`() {
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-a")
        coordinator.onStreamToken(token(conversationId = "conv-b", token = "别的会话", messageId = "m-b"))
        // 防御：跨会话残留必须重置（桌面串行不并发，但手机查看会话可能已切走）
        assertEquals("conv-b", coordinator.state.value!!.conversationId)
        assertEquals("别的会话", coordinator.state.value!!.segments.single().text)
    }

    @Test
    fun `done残留后新流归属事件自身会话`() {
        // WHY: 曾沿用 cur?.conversationId 把新流挂到已 done 的旧会话上——
        // 快照门错开（旧会话被门禁、新会话不门禁）、渲染错挂会话。归属键
        // 必须是事件自身 conversationId（Dev Notes §2 路由键）。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-a")
        coordinator.onStreamToken(token(conversationId = "conv-a", token = "首轮", messageId = "m-a"))
        coordinator.onStreamToken(token(conversationId = "conv-a", token = "", done = true, messageId = "m-a", phase = null))
        assertTrue(coordinator.state.value!!.done)

        // 桌面在 conv-b 发起新流：首 token 必须建立 conv-b 的流
        coordinator.onStreamToken(token(conversationId = "conv-b", token = "新轮", messageId = "m-b"))
        val s = coordinator.state.value!!
        assertEquals("conv-b", s.conversationId)
        assertEquals("新轮", s.segments.single().text)
        assertTrue(!s.done)
    }

    @Test
    fun `phase为null的历史SSE形状仍按正文累加`() {
        // WHY：桌面 SseEvent::Text/Error 路径不带 phase（serde skip 后无字段）——
        // answering 兼容不得切断历史路径的正文累加（跨端双形状契约，缺一即整段回填回归）。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(token(token = "历史", messageId = "m-1", phase = null))
        coordinator.onStreamToken(token(token = "形状", messageId = "m-1", phase = null))
        assertEquals("历史形状", coordinator.state.value!!.segments.single().text)
    }

    @Test
    fun `未知phase值保守忽略不误入正文也不扰动思考态`() {
        // WHY：桌面未来扩展 phase 值时，旧版手机必须 fail-safe：不累加文本
        // （宁缺勿错显）、不误关思考态——否则新桌面一上线旧手机立刻错乱。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(token(token = "来自未来", messageId = "m-1", phase = "future_phase"))
        val s = coordinator.state.value!!
        assertEquals("", s.segments.single().text)
        assertTrue(s.thinking)
    }

    @Test
    fun `thinking段切换到answering正文落段且思考态关闭`() {
        // WHY：生产时序是 thinking token（phase=thinking）→ answering token——
        // 判定函数必须让正文在思考后正确落段（用户报告的空光标即此断点）。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(token(token = "先想想", thinking = true, phase = "thinking", statusText = "思考中..."))
        var s = coordinator.state.value!!
        assertTrue(s.thinking)
        assertEquals("", s.segments.single().text)

        coordinator.onStreamToken(token(token = "答案", messageId = "m-1"))
        s = coordinator.state.value!!
        assertEquals("答案", s.segments.single().text)
        assertTrue(!s.thinking)
    }

    // ── 快照延后门（Dev Notes §4 + M1' 新鲜度守卫）───────────────

    @Test
    fun `活跃流期间快照暂存只留最新流结束补应用`() {
        // WHY：流式中段 STATE_DELTA 必然到达（chat 写入 2s debounce 重建）——
        // 立即应用会触发 onSnapshotReplaced 掐掉流式气泡；流结束 flush 补应用
        // 且只补最新一条（暂存槽单条）。新契约下暂存须含本轮完成回复（守卫
        // 按锚点行校验）才放行——夹具带会话数据。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")

        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.deliverSnapshot(snapshotWithConversation(1), null)
        coordinator.deliverSnapshot(snapshotWithConversation(2), null)
        assertTrue(applied.isEmpty())

        coordinator.streamEnded("conv-1")
        assertEquals(listOf(2), applied)
    }

    @Test
    fun `无活跃流快照直接应用`() {
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.deliverSnapshot(snapshot(1), null)
        // 门只挡流式期间；常态直通（守卫仅在 flush 暂存路径生效）
        assertEquals(listOf(1), applied)
    }

    @Test
    fun `无关会话的streamEnded不击穿快照门`() {
        // WHY: streamEnded 无条件 flush 会让迟到/无关会话的收口信号立即
        // 应用暂存快照——onSnapshotReplaced 掐掉另一条正在流式的气泡，
        // 恰是快照门要防的事。flush 仅在无活跃流时允许。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-b")
        coordinator.streamStarting("conv-b", "m-b-1")

        // conv-a 流的迟到收口信号（当前 conv-b 流正活跃，暂存未 flush）
        coordinator.deliverSnapshot(snapshotWithConversation(1, "conv-b", "m-b-1"), null)
        assertTrue(applied.isEmpty())
        coordinator.streamEnded("conv-a")
        assertTrue("无关收口不得触发暂存应用", applied.isEmpty())
        assertTrue(!coordinator.state.value!!.done)

        // conv-b 自己的收口才 flush（守卫校验 conv-b 锚点完成行通过）
        coordinator.streamEnded("conv-b")
        assertEquals(listOf(1), applied)
    }

    @Test
    fun `切换查看会话时flush暂存防陈旧回放`() {
        // WHY: 切走查看会话后快照门对流会话失效（后续快照直通应用）——
        // 旧暂存若滞留到流结束才 flush，会覆盖更新的已应用快照（UI 回退）。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-a")
        coordinator.streamStarting("conv-a", "m-a-1")
        coordinator.deliverSnapshot(snapshotWithConversation(1, "conv-a", "m-a-1"), null)
        assertTrue(applied.isEmpty())

        // 用户切看 conv-b：暂存 S1 立即应用（含完成行，守卫通过）；此后快照直通（S2 覆盖 S1，顺序正确）
        coordinator.onViewedConversation("conv-b")
        assertEquals(listOf(1), applied)
        coordinator.deliverSnapshot(snapshotWithConversation(2, "conv-a", "m-a-1"), null)
        assertEquals(listOf(1, 2), applied)

        // 流结束再 flush 不得回放旧暂存（已清空）
        coordinator.streamEnded("conv-a")
        assertEquals(listOf(1, 2), applied)
    }

    @Test
    fun `reset清空流态并flush暂存`() {
        // WHY: 断连清态须先按本回合上下文 flush 再清态——守卫需读回合锚点/会话；
        // 含完成行的暂存照常应用（会话重建后收敛）。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.deliverSnapshot(snapshotWithConversation(1), null)

        coordinator.reset()
        assertNull(coordinator.state.value)
        // 含完成行的暂存不丢（陈旧暂存由守卫丢弃——见下方 M1' 用例）
        assertEquals(listOf(1), applied)
    }

    @Test
    fun `断连reset时陈旧暂存被丢弃`() {
        // WHY：断连瞬间暂存必为流中段快照（占位行未完成）——回放会把本地
        // 已定文本掐成空气泡；守卫在 reset 的 flush 路径同样生效（丢弃优于
        // 定时回放，重连后新快照收敛）。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.onStreamToken(token(token = "部分", messageId = null))
        coordinator.deliverSnapshot(
            snapshotWithConversation(1, assistantComplete = false, assistantContent = ""),
            null,
        )
        assertTrue(applied.isEmpty())

        coordinator.reset()
        assertNull(coordinator.state.value)
        assertTrue("陈旧暂存被丢弃，不回放占位空气泡", applied.isEmpty())
    }

    @Test
    fun `流式中切走查看会话时陈旧暂存被丢弃`() {
        // WHY：切走瞬间的 flush 与流结束 flush 同守卫——流仍在进行，暂存必为
        // 流中段；回放会掐掉该流已渲染文本（切回时凭空消失）。丢弃后新查看
        // 会话的快照直通，流会话由 done 后新快照收敛。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.onStreamToken(token(token = "部分", messageId = null))
        coordinator.deliverSnapshot(
            snapshotWithConversation(1, assistantComplete = false, assistantContent = ""),
            null,
        )
        assertTrue(applied.isEmpty())

        coordinator.onViewedConversation("conv-2")
        assertTrue("切走时陈旧暂存被丢弃", applied.isEmpty())
    }

    // ── M1'：flush 新鲜度守卫 ────────────────────────────────────

    @Test
    fun `陈旧暂存被丢弃不回放流中段快照`() {
        // WHY（M1'）：流中段 STATE_DELTA（占位行 isComplete=false、content 空）
        // 在 done 时刻 flush 回放会触发 onSnapshotReplaced 掐掉已定文本——气泡
        // 消失约 2s 后随写信号新快照回归（闪烁）。守卫：锚点消息未完成即丢弃；
        // done 写信号触发的新快照约 2s 后直通应用。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1", "m-a-1")
        // 生产形状：首段 token 无 messageId（锚点仅来自 ack）
        coordinator.onStreamToken(token(token = "部分回复", messageId = null))
        // 流中段快照：占位行未完成、content 空
        coordinator.deliverSnapshot(
            snapshotWithConversation(1, assistantComplete = false, assistantContent = ""),
            null,
        )
        assertTrue(applied.isEmpty())

        coordinator.streamEnded("conv-1")
        assertTrue("陈旧暂存被丢弃（不回放占位空气泡）", applied.isEmpty())

        // done 后含完成行的新快照：流已 done → 直通应用（收敛）
        coordinator.deliverSnapshot(snapshotWithConversation(2), null)
        assertEquals(listOf(2), applied)
    }

    @Test
    fun `暂存已含本轮完成回复照常应用`() {
        // WHY：网络重排/延迟下完整快照可先于 done 帧处理而暂存——此时暂存即
        // 收敛目标，照常应用（守卫不得误杀新鲜快照）。
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.onStreamToken(token(token = "完整回复", messageId = null))

        coordinator.deliverSnapshot(snapshotWithConversation(1), null)
        assertTrue(applied.isEmpty())

        coordinator.streamEnded("conv-1")
        assertEquals(listOf(1), applied)
    }

    @Test
    fun `无锚回退会话末条assistant完成才放行`() {
        // WHY：旧桌面/兜底路径 ack 无 assistantMessageId——守卫回退校验会话
        // 末条 assistant 行：末条为占位（未完成）即陈旧丢弃（防无锚路径回放
        // 空气泡），完成且非空才放行。
        val applied = mutableListOf<Int>()
        val fresh = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        fresh.onViewedConversation("conv-1")
        fresh.streamStarting("conv-1") // 无锚（旧桌面 ack 形状）
        fresh.onStreamToken(token(token = "回复", messageId = null))
        fresh.deliverSnapshot(snapshotWithConversation(1, assistantMessageId = "m-last"), null)
        fresh.streamEnded("conv-1")
        assertEquals("末条完成行 → 照常应用", listOf(1), applied)

        val staleApplied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> staleApplied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1")
        coordinator.onStreamToken(token(token = "回复", messageId = null))
        coordinator.deliverSnapshot(
            snapshotWithConversation(1, assistantMessageId = "m-last", assistantComplete = false, assistantContent = ""),
            null,
        )
        coordinator.streamEnded("conv-1")
        assertTrue("末条占位行 → 陈旧丢弃", staleApplied.isEmpty())
    }

    // ── M2'：委派分段条件放宽（FR-1 恢复）───────────────────────

    @Test
    fun `唤醒帧null到Some分段两段且首段sealed`() {
        // WHY（M2'）：委派两段（FR-1）——桌面首段 token messageId=null、唤醒帧/
        // 二段 messageId=Some(占位id)+phase=answering（agent_engine.rs:3034/3047）；
        // 旧分段条件要求活跃段已有 messageId，null 活跃段永不切段——两段气泡
        // 退化成一段。null→Some 切换即分段。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.onStreamToken(token(token = "稍等，我让产品经理看一下。", messageId = null))
        // 唤醒帧：answering + 空 token + 非 done + Some id
        coordinator.onStreamToken(token(token = "", messageId = "m-a-1"))
        coordinator.onStreamToken(token(token = "来自产品经理的反馈。", messageId = "m-a-1"))

        val s = coordinator.state.value!!
        assertEquals(2, s.segments.size)
        assertTrue(s.segments[0].sealed)
        assertEquals("稍等，我让产品经理看一下。", s.segments[0].text)
        assertEquals("来自产品经理的反馈。", s.segments[1].text)
        assertEquals("m-a-1", s.segments[1].messageId)
        assertEquals("锚点来自 ack（唤醒帧不再捕获——spec 矩阵行 8：仅 done 帧兜底）", "m-a-1", s.roundAssistantId)
    }

    @Test
    fun `process与tool帧携带Some id不误分段`() {
        // WHY：process/tool 帧也带 Some 占位 id——仅按 messageId!=null 放宽分段
        // 会把普通流的工具期误切两气泡；仅 answering 帧允许 null→Some 切换。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1", "m-a-1")
        coordinator.onStreamToken(token(token = "部分", messageId = null))
        coordinator.onStreamToken(
            token(
                phase = "process",
                messageId = "m-a-1",
                processEvent = """{"eventType":"tool","toolName":"memory_search","status":"completed","summary":"检索记忆库"}""",
            ),
        )
        coordinator.onStreamToken(
            token(phase = "tool", messageId = "m-a-1", statusText = "正在使用工具...", toolName = "bash"),
        )

        val s = coordinator.state.value!!
        assertEquals("工具期不切段（单段）", 1, s.segments.size)
        assertEquals("部分", s.segments.single().text)
    }

    @Test
    fun `done帧归入活跃段不分段且Some id捕获为锚点`() {
        // WHY：done 收口帧归入活跃段（不分段）；其 Some id 兜底捕获为锚点
        //（无 ack 路径仅 done 落库受益——落库 id 与快照行对齐）。历史 SSE 收口
        // 形状（phase=null、isAnswerText 判真）由 !done 判据排除分段。
        val coordinator = StreamCoordinator { _, _ -> }
        coordinator.streamStarting("conv-1") // 无锚（旧桌面/兜底路径）
        coordinator.onStreamToken(token(token = "回复", messageId = null))
        coordinator.onStreamToken(token(token = "", done = true, messageId = "m-a-1", phase = null))

        val s = coordinator.state.value!!
        assertTrue(s.done)
        assertEquals(1, s.segments.size)
        assertEquals("回复", s.segments.single().text)
        assertEquals("m-a-1", s.roundAssistantId)
    }
}

/** 测试夹具快照（最小可用形状，generatedAt 携带区分标记）。 */
internal object SnapshotFixture {
    fun snapshot(id: Int): DesktopSnapshot =
        DesktopSnapshot(
            schemaVersion = 1,
            generatedAt = id.toString().padStart(4, '0'),
            dataCutoffAt = null,
            truncated = false,
            truncatedDomains = emptyList(),
            roles = emptyList(),
            tasks = emptyList(),
            dashboard = com.egosync.companion.sync.SnapshotDashboard(
                statuses = emptyList(),
                metrics = com.egosync.companion.sync.SnapshotMetrics(0, 0, 0, 0, "0"),
            ),
            conversations = emptyList(),
            notifications = emptyList(),
            briefings = emptyList(),
            weeklyReviews = emptyList(),
        )

    /**
     * 含会话与 assistant 行的夹具（M1' 守卫按会话/锚点行校验新鲜度——空会话
     * 夹具在新契约下一律判陈旧）：完成行=新鲜暂存放行；占位行（未完成/空
     * content）=流中段陈旧暂存。
     */
    fun snapshotWithConversation(
        id: Int,
        conversationId: String = "conv-1",
        assistantMessageId: String = "m-a-1",
        assistantComplete: Boolean = true,
        assistantContent: String = "已完成回复",
    ): DesktopSnapshot = snapshot(id).copy(
        conversations = listOf(
            SnapshotConversation(
                id = conversationId,
                roleId = null,
                title = "会话",
                updatedAt = "2026-09-11T00:00:00Z",
                messages = listOf(
                    SnapshotMessage(
                        id = assistantMessageId,
                        role = "assistant",
                        content = assistantContent,
                        thinkingContent = "",
                        isComplete = assistantComplete,
                        createdAt = "2026-09-11T00:00:01Z",
                    ),
                ),
            ),
        ),
    )
}
