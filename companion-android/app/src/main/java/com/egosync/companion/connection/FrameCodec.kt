package com.egosync.companion.connection

import org.json.JSONException
import org.json.JSONObject

/**
 * 协议帧——JSON 形态 `{"type":"<小写帧名>", ...payload}`，与桌面
 * crates/companion-proto/src/frames.rs 一一对应。SNAPSHOT/STATE_DELTA/COMMAND/
 * COMMAND_RESULT/STREAM_TOKEN 五类本 story 仅定义占位，消费方为 13.x。
 */
sealed class Frame {
    data class Hello(val protocolVersion: Int) : Frame()
    data class Snapshot(val data: String) : Frame()
    data class StateDelta(val data: String) : Frame()
    data class Command(val data: String) : Frame()
    data class CommandResult(val data: String) : Frame()
    data class StreamToken(val data: String) : Frame()
    data class Notice(val data: String) : Frame()
    data object Ping : Frame()
}

/**
 * 帧编解码——镜像 frames.rs：`u32 大端长度前缀 + Noise 传输密文`，
 * 内层帧 JSON camelCase；HELLO 的 protocolVersion 必须等于 [PROTOCOL_VERSION]。
 *
 * 编码用显式键序拼装（org.json 的 JSONObject 键无序，而桌面 serde 产物键序固定：
 * type 在前、payload 字段在后），字符串值经 [JSONObject.quote] 转义——
 * 8 类帧的 wire 形态受 FrameCodecTest 逐字节黄金串约束。
 */
object FrameCodec {
    const val PROTOCOL_VERSION = 1

    /** 单帧密文上限（镜像 crypto.rs MAX_CIPHERTEXT_LEN，低于 relay WS 128KB 消息上限）。 */
    const val MAX_CIPHERTEXT_LEN = 65535

    private const val CHACHA_TAG_LEN = 16

    class FrameCodecException(message: String, cause: Throwable? = null) : Exception(message, cause)

    fun encode(frame: Frame, session: NoiseChannel.Transport): ByteArray {
        val json = when (frame) {
            is Frame.Hello -> """{"type":"hello","protocolVersion":${frame.protocolVersion}}"""
            is Frame.Snapshot -> tagged("snapshot", frame.data)
            is Frame.StateDelta -> tagged("state_delta", frame.data)
            is Frame.Command -> tagged("command", frame.data)
            is Frame.CommandResult -> tagged("command_result", frame.data)
            is Frame.StreamToken -> tagged("stream_token", frame.data)
            is Frame.Notice -> tagged("notice", frame.data)
            Frame.Ping -> """{"type":"ping"}"""
        }
        val plaintext = json.toByteArray(Charsets.UTF_8)
        if (plaintext.size > MAX_CIPHERTEXT_LEN - CHACHA_TAG_LEN) {
            throw FrameCodecException(
                "帧明文（${plaintext.size} 字节）超过单帧明文上限（${MAX_CIPHERTEXT_LEN - CHACHA_TAG_LEN} 字节）",
            )
        }
        val ciphertext = session.encrypt(plaintext)
        return byteArrayOf(
            (ciphertext.size ushr 24).toByte(),
            (ciphertext.size ushr 16).toByte(),
            (ciphertext.size ushr 8).toByte(),
            ciphertext.size.toByte(),
        ) + ciphertext
    }

    fun decode(bytes: ByteArray, session: NoiseChannel.Transport): Frame {
        if (bytes.size < 4) {
            throw FrameCodecException("帧数据不足 4 字节长度前缀（实际 ${bytes.size} 字节）")
        }
        val declared =
            ((bytes[0].toInt() and 0xFF) shl 24) or
                ((bytes[1].toInt() and 0xFF) shl 16) or
                ((bytes[2].toInt() and 0xFF) shl 8) or
                (bytes[3].toInt() and 0xFF)
        if (declared > MAX_CIPHERTEXT_LEN) {
            throw FrameCodecException("长度前缀（$declared）超过单帧密文上限（$MAX_CIPHERTEXT_LEN 字节）")
        }
        if (bytes.size - 4 != declared) {
            throw FrameCodecException("长度前缀（$declared）与实际密文字节数（${bytes.size - 4}）不符")
        }
        val plaintext = try {
            session.decrypt(bytes.copyOfRange(4, bytes.size))
        } catch (e: Exception) {
            throw FrameCodecException("传输密文解密失败", e)
        }
        val json = try {
            JSONObject(plaintext.decodeToString())
        } catch (e: JSONException) {
            throw FrameCodecException("帧 JSON 解析失败", e)
        }
        return when (val type = json.optString("type")) {
            "hello" -> {
                val version = try {
                    json.getInt("protocolVersion")
                } catch (e: JSONException) {
                    throw FrameCodecException("HELLO 缺失 protocolVersion 字段", e)
                }
                if (version != PROTOCOL_VERSION) {
                    throw FrameCodecException(
                        "协议版本不匹配：对端 protocolVersion=$version，本端=$PROTOCOL_VERSION",
                    )
                }
                Frame.Hello(version)
            }
            "snapshot" -> Frame.Snapshot(json.optString("data"))
            "state_delta" -> Frame.StateDelta(json.optString("data"))
            "command" -> Frame.Command(json.optString("data"))
            "command_result" -> Frame.CommandResult(json.optString("data"))
            "stream_token" -> Frame.StreamToken(json.optString("data"))
            "notice" -> Frame.Notice(json.optString("data"))
            "ping" -> Frame.Ping
            else -> throw FrameCodecException("未知帧类型: $type")
        }
    }

    private fun tagged(type: String, data: String): String =
        """{"type":"$type","data":${JSONObject.quote(data)}}"""
}
