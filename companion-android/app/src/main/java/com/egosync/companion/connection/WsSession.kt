package com.egosync.companion.connection

import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeout
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString

/**
 * 直连与中继共享的 WS 会话桥：OkHttp WS 消息泵 → [incoming] 通道，
 * 上层（E2E 握手/帧循环）对承载来源无感。text 帧仅 relay 控制协议使用，
 * 由各承载自行处理，本桥只转 binary。
 */
internal class WsSession(internal val ws: WebSocket) {

    val incoming = Channel<ByteArray>(capacity = Channel.UNLIMITED)

    lateinit var channel: NoiseChannel
    lateinit var transport: NoiseChannel.Transport

    fun send(bytes: ByteArray): Boolean = ws.send(bytes.toByteString())

    fun sendText(text: String): Boolean = ws.send(text)

    fun close() {
        incoming.close()
        runCatching { ws.close(1000, null) }
    }
}

/** 建立 WS 会话（调用方用 withTimeout 包预算，P4：连接不得无限挂起）。 */
internal suspend fun openWsSession(url: String, wsClient: OkHttpClient, connectTimeoutMs: Long): WsSession =
    kotlinx.coroutines.suspendCancellableCoroutine { cont ->
        val sessionRef = AtomicReference<WsSession?>(null)
        val request = Request.Builder().url(url).build()
        // P12：兑现 connectTimeoutMs 语义（此前参数未用，TCP 连接超时只靠外层
        // withTimeout——套接字层无预算会在极端场景拖满外层预算才失败）
        val client = wsClient.newBuilder()
            .connectTimeout(connectTimeoutMs, java.util.concurrent.TimeUnit.MILLISECONDS)
            .build()
        val ws = client.newWebSocket(
            request,
            object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    val session = WsSession(webSocket)
                    if (sessionRef.compareAndSet(null, session) && cont.isActive) {
                        cont.resume(session, onCancellation = null)
                    }
                }

                override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                    sessionRef.get()?.incoming?.trySend(bytes.toByteArray())
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    sessionRef.get()?.incoming?.close()
                    if (cont.isActive) cont.resumeWith(Result.failure(t))
                }

                override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                    // P12：对端优雅 Close（如桌面换绑拒绝路径）——立即完成关闭
                    // 握手并关闭入站通道，检测不得拖到 WS-ping 超时（15s+）
                    sessionRef.get()?.incoming?.close()
                    runCatching { webSocket.close(1000, null) }
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    sessionRef.get()?.incoming?.close()
                    if (cont.isActive) {
                        cont.resumeWith(Result.failure(java.io.IOException("连接已关闭（code=$code）")))
                    }
                }
            },
        )
        cont.invokeOnCancellation { ws.cancel() }
    }
