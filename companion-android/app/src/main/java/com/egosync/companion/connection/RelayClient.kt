package com.egosync.companion.connection

import java.io.IOException
import kotlinx.coroutines.withTimeout
import okhttp3.OkHttpClient
import org.json.JSONObject

/**
 * 中继承载（AC4）：连 `<relay_addr>/relay` → 首消息 **text JSON**
 * `{"type":"register","relayId":"<16hex>","role":"phone"}` → relay 作 XX initiator
 * （一次性密钥对），手机作 responder 用真实静态私钥完成鉴权握手（3 条 binary）→
 * 转发态。控制协议逐字镜像 relay-server/src/auth.rs（不改 relay-server）。
 *
 * 鉴权完成后 noise transport 即弃（零知识：relay 不参与后续加密）——转发态里
 * 手机经转发通道对桌面发起 E2E XX initiator 握手（上层复用直连同款序列）。
 *
 * open 仅为编排层 JVM 测试的替换缝（P7）：生产装配恒为原类。
 */
open class RelayClient(private val wsClient: OkHttpClient) {

    /**
     * 完成注册与鉴权，返回可用于 E2E 握手/会话循环的转发态会话。
     * 任何一步失败即关闭连接（镜像 relay「失败不登记」的客户端侧纪律）。
     */
    internal open suspend fun connect(relayAddr: String, relayId: String, staticPrivate: ByteArray): WsSession {
        val url = if (relayAddr.endsWith("/")) "${relayAddr}relay" else "$relayAddr/relay"
        val session = openWsSession(url, wsClient, CONNECT_TIMEOUT_MS)
        try {
            // 1. register 首消息（text；16 位小写 hex relayId 由桌面 QR 派生保证）
            val register = JSONObject()
                .put("type", "register")
                .put("relayId", relayId)
                .put("role", "phone")
                .toString()
            if (!session.sendText(register)) throw IOException("register 发送失败")

            // 2. XX responder 鉴权握手：m1 入 → m2 出 → m3 入
            //    整体 10s 预算（镜像 AUTH_TIMEOUT_SECS 的单一截止点语义）
            withTimeout(AUTH_TIMEOUT_MS) {
                val channel = NoiseChannel.responder(staticPrivate)
                val m1 = session.incoming.receive()
                // P20：镜像 relay auth.rs 双向判据（<32 拒 / >4096 拒），
                // m1 与 m3 对称——本地纵深不依赖中继代为把关
                requireHandshakeMessage(m1, "m1")
                channel.readHandshakeMessage(m1)
                if (!session.send(channel.writeHandshakeMessage())) throw IOException("鉴权应答发送失败")
                val m3 = session.incoming.receive()
                requireHandshakeMessage(m3, "m3")
                channel.readHandshakeMessage(m3)
            }
            // 鉴权 transport 弃用（零知识纪律），session 交上层跑 E2E 握手
            return session
        } catch (e: Throwable) {
            session.close()
            throw e
        }
    }

    private fun requireHandshakeMessage(bytes: ByteArray, label: String) {
        if (bytes.size < MIN_HANDSHAKE_MSG_LEN || bytes.size > MAX_HANDSHAKE_MSG_LEN) {
            throw IOException("$label 鉴权消息长度非法（${bytes.size} 字节）")
        }
    }

    companion object {
        private const val CONNECT_TIMEOUT_MS = 10_000L
        private const val AUTH_TIMEOUT_MS = 10_000L
        private const val MIN_HANDSHAKE_MSG_LEN = 32

        /** 镜像 relay auth.rs 的 MAX_HANDSHAKE_MSG_LEN（4096）。 */
        private const val MAX_HANDSHAKE_MSG_LEN = 4096
    }
}
