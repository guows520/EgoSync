package com.egosync.companion.command

import com.egosync.companion.sync.ExecutionTraceBlock
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 流式黄金契约测试（SPEC-companion-connection-chat-ux / streaming-protocol.md §4）。
 *
 * WHY：桌面 llm:stream 的 StreamPayload 形状是与本端折叠状态机的跨端契约，
 * 历史缺陷（空光标后整段回填）正源于两端判据漂移——桌面生产正文 token 一律
 * phase="answering"（agent_engine.rs emit_stream_token），而手机只认 phase=null。
 * 本测试用「桌面真实发射形状」的 fixture（resources/streaming/，来源锚定见其
 * README.md）驱动 onStreamToken，锁定归类语义；桌面侧由 models/chat.rs
 * stream_phase_domain_is_locked 测试锚定同一值域（用户裁决 A+B 双轨）。
 * fixture 更新必须与桌面发射点同步，禁止凭空编造形状。
 */
class StreamGoldenContractTest {

    private fun feed(name: String): List<JSONObject> =
        javaClass.classLoader!!.getResourceAsStream("streaming/$name")!!.bufferedReader()
            .readLines()
            .filter { it.isNotBlank() }
            .map { JSONObject(it) }

    private fun newCoordinator(): StreamCoordinator = StreamCoordinator { _, _ -> }

    private fun StreamCoordinator.push(events: List<JSONObject>) {
        events.forEach { onStreamToken(it.toString()) }
    }

    @Test
    fun `01 answering多token逐字累加`() {
        val coordinator = newCoordinator()
        val events = feed("01-answering-multi-token.jsonl")
        assertEquals(4, events.size)

        // 逐 token：首个 token 落段即有可见文本（不是空光标等整段回填）
        coordinator.onStreamToken(events[0].toString())
        assertEquals("早", coordinator.state.value!!.segments.single().text)

        coordinator.push(events.drop(1))
        val s = coordinator.state.value!!
        assertEquals("早上好，今天也要加油。", s.segments.single().text)
        assertFalse(s.thinking)
        assertFalse(s.done)
    }

    @Test
    fun `02 thinking token不落正文段`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("02-thinking-token.jsonl"))
        val s = coordinator.state.value!!
        assertTrue(s.thinking)
        assertEquals("", s.segments.single().text)
    }

    @Test
    fun `03 thinking切换answering正文落段且思考态关闭`() {
        // 用户报告缺陷的真实时序：思考 token → answering token。
        // 修复前：answering token 被判非正文 → 段保持空 → 空光标。
        val coordinator = newCoordinator()
        coordinator.push(feed("03-thinking-then-answering.jsonl"))
        val s = coordinator.state.value!!
        assertEquals("早上好。", s.segments.single().text)
        assertFalse(s.thinking)
    }

    @Test
    fun `04 tool阶段产生工具行不产生正文`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("04-tool.jsonl"))
        val s = coordinator.state.value!!
        assertEquals("检索记忆库", s.toolTitle)
        assertEquals("", s.segments.single().text)
    }

    @Test
    fun `05 process三型事件累计溯源块不进正文`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("05-process.jsonl"))
        val s = coordinator.state.value!!
        assertEquals(3, s.traceBlocks.size)
        assertEquals("", s.segments.single().text)
        assertTrue(s.traceBlocks[0] is ExecutionTraceBlock.Thinking)
        assertTrue(s.traceBlocks[1] is ExecutionTraceBlock.Narration)
        assertTrue(s.traceBlocks[2] is ExecutionTraceBlock.Action)
    }

    @Test
    fun `06 done帧带phase收口不追加正文`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("06-done-with-phase.jsonl"))
        val s = coordinator.state.value!!
        assertTrue(s.done)
        assertEquals("好的", s.segments.single().text)
    }

    @Test
    fun `07 历史SSE无phase路径仍按正文累加并收口`() {
        // WHY：桌面 SseEvent::Text/Error 路径不带 phase——answering 修复
        // 不得切断历史路径（双形状并存是跨端兼容的硬约束）。
        val coordinator = newCoordinator()
        coordinator.push(feed("07-done-legacy-null.jsonl"))
        val s = coordinator.state.value!!
        assertTrue(s.done)
        assertEquals("旧的", s.segments.single().text)
    }

    @Test
    fun `08 messageId切换双段且前段sealed`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("08-multi-message-id.jsonl"))
        val s = coordinator.state.value!!
        assertTrue(s.done)
        assertEquals(2, s.segments.size)
        assertTrue(s.segments[0].sealed)
        assertEquals("稍等，我让产品经理看一下。", s.segments[0].text)
        assertEquals("来自产品经理的反馈：可以排期。", s.segments[1].text)
    }

    @Test
    fun `09 会话切换后新流归属事件自身会话`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("09-multi-conversation.jsonl"))
        val s = coordinator.state.value!!
        assertEquals("conv-b", s.conversationId)
        assertEquals("新轮", s.segments.single().text)
        assertFalse(s.done)
    }

    @Test
    fun `10 无phase字段的历史形状正文兼容`() {
        val coordinator = newCoordinator()
        coordinator.push(feed("10-phase-null-compat.jsonl"))
        assertEquals("历史形状仍按正文", coordinator.state.value!!.segments.single().text)
    }
}
