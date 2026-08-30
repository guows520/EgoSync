package com.egosync.companion.command

import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.ExecutionTraceBlock
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

    private fun token(
        conversationId: String = "conv-1",
        token: String = "",
        done: Boolean = false,
        thinking: Boolean = false,
        messageId: String? = "m-1",
        phase: String? = null,
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

        coordinator.onStreamToken(token(done = true, messageId = "m-1"))
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
        coordinator.onStreamToken(token(conversationId = "conv-a", token = "", done = true, messageId = "m-a"))
        assertTrue(coordinator.state.value!!.done)

        // 桌面在 conv-b 发起新流：首 token 必须建立 conv-b 的流
        coordinator.onStreamToken(token(conversationId = "conv-b", token = "新轮", messageId = "m-b"))
        val s = coordinator.state.value!!
        assertEquals("conv-b", s.conversationId)
        assertEquals("新轮", s.segments.single().text)
        assertTrue(!s.done)
    }

    // ── 快照延后门（Dev Notes §4）────────────────────────────────

    @Test
    fun `活跃流期间快照暂存只留最新流结束补应用`() {
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")

        coordinator.streamStarting("conv-1")
        coordinator.deliverSnapshot(snapshot(1), null)
        coordinator.deliverSnapshot(snapshot(2), null)
        // 流式中段 STATE_DELTA 必然到达（chat 写入 2s debounce 重建）——
        // 立即应用会触发 onSnapshotReplaced 掐掉流式气泡
        assertTrue(applied.isEmpty())

        coordinator.streamEnded("conv-1")
        // 流结束立即补应用，且只补最新一条（暂存槽单条）
        assertEquals(listOf(2), applied)
    }

    @Test
    fun `无活跃流快照直接应用`() {
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.deliverSnapshot(snapshot(1), null)
        // 门只挡流式期间；常态直通
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
        coordinator.streamStarting("conv-b")

        // conv-a 流的迟到收口信号（当前 conv-b 流正活跃，暂存未 flush）
        coordinator.deliverSnapshot(snapshot(1), null)
        assertTrue(applied.isEmpty())
        coordinator.streamEnded("conv-a")
        assertTrue("无关收口不得触发暂存应用", applied.isEmpty())
        assertTrue(!coordinator.state.value!!.done)

        // conv-b 自己的收口才 flush
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
        coordinator.streamStarting("conv-a")
        coordinator.deliverSnapshot(snapshot(1), null)
        assertTrue(applied.isEmpty())

        // 用户切看 conv-b：暂存 S1 立即应用；此后快照直通（S2 覆盖 S1，顺序正确）
        coordinator.onViewedConversation("conv-b")
        assertEquals(listOf(1), applied)
        coordinator.deliverSnapshot(snapshot(2), null)
        assertEquals(listOf(1, 2), applied)

        // 流结束再 flush 不得回放旧暂存（已清空）
        coordinator.streamEnded("conv-a")
        assertEquals(listOf(1, 2), applied)
    }

    @Test
    fun `reset清空流态并flush暂存`() {
        val applied = mutableListOf<Int>()
        val coordinator = StreamCoordinator { s, _ -> applied += s.generatedAt.toInt() }
        coordinator.onViewedConversation("conv-1")
        coordinator.streamStarting("conv-1")
        coordinator.deliverSnapshot(snapshot(1), null)

        coordinator.reset()
        assertNull(coordinator.state.value)
        // 断连清态：暂存快照不丢（会话重建后收敛）
        assertEquals(listOf(1), applied)
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
}
