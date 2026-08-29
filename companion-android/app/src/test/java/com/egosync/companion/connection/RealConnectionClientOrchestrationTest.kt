package com.egosync.companion.connection

import java.util.Collections
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.EmptyCoroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.Request
import okio.ByteString
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 连接编排层意图回归（P7：RealConnectionClient 是最大最状态化的类，P1 的
 * 「超时被吞」「状态机输出未接线」都死在这里——单测协作者全绿不等于装配正确）：
 *
 * - 「三态对 UI 可见」：编排内状态机输出必须驱动 connection.state
 *   （AC3/AC4/AC5 的「状态栏显示」在运行时才成立）；
 * - 「双承载皆断必须 Offline」：直连丢失 + 中继连不上（从未建立）不得永久滞留 Direct；
 * - 「unpair 擦除必须先于重配对密钥生成」：迟到的异步 wipe 不得毁掉新配对（AC6）；
 * - 「信任锚不等即断开」：中间人公钥必须被拒且会话关闭（负向安全测试）。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RealConnectionClientOrchestrationTest {

    // ── 测试替身 ──────────────────────────────────────────────────────

    private class FakeSharedPreferences(initialPaired: Boolean = false) : android.content.SharedPreferences {
        private val map = ConcurrentHashMap<String, Any?>().apply { put("paired", initialPaired) }

        override fun getAll(): MutableMap<String, *> = map.toMutableMap()
        override fun getString(key: String, defValue: String?): String? = map[key] as? String ?: defValue
        override fun getStringSet(key: String, defValues: MutableSet<String>?): MutableSet<String>? =
            @Suppress("UNCHECKED_CAST") (map[key] as? MutableSet<String>) ?: defValues
        override fun getInt(key: String, defValue: Int): Int = map[key] as? Int ?: defValue
        override fun getLong(key: String, defValue: Long): Long = map[key] as? Long ?: defValue
        override fun getFloat(key: String, defValue: Float): Float = map[key] as? Float ?: defValue
        override fun getBoolean(key: String, defValue: Boolean): Boolean = map[key] as? Boolean ?: defValue
        override fun contains(key: String): Boolean = map.containsKey(key)
        override fun edit(): android.content.SharedPreferences.Editor = FakeEditor()
        override fun registerOnSharedPreferenceChangeListener(listener: android.content.SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
        override fun unregisterOnSharedPreferenceChangeListener(listener: android.content.SharedPreferences.OnSharedPreferenceChangeListener?) = Unit

        private inner class FakeEditor : android.content.SharedPreferences.Editor {
            // key/value 可能为 null（SDK 契约）：null 键无操作；null 值按「移除」处理
            // （ConcurrentHashMap 不收 null 值，语义上与 getString 返回默认值等价）
            override fun putString(key: String?, value: String?) = apply {
                if (key != null) { if (value != null) map[key] = value else map.remove(key) }
            }
            override fun putStringSet(key: String?, values: MutableSet<String>?) = apply {
                if (key != null) { if (values != null) map[key] = values else map.remove(key) }
            }
            override fun putInt(key: String?, value: Int) = apply { key?.let { map[it] = value } }
            override fun putLong(key: String?, value: Long) = apply { key?.let { map[it] = value } }
            override fun putFloat(key: String?, value: Float) = apply { key?.let { map[it] = value } }
            override fun putBoolean(key: String?, value: Boolean) = apply { key?.let { map[it] = value } }
            override fun remove(key: String?) = apply { key?.let { map.remove(it) } }
            override fun clear() = apply { map.clear() }
            override fun commit() = true
            override fun apply() = Unit
        }
    }

    /** 静态密钥替身：记录事件序（P3 竞态回归用），密钥内容固定 32 字节。 */
    private class FakeSecrets : SecretsProvider {
        val events = Collections.synchronizedList(mutableListOf<String>())
        var wipeDelayMs = 0L
        override fun loadOrCreateStaticPrivateKey(): ByteArray {
            events.add("load")
            return ByteArray(32) { 7 }
        }
        override fun wipe() {
            events.add("wipe")
            if (wipeDelayMs > 0) Thread.sleep(wipeDelayMs)
            events.add("wipe-done")
        }
    }

    /** NSD 替身：replay=1 保证 discover 后 awaitResolved 立即取到（可关停模拟桌面消失）。 */
    private class FakeNsd : NsdDiscoverer {
        override val resolved = MutableSharedFlow<NsdEndpoint>(replay = 1, extraBufferCapacity = 1)
        override val failed = MutableSharedFlow<Unit>(replay = 0, extraBufferCapacity = 1)
        var resolveEnabled = true
        override fun discover(expectedInstanceName: String) {
            if (resolveEnabled) resolved.tryEmit(NsdEndpoint("127.0.0.1", 47_000))
        }
        override fun stopDiscovery() = Unit
    }

    /** okhttp WebSocket 替身：记录发送序与关闭态（P4 之外的 JVM 会话承载）。 */
    private class FakeWebSocket : okhttp3.WebSocket {
        val sent = Collections.synchronizedList(mutableListOf<ByteArray>())
        @Volatile var closed = false
        override fun request(): Request = Request.Builder().url("ws://127.0.0.1:1/").build()
        override fun queueSize(): Long = 0L
        override fun send(text: String): Boolean = false
        override fun send(bytes: ByteString): Boolean {
            sent.add(bytes.toByteArray())
            return true
        }
        override fun close(code: Int, reason: String?): Boolean {
            closed = true
            return true
        }
        override fun cancel() {
            closed = true
        }
    }

    /** 中继替身：恒不可达（P2 回归用）。 */
    private class FakeUnreachableRelay : RelayClient(OkHttpClient()) {
        override suspend fun connect(relayAddr: String, relayId: String, staticPrivate: ByteArray): WsSession =
            throw java.io.IOException("中继不可达（测试注入）")
    }

    /**
     * 进程内「桌面/冒名」WS 会话：真 NoiseChannel responder 泵——m1 入 → m2 出 →
     * m3 入 → split →（可选）HELLO+deviceInfo 帧确认。
     */
    private fun openPumpedSession(
        pumpScope: CoroutineScope,
        responderPriv: ByteArray,
        readIntroFrames: Boolean,
    ): WsSession {
        val ws = FakeWebSocket()
        val session = WsSession(ws)
        pumpScope.launch {
            val responder = NoiseChannel.responder(responderPriv)
            awaitMinSize(ws.sent, 1)
            responder.readHandshakeMessage(ws.sent[0]) // m1
            session.incoming.trySend(responder.writeHandshakeMessage()) // m2
            awaitMinSize(ws.sent, 2)
            responder.readHandshakeMessage(ws.sent[1]) // m3
            if (readIntroFrames) {
                val transport = responder.split()
                awaitMinSize(ws.sent, 4)
                FrameCodec.decode(ws.sent[2], transport) // HELLO
                FrameCodec.decode(ws.sent[3], transport) // NOTICE(deviceInfo)
            }
        }
        return session
    }

    private suspend fun awaitMinSize(sent: MutableList<ByteArray>, n: Int) {
        while (sent.size < n) delay(10)
    }

    private fun validQr(pubkeyHex: String) =
        """{"relayAddr":null,"desktopStaticPubkey":"$pubkeyHex",""" +
            """"relayId":"1122334455667788","pairingNonce":"n-1"}"""

    // ── 用例 ──────────────────────────────────────────────────────────

    @Test
    fun `三态必须经编排接线驱动 UI 状态流`() = runTest {
        // WHY（P1）：状态机输出此前从未被收集进 liveState——冷启动已配对用户
        // 永远显示离线、中继态永不显示；机制单测全绿但装配层输出被整体丢弃。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val sessions = Collections.synchronizedList(mutableListOf<WsSession>())
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                openPumpedSession(backgroundScope, deskPriv, readIntroFrames = true)
                    .also { sessions.add(it) }
            },
        )

        // 冷启动：如实 Offline，直连建立后必须翻 Direct（接线生效的唯一证明）
        advanceTimeBy(2_000)
        assertEquals("直连建立后 UI 状态必须为 Direct", ConnectionState.Direct, client.state.value)

        // 会话断开：如实 Offline（不误报在线，AC5）——断开后 ~1s 内重连会再度
        // 翻 Direct，Offline 断言必须落在退避窗内（首跳 [500,1000]ms）
        sessions.first().incoming.close()
        advanceTimeBy(100)
        assertTrue("会话断开后必须 Offline", client.state.value is ConnectionState.Offline)

        // 重连成功：回到 Direct（三态可反复如实翻转）
        advanceTimeBy(5_000)
        assertEquals("重连后必须回到 Direct", ConnectionState.Direct, client.state.value)
    }

    @Test
    fun `直连丢失且中继不可达必须到达 Offline`() = runTest {
        // WHY（P2）：中继「从未建立」的失败此前是状态机 no-op——Direct 滞回窗内
        // 双承载皆断却永久驻留 Direct，UI 误报在线（AC5 击穿）。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", "ws://relay.test:7333")
        val nsd = FakeNsd().apply { resolveEnabled = false } // 桌面彻底消失
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            relayClient = FakeUnreachableRelay(),
        )

        // t=12s 直连发现超时 → 滞回 3s → t=15s 中继尝试失败（从未建立）→ Offline
        advanceTimeBy(16_000)
        assertTrue(
            "双承载皆不可达必须 Offline（不得滞留 Direct）",
            client.state.value is ConnectionState.Offline,
        )
    }

    @Test
    fun `unpair 的异步擦除必须先于重配对密钥生成完成`() = runBlocking {
        // WHY（P3）：擦除是后台任务——重配对不等它完成，新密钥/新元数据会被
        // 迟到的 wipe 删掉，手机「已配对」却永久无法解包私钥（AC6 信任链损坏）。
        val secrets = FakeSecrets().apply { wipeDelayMs = 150 }
        val store = PairingStateStore(FakeSharedPreferences(initialPaired = false))
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val client = RealConnectionClient(scope, store, secrets, FakeNsd())

        client.unpair()
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(ByteArray(32) { (it + 1).toByte() }))
        client.pairWithQr(validQr(deskPubHex))

        val deadline = System.currentTimeMillis() + 5_000
        while (!secrets.events.contains("load") && System.currentTimeMillis() < deadline) {
            delay(20)
        }
        assertTrue("重配对必须触发密钥生成", secrets.events.contains("load"))
        assertTrue(
            "wipe 必须先于 load 完成（join 生效）；实际序：${secrets.events}",
            secrets.events.indexOf("wipe-done") < secrets.events.indexOf("load"),
        )
        scope.cancel()
        Unit
    }

    @Test
    fun `信任锚不等的会话必须被拒绝并关闭`() = runTest {
        // WHY：信任锚不可绕过——冒名 responder 的公钥 ≠ QR 桌面公钥时必须断开；
        // 放行即中间人可永久驻留（配对信任链的负向安全测试）。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val roguePriv = ByteArray(32) { (it + 9).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val sessions = Collections.synchronizedList(mutableListOf<WsSession>())
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                openPumpedSession(backgroundScope, roguePriv, readIntroFrames = false)
                    .also { sessions.add(it) }
            },
        )

        advanceTimeBy(2_000)
        assertTrue("冒名公钥必须落入 Offline", client.state.value is ConnectionState.Offline)
        val rogueWs = sessions.first().ws as FakeWebSocket
        assertTrue("冒名会话必须被关闭（不等 WS-ping 超时）", rogueWs.closed)
        assertTrue(
            "信任锚拒绝发生在握手完成后（m1/m3 已发出）",
            rogueWs.sent.size >= 2,
        )
    }
}
