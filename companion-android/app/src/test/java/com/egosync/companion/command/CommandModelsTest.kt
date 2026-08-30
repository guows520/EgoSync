package com.egosync.companion.command

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 指令 envelope/ack/token 模型契约测试（Story 13.3 T9，AC:7）。
 *
 * WHY：envelope 是双端共同事实源（桌面 `models/companion_command.rs` serde
 * camelCase 产物）——任一字段名/键序漂移都会在真链路解码时爆炸，单元层
 * 用字节级断言钉死 wire 形状。
 */
class CommandModelsTest {

    // ── envelope 编码（wire 字节断言）──────────────────────────────

    @Test
    fun `envelope编码与桌面serde产物一致`() {
        val encoded = CommandEnvelope.encode("c-1", "chat.send", """{"content":"你好"}""")
        assertEquals(
            """{"schemaVersion":1,"commandId":"c-1","action":"chat.send","params":{"content":"你好"}}""",
            encoded,
        )
    }

    @Test
    fun `encode拒绝非法paramsJson`() {
        // WHY: 非法 params 产出 wire 上不可解码的 envelope——错误要到桌面
        // serde 拒绝后走完整个超时往返才暴露；本地快速失败才是诚实路径。
        for (bad in listOf("", "not json", "[1,2]", "null", "\"str\"")) {
            val e = runCatching { CommandEnvelope.encode("c-1", "chat.send", bad) }
                .exceptionOrNull() as CommandException
            assertEquals("ValidationError", e.variant)
        }
    }

    @Test
    fun `encode拒绝超限envelope`() {
        // WHY: 超限帧在 FrameCodec 编码层才被拒（pump 侧），发送方拿不到
        // 该异常只能等超时；app 层前置校验让发送方立即拿到显式错误。
        val bigParams = """{"content":"${"x".repeat(70_000)}"}"""
        val e = runCatching { CommandEnvelope.encode("c-1", "chat.send", bigParams) }
            .exceptionOrNull() as CommandException
        assertEquals("ValidationError", e.variant)
        assertTrue(e.message!!.contains("单帧上限"))
    }

    @Test
    fun `envelope转义特殊字符防注入破坏`() {
        val encoded = CommandEnvelope.encode("c\"2", "memory.list", "{}")
        // commandId 含引号必须转义——否则 envelope JSON 结构被撕裂
        assertTrue(encoded.contains("\"commandId\":\"c\\\"2\""))
        assertTrue(JSONObject(encoded).optString("commandId") == "c\"2")
    }

    @Test
    fun `buildParams跳过null保持标量类型`() {
        val params = CommandEnvelope.buildParams(
            listOf(
                "conversationId" to null as String?,
                "isCompleted" to true,
                "count" to 3,
            ),
        )
        val obj = JSONObject(params)
        // null 键不发：桌面 deny_unknown_fields 会拒绝多余键，但 null 值
        // 同样过不了 Option<String> 反序列化——键必须整体缺席
        assertTrue(!obj.has("conversationId"))
        assertEquals(true, obj.get("isCompleted"))
        assertEquals(3, obj.get("count"))
    }

    // ── ack 解析 ──────────────────────────────────────────────────

    @Test
    fun `成功ack解析取result`() {
        val ack = CommandAck.parse(
            """{"schemaVersion":1,"commandId":"c-9","ok":true,"result":{"conversationId":"conv-1"}}""",
        ) ?: error("合法 ack 不得解析失败")
        assertTrue(ack.ok)
        assertEquals("c-9", ack.commandId)
        assertEquals("conv-1", ack.result?.optString("conversationId"))
        assertEquals("conv-1", ack.resultOrThrow().optString("conversationId"))
    }

    @Test
    fun `失败ack解析携带AppError变体`() {
        val ack = CommandAck.parse(
            """{"schemaVersion":1,"commandId":"c-9","ok":false,"error":{"NotFound":"任务不存在"}}""",
        ) ?: error("合法 ack 不得解析失败")
        assertTrue(!ack.ok)
        val e = runCatching { ack.resultOrThrow() }.exceptionOrNull() as CommandException
        // 变体名单键 map（桌面 AppError serde 形状），UI 据此分类提示
        assertEquals("NotFound", e.variant)
        assertEquals("任务不存在", e.message)
    }

    @Test
    fun `非法JSON的ack解析为null不崩溃`() {
        assertNull(CommandAck.parse("不是 json"))
        assertNull(CommandAck.parse(""))
    }

    @Test
    fun `错误形状降级为ValidationError`() {
        // 非单键 map 的 error（防御：桌面契约破坏时不抛裸异常）
        val e = CommandException.fromErrorJson(JSONObject("""{"a":1}"""))
        assertEquals("ValidationError", e.variant)
    }

    @Test
    fun `多键error降级而非任取首键`() {
        // WHY: 多键且值均为 String 是契约破坏形态——迭代顺序不定，任取首键
        // 会把错误分类建立在不确定值上（UI 提示建立在随机变体）。必须降级。
        val e = CommandException.fromErrorJson(JSONObject("""{"NotFound":"a","DbError":"b"}"""))
        assertEquals("ValidationError", e.variant)
        assertEquals("未知错误形状", e.message)
    }

    @Test
    fun `成功ack缺result显式报错不伪造空结果`() {
        // WHY: 桌面成功 ack 恒带 result；缺席即契约破坏——返回空 JSONObject
        // 会把「回执缺字段」静默抹平成正常成功（调用方无从区分）。
        val ack = CommandAck.parse(
            """{"schemaVersion":1,"commandId":"c-9","ok":true}""",
        ) ?: error("合法 ack 不得解析失败")
        val e = runCatching { ack.resultOrThrow() }.exceptionOrNull() as CommandException
        assertEquals("ValidationError", e.variant)
        assertTrue(e.message!!.contains("result"))
    }

    @Test
    fun `schemaVersion漂移的ack拒收`() {
        // WHY: 不校验版本时未来 v2 载荷会被静默误解析（字段形状不同）而非
        // 显式拒收——漂移必须爆炸是双端契约纪律。
        assertNull(CommandAck.parse("""{"schemaVersion":2,"commandId":"c","ok":true,"result":{}}"""))
    }

    // ── STREAM_TOKEN 事件解析 ──────────────────────────────────────

    @Test
    fun `流事件解析全字段`() {
        val event = StreamEvent.parse(
            """{"conversationId":"conv-1","token":"你","done":false,"thinking":false,
                "messageId":"m-1","phase":"tool","statusText":"检索记忆","toolName":"memory_search",
                "processEvent":{"eventType":"tool","toolName":"memory_search","status":"running",
                "summary":"检索记忆库","rawJson":null,"workingDirectory":null}}""".replace("\n", ""),
        ) ?: error("合法事件不得解析失败")
        assertEquals("conv-1", event.conversationId)
        assertEquals("你", event.token)
        assertEquals("m-1", event.messageId)
        assertEquals("tool", event.phase)
        assertEquals("检索记忆", event.statusText)
        assertEquals("memory_search", event.toolName)
        assertEquals("tool", event.processEvent?.eventType)
        assertEquals("running", event.processEvent?.status)
    }

    @Test
    fun `缺conversationId或非法JSON返回null`() {
        // conversationId 是流态聚合主键——缺席即无法归属会话，跳帧优于错渲染
        assertNull(StreamEvent.parse("""{"token":"你"}"""))
        assertNull(StreamEvent.parse("坏 json"))
    }
}
