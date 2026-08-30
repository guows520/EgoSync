package com.egosync.companion.command

import org.json.JSONException
import org.json.JSONObject

/**
 * 指令 envelope schema（与桌面 `models/companion_command.rs` 一一对应）。
 *
 * 走 app 层 JSON 塞 `Frame.Command.data`（对协议层 opaque），不 bump
 * PROTOCOL_VERSION（13.1 裁决 1）。字段 camelCase 双端共同事实源。
 */
object CommandEnvelope {
    const val SCHEMA_VERSION = 1

    /** 单帧明文上限（镜像桌面 COMMAND_DATA_MAX_BYTES = MAX_PLAINTEXT_LEN = 65535-16）。 */
    const val MAX_ENVELOPE_BYTES = 65519

    /**
     * 编码 envelope JSON：`{"schemaVersion":1,"commandId":"...","action":"...","params":{...}}`。
     * [paramsJson] 必须是合法 JSON 对象字面量（调用方用 [buildParams] 拼装）。
     *
     * 整改：编码前自校验——paramsJson 非法或 envelope 超单帧上限立即抛
     * [CommandException]，本地快速失败；否则帧要等桌面 serde 拒绝后走完
     * 整个超时往返才暴露（FrameCodec 编码层兜底仅在 pump 侧，发送方拿不到
     * 该异常）。
     */
    fun encode(commandId: String, action: String, paramsJson: String): String {
        try {
            JSONObject(paramsJson)
        } catch (e: JSONException) {
            throw CommandException("ValidationError", "指令参数必须是合法 JSON 对象")
        }
        // 显式键序拼装（与桌面 serde 产物一致）；paramsJson 原样嵌入（上方已校验合法）。
        // 用 org.json.quote 转义字符串值，避免注入式破坏（envelope 是 wire 字符串）。
        val envelope = """{"schemaVersion":${SCHEMA_VERSION},"commandId":${JSONObject.quote(commandId)},"action":${JSONObject.quote(action)},"params":$paramsJson}"""
        if (envelope.toByteArray(Charsets.UTF_8).size > MAX_ENVELOPE_BYTES) {
            throw CommandException("ValidationError", "指令超出单帧上限（$MAX_ENVELOPE_BYTES 字节）")
        }
        return envelope
    }

    /** 便捷拼装 params（键→字符串/数字/布尔值）。 */
    fun buildParams(pairs: List<Pair<String, Any?>>): String {
        val obj = JSONObject()
        for ((k, v) in pairs) {
            if (v == null) continue
            obj.put(k, when (v) {
                is String, is Number, is Boolean -> v
                is JSONObject -> v
                else -> v.toString()
            })
        }
        return obj.toString()
    }
}

/**
 * 指令执行错误（镜像桌面 AppError 单键 map 形状 `{"<Variant>":"<msg>"}`）。
 * [variant] = AppError 变体名（NotFound/ValidationError/ConnectionError...）。
 */
class CommandException(val variant: String, message: String) : Exception(message) {
    override fun toString(): String = "$variant: $message"

    companion object {
        /** 从 ack error JSON 提取 variant 与 message；形状不符时降级为 ValidationError。 */
        fun fromErrorJson(error: Any?): CommandException {
            val obj = error as? JSONObject
                ?: return CommandException("ValidationError", "未知错误形状")
            // AppError 单键 map 契约：多键即契约破坏（迭代顺序不定，任取首键
            // 会把错误分类建立在不确定值上）——降级不猜。
            val variant = obj.keys().nextOrNull() ?: return CommandException("ValidationError", "空错误")
            if (obj.length() > 1) {
                return CommandException("ValidationError", "未知错误形状")
            }
            // 值非 String 即契约破坏，降级不抛
            val raw = obj.opt(variant)
            val msg = (raw as? String) ?: return CommandException("ValidationError", "未知错误形状")
            return CommandException(variant, msg)
        }
    }
}

/**
 * COMMAND_RESULT.data（桌面 → 手机）：`{"schemaVersion":1,"commandId":"...","ok":true,"result":{...}}`
 * 或 `{"...","ok":false,"error":{"<Variant>":"<msg>"}}`。
 */
data class CommandAck(
    val commandId: String,
    val ok: Boolean,
    val result: JSONObject?,
    val error: CommandException?,
) {
    /** 成功取 result，失败抛 [CommandException]；缺字段显式报错不伪造空结果。 */
    fun resultOrThrow(): JSONObject =
        if (ok) {
            result ?: throw CommandException("ValidationError", "成功回执缺少 result 字段")
        } else {
            throw error ?: CommandException("ValidationError", "空错误")
        }

    companion object {
        fun parse(data: String): CommandAck? {
            val json = try {
                JSONObject(data)
            } catch (e: JSONException) {
                return null
            }
            // schemaVersion 漂移显式拒收（整改：未来 v2 载荷会被静默误解析
            // 而非拒收——缺省 1 兼容桌面侧未携带该字段的历史回执形状）
            if (json.optInt("schemaVersion", 1) != CommandEnvelope.SCHEMA_VERSION) return null
            val commandId = json.optString("commandId", "")
            return if (json.optBoolean("ok", false)) {
                CommandAck(commandId, true, json.optJSONObject("result"), null)
            } else {
                CommandAck(commandId, false, null, CommandException.fromErrorJson(json.opt("error")))
            }
        }
    }
}

/**
 * STREAM_TOKEN.data（桌面 llm:stream StreamPayload 原样 JSON）：
 * `{conversationId, token, done, thinking, messageId?, phase?, statusText?, toolName?, processEvent?}`。
 *
 * 非法 JSON → null（fail-safe：跳过不断流，与桌面 mirror_stream_payload 同语义）。
 */
data class StreamEvent(
    val conversationId: String,
    val token: String,
    val done: Boolean,
    val thinking: Boolean,
    val messageId: String?,
    val phase: String?,
    val statusText: String?,
    val toolName: String?,
    val processEvent: ProcessEvent?,
) {
    /** 桌面 MessageProcessEvent 子集（models/chat.rs:36-48）。 */
    data class ProcessEvent(
        val eventType: String,
        val toolName: String?,
        val status: String?,
        val summary: String,
        val rawJson: String?,
        val workingDirectory: String?,
    )

    companion object {
        fun parse(data: String): StreamEvent? {
            val json = try {
                JSONObject(data)
            } catch (e: JSONException) {
                return null
            }
            // schemaVersion 漂移显式拒收（StreamPayload 无该字段——仅当显式
            // 携带且不等于 1 时拒收，防未来形状误解析）
            if (json.has("schemaVersion") && json.optInt("schemaVersion") != 1) return null
            val conversationId = json.optString("conversationId", "")
            if (conversationId.isEmpty()) return null
            val processEvent = json.optJSONObject("processEvent")?.let { pe ->
                ProcessEvent(
                    eventType = pe.optString("eventType", ""),
                    toolName = pe.optStringOrNull("toolName"),
                    status = pe.optStringOrNull("status"),
                    summary = pe.optString("summary", ""),
                    rawJson = pe.optStringOrNull("rawJson"),
                    workingDirectory = pe.optStringOrNull("workingDirectory"),
                )
            }
            return StreamEvent(
                conversationId = conversationId,
                token = json.optString("token", ""),
                done = json.optBoolean("done", false),
                thinking = json.optBoolean("thinking", false),
                messageId = json.optStringOrNull("messageId"),
                phase = json.optStringOrNull("phase"),
                statusText = json.optStringOrNull("statusText"),
                toolName = json.optStringOrNull("toolName"),
                processEvent = processEvent,
            )
        }
    }
}

private fun JSONObject.optStringOrNull(key: String): String? =
    if (isNull(key) || !has(key)) null else optString(key, null)

private fun <T> Iterator<T>.nextOrNull(): T? = if (hasNext()) next() else null
