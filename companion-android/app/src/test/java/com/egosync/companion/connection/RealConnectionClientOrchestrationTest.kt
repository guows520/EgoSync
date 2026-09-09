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
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import okio.ByteString
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 连接编排层意图回归（P7：RealConnectionClient 是最大最状态化的类，P1 的
 * 「超时被吞」「状态机输出未接线」都死在这里——单测协作者全绿不等于装配正确）：
 *
 * - 「三态对 UI 可见」：编排内状态机输出必须驱动 connection.state
 *   （AC3/AC4/AC5 的「状态栏显示」在运行时才成立）；
 * - 「双承载皆断经宽限到 Degraded」：直连丢失 + 中继连不上（从未建立）不得
 *   永久滞留 Direct，20s 宽限耗尽如实降级（SPEC state-model §2/§3）；
 * - 「unpair 擦除必须先于重配对密钥生成」：迟到的异步 wipe 不得毁掉新配对（AC6）；
 * - 「信任锚不等即断开」：中间人公钥必须被拒且会话关闭（负向安全测试）。
 * - 「损坏配对态冷启动必须自愈」：覆盖安装遗留 paired=true 缺 relayId →
 *   编排静默退出＋遮罩无出口即永久死锁（FR-40 重装重扫即恢复）。
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

    /** 静态密钥替身：记录事件序（P3 竞态回归用），密钥内容固定 32 字节；
     *  T-S4 拆分后可注入缺文件（hasKey=false）/密文失效（invalid=true）。 */
    private class FakeSecrets : SecretsProvider {
        val events = Collections.synchronizedList(mutableListOf<String>())
        var wipeDelayMs = 0L
        var hasKey = true
        var invalid = false
        override fun hasStaticPrivateKey(): Boolean = hasKey
        override fun loadExistingStaticPrivateKey(): ByteArray {
            events.add("load")
            if (!hasKey) throw MissingSecretsException()
            if (invalid) throw SecretsInvalidatedException()
            return ByteArray(32) { 7 }
        }
        override fun createForPairingStaticPrivateKey(): ByteArray {
            events.add("create")
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
        var discoverCount = 0
            private set
        override fun discover(expectedInstanceName: String) {
            discoverCount++
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

    /** 中继替身（Story 12.5）：每次 connect 产出注入的会话（回退编排测试缝）。 */
    private class FakeRelay(private val sessionProvider: () -> WsSession) : RelayClient(OkHttpClient()) {
        var connectCount = 0
            private set
        override suspend fun connect(relayAddr: String, relayId: String, staticPrivate: ByteArray): WsSession {
            connectCount++
            return sessionProvider()
        }
    }

    /**
     * 进程内「桌面/冒名」WS 会话：真 NoiseChannel responder 泵——m1 入 → m2 出 →
     * m3 入 → split →（可选）读 N 条 intro 帧 →（可选）以 Ping 帧应答 PING
     * （probeSessionAlive 的存活判据，Story 12.5 配对回退用：answerPing=false
     * 模拟桌面 pending 关闭连接、探测超时）。
     */
    private fun openPumpedSession(
        pumpScope: CoroutineScope,
        responderPriv: ByteArray,
        readIntroFrameCount: Int,
        answerPing: Boolean = false,
        /** T-S5：读毕 intro 帧即发该 Notice 载荷并关连接（模拟桌面拒绝点行为）。 */
        rejectAfterIntro: String? = null,
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
            if (readIntroFrameCount > 0 || answerPing || rejectAfterIntro != null) {
                val transport = responder.split()
                val introEnd = 2 + readIntroFrameCount
                if (readIntroFrameCount > 0) awaitMinSize(ws.sent, introEnd)
                for (i in 2 until introEnd) FrameCodec.decode(ws.sent[i], transport)
                if (rejectAfterIntro != null) {
                    // 桌面拒绝点同构（companion_connection.rs：先 Notice 再关连接）
                    session.incoming.trySend(FrameCodec.encode(Frame.Notice(rejectAfterIntro), transport))
                    ws.close(1000, "pairingRejected")
                } else if (answerPing) {
                    awaitMinSize(ws.sent, introEnd + 1)
                    val ping = FrameCodec.decode(ws.sent[introEnd], transport)
                    check(ping is Frame.Ping) { "intro 后首帧应为 PING 探测" }
                    session.incoming.trySend(FrameCodec.encode(Frame.Ping, transport))
                }
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

    /** 携带中继地址的 QR（Story 12.5：跨网回退的前提）。 */
    private fun qrWithRelay(pubkeyHex: String) =
        """{"relayAddr":"ws://relay.test:7333","desktopStaticPubkey":"$pubkeyHex",""" +
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
                openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 2)
                    .also { sessions.add(it) }
            },
        )

        // 冷启动：第一帧 Connecting（T-S3 宽限态），直连建立后必须翻 Direct（接线生效的唯一证明）
        advanceTimeBy(2_000)
        assertEquals("直连建立后 UI 状态必须为 Direct", TransportStatus.Direct, client.state.value)

        // 会话断开：Reconnecting（宽限中，不误报在线 AC5）——断开后 ~1s 内重连会
        // 再度翻 Direct，宽限态断言必须落在退避窗内（首跳 [500,1000]ms）
        sessions.first().incoming.close()
        advanceTimeBy(100)
        assertTrue("会话断开后必须 Reconnecting（非误报在线）", client.state.value is TransportStatus.Reconnecting)

        // 重连成功：回到 Direct（承载态可反复如实翻转）
        advanceTimeBy(5_000)
        assertEquals("重连后必须回到 Direct", TransportStatus.Direct, client.state.value)
    }

    @Test
    fun `bind先于Direct发布且断开时commandReady先行翻false`() = runTest {
        // WHY（T-S2 / Finding 7）：UI 以 commandReady 为写操作判据——若
        // onDirectEstablished 先于 bind 发布 Direct，滞回/竞窗内会出现
        // 「展示在线、点发送报桌面引擎不可达」；断开方向同理：unbind（false）
        // 必须先于状态机失去处理，UI 观察不到「在线展示+不可发送」组合。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val sessions = Collections.synchronizedList(mutableListOf<WsSession>())
        val commandChannel = com.egosync.companion.command.CommandChannel()
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            commandChannel = commandChannel,
            wsOpener = { _, _, _ ->
                openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 2)
                    .also { sessions.add(it) }
            },
        )
        // 每次状态发射时刻的 (状态档, commandReady) 快照——时序不变量的可观察证明
        val observations = Collections.synchronizedList(mutableListOf<Pair<String, Boolean>>())
        backgroundScope.launch {
            client.state.collect { s ->
                val label = if (s is TransportStatus.Direct) "Direct" else "notReady"
                observations.add(label to commandChannel.sessionActive.value)
            }
        }

        advanceTimeBy(2_000)
        val directObs = observations.filter { it.first == "Direct" }
        assertTrue("直连建立期间必须观察到 Direct", directObs.isNotEmpty())
        assertTrue(
            "Direct 发布时刻 commandReady 必须已为 true（bind 先行）",
            directObs.all { it.second },
        )

        sessions.first().incoming.close()
        advanceTimeBy(100)
        val offlineObs = observations.filter { it.first == "notReady" }
        assertTrue("会话断开后必须观察到非在线态（Reconnecting/宽限）", offlineObs.isNotEmpty())
        assertTrue(
            "非在线态发布时刻 commandReady 必须已为 false（unbind 先行）",
            offlineObs.none { it.second },
        )
    }

    @Test
    fun `直连丢失且中继不可达经宽限后到达Degraded`() = runTest {
        // WHY（P2 + SPEC state-model §3）：中继「从未建立」的失败此前是状态机
        // no-op——Direct 滞回窗内双承载皆断却永久驻留 Direct，UI 误报在线
        // （AC5 击穿）；新语义下双断不再瞬时 Offline，而是经 20s 宽限如实降级。
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

        // t=12s 直连发现超时 → 滞回 3s → t=15s 中继尝试失败（从未建立）→
        // 宽限中 Connecting；宽限（自编排启动起 20s）耗尽 → Degraded
        advanceTimeBy(16_000)
        assertTrue(
            "双承载皆不可达不得滞留 Direct（宽限中 Connecting/Reconnecting）",
            client.state.value.let { it !is TransportStatus.Direct && it !is TransportStatus.Relay },
        )
        advanceTimeBy(6_000)
        assertTrue(
            "宽限耗尽必须 Degraded（不得误报在线）",
            client.state.value is TransportStatus.Degraded,
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
        while (!secrets.events.contains("create") && System.currentTimeMillis() < deadline) {
            delay(20)
        }
        assertTrue("重配对必须触发密钥生成（T-S4：配对路径显式 createForPairing）", secrets.events.contains("create"))
        assertTrue(
            "wipe 必须先于 create 完成（join 生效）；实际序：${secrets.events}",
            secrets.events.indexOf("wipe-done") < secrets.events.indexOf("create"),
        )
        scope.cancel()
        Unit
    }

    @Test
    fun `信任锚不等的会话必须被拒绝并关闭`() = runTest {
        // WHY：信任锚不可绕过——冒名 responder 的公钥 ≠ QR 桌面公钥时必须断开；
        // 放行即中间人可永久驻留（配对信任链的负向安全测试）。
        // T-S4 扩展：已配对上下文命中即 PairingRevoked/TrustMismatch 类恢复——
        // 桌面身份变化（重装）靠退避重试永远不可能自愈，必须停环清态发布事件。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val roguePriv = ByteArray(32) { (it + 9).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val sessions = Collections.synchronizedList(mutableListOf<WsSession>())
        val recoveries = Collections.synchronizedList(mutableListOf<PairingRecoveryReason>())
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                openPumpedSession(backgroundScope, roguePriv, readIntroFrameCount = 0)
                    .also { sessions.add(it) }
            },
        )
        // 事件订阅必须先于首次发射（SharedFlow 无 replay——运行中恢复事件只送达
        // 当时的订阅者），订阅协程与编排协程同排队，编排在 m2 等待处挂起让位
        backgroundScope.launch { client.pairingRecovery.collect { recoveries.add(it) } }

        advanceTimeBy(2_000)
        assertTrue("信任锚失败必须发布 TrustMismatch 恢复事件", recoveries == listOf(PairingRecoveryReason.TrustMismatch))
        assertEquals("健康面必须亮 TrustMismatch（驱动配对屏重配原因提示）", PairingHealth.TrustMismatch, client.pairingHealth.value)
        assertFalse("配对态必须清除（旧信任链已不可用）", client.paired.value)
        assertFalse("配对元数据必须落盘清除", store.paired)
        assertTrue("恢复后承载态如实 Degraded（不误报在线）", client.state.value is TransportStatus.Degraded)
        val rogueWs = sessions.first().ws as FakeWebSocket
        assertTrue("冒名会话必须被关闭（不等 WS-ping 超时）", rogueWs.closed)
        assertTrue(
            "信任锚拒绝发生在握手完成后（m1/m3 已发出）",
            rogueWs.sent.size >= 2,
        )
        // 恢复必须终止重试环：桌面不换回来，退避重试只会无限冒名拒绝
        advanceTimeBy(20_000)
        assertEquals("恢复后不得继续尝试连接冒名者（重试环已终止）", 1, sessions.size)
    }

    @Test
    fun `损坏配对态冷启动必须自愈且零网络尝试`() = runTest {
        // WHY（H6 死锁回归）：覆盖安装遗留 paired=true 但 relayId 缺失——编排
        // 首轮即静默 return（零重试零网络活动），叠加离线遮罩无应用内出口，
        // 用户永久锁死「离线 · 暂无缓存」（FR-40 击穿）。自愈必须发生在 init
        // 同步段：AppNavHost.startDestination 在首组合 remember{} 定格，晚了
        // paired 仍为 true，冷启动仍直落主界面＋遮罩。
        val store = PairingStateStore(FakeSharedPreferences(initialPaired = true))
        var wsOpens = 0
        val nsd = FakeNsd()
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                wsOpens++
                throw AssertionError("自愈生效后不得发起任何 WS 连接尝试")
            },
        )

        // 时序护栏：不 advanceTimeBy 直接断言——自愈若被挪进编排协程（异步），
        // 此处 paired 仍为 true，测试必须红（防护 startDestination 首组合定格）
        assertFalse("自愈必须在 init 同步段完成（协程启动前）", client.paired.value)

        advanceTimeBy(2_000)
        assertFalse("损坏配对态必须自愈回未配对（冷启动直落配对扫码流）", client.paired.value)
        // 持久层断言：仅清内存标志不够——store.clear() 被删时跨重启损坏态复活
        assertFalse("自愈必须落盘清除（跨重启不得复活损坏态）", store.paired)
        assertNull("损坏元数据必须一并清除（relayId）", store.relayId)
        assertNull("损坏元数据必须一并清除（desktopPubkeyHex）", store.desktopPubkeyHex)
        assertEquals("不得发起任何 WS 连接尝试", 0, wsOpens)
        assertEquals("不得发起任何 NSD 发现", 0, nsd.discoverCount)
        assertEquals(
            "未配对态沿用 Direct 既有语义（指示条不显示、不闪现离线）",
            TransportStatus.Direct,
            client.state.value,
        )
    }

    @Test
    fun `健康配对态冷启动行为不变`() = runTest {
        // WHY：自愈判定不得误伤健康配对——relayId 与桌面公钥齐全时编排必须
        // 照常启动（退避重连是已配对设备的正常离线表现，唯一出路仍是自动恢复），
        // 若自愈把健康态也清掉，每次冷启动都强制重扫即配对功能回归性损坏。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val nsd = FakeNsd()
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 2)
            },
        )

        advanceTimeBy(2_000)
        assertTrue("健康配对态必须保持已配对", client.paired.value)
        assertTrue("编排必须照常启动（NSD 发现发生、直连建立翻 Direct）", nsd.discoverCount > 0)
        assertEquals("直连建立后 UI 状态必须为 Direct", TransportStatus.Direct, client.state.value)
    }

    @Test
    fun `单缺 relayId 或单缺公钥同样自愈`() = runTest {
        // WHY：自愈判定是 OR——若回归成 AND，单字段损坏（如残留空串 relay_id）
        // 仍落入零重试死锁。两分支各自构造最小损坏态锁定 OR 语义（含空串边界，
        // isNullOrBlank：旧版残留的空值元数据不得绕过自愈）。
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(ByteArray(32) { 3 }))
        // 分支一：仅缺 relayId（公钥齐全）
        val storeA = PairingStateStore(
            FakeSharedPreferences(initialPaired = true).apply {
                edit().putString("desktop_pubkey_hex", deskPubHex).commit()
            },
        )
        // 分支二：仅缺 desktopPubkeyHex（relayId 为空串）
        val storeB = PairingStateStore(
            FakeSharedPreferences(initialPaired = true).apply {
                edit().putString("relay_id", "").commit()
            },
        )
        for (store in listOf(storeA, storeB)) {
            val client = RealConnectionClient(
                scope = backgroundScope,
                store = store,
                secrets = FakeSecrets(),
                nsd = FakeNsd(),
                ioDispatcher = EmptyCoroutineContext,
                wsOpener = { _, _, _ -> throw AssertionError("自愈生效后不得发起任何 WS 连接尝试") },
            )
            assertFalse("单字段损坏必须触发自愈（OR 分支）", client.paired.value)
            assertFalse("自愈必须落盘清除", store.paired)
        }
    }

    // ── T-S4 凭据生命周期 ───────────────────────────────────────────

    @Test
    fun `已配对私钥缺失冷启动不自建身份并亮CredentialMissing`() = runTest {
        // WHY（SPEC state-model §5.1）：paired=true 但包裹私钥文件缺失（重装/系统
        // 数据清理残留）——若静默生成新私钥「顶替」，桌面公钥记录对不上新身份，
        // 手机会永远被桌面拒绝且用户无从知晓；必须如实清配对态回重扫路径，
        // 健康面亮 CredentialMissing 供配对屏展示原因（冷启动无订阅者在跑，
        // 一次性事件不发——健康态承载原因）。
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(ByteArray(32) { 3 }))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null) // 元数据齐全：只能走缺钥分支
        val secrets = FakeSecrets().apply { hasKey = false }
        val nsd = FakeNsd()
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = secrets,
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ -> throw AssertionError("缺钥自愈后不得发起任何 WS 连接尝试") },
        )

        // 时序护栏：不 advanceTimeBy 直接断言（镜像损坏配对态测试——
        // startDestination 首组合定格要求自愈在 init 同步段完成）
        assertFalse("缺钥自愈必须在 init 同步段完成", client.paired.value)
        assertEquals("健康面必须亮 CredentialMissing（配对屏原因提示）", PairingHealth.CredentialMissing, client.pairingHealth.value)

        advanceTimeBy(2_000)
        assertFalse("配对态必须清除（不得换新身份顶替）", client.paired.value)
        assertFalse("清除必须落盘", store.paired)
        // wipe（异步 IO）合法：清残留 Keystore 别名；load/create 必须为零——
        // 生成新身份即「静默顶替」，桌面公钥记录对不上新公钥，手机被永久拒绝
        assertTrue(
            "不得生成新私钥（load/create 均不得发生）：实际序 ${secrets.events}",
            secrets.events.none { it == "load" || it == "create" },
        )
        assertEquals("不得发起任何 WS 连接尝试", 0, nsd.discoverCount)
        assertEquals(
            "未配对态沿用 Direct 既有语义（指示条不显示）",
            TransportStatus.Direct,
            client.state.value,
        )
    }

    @Test
    fun `运行中密钥失效触发原子恢复序列且重试环终止`() = runTest {
        // WHY（SPEC state-model §5.2）：Keystore 失效/密文损坏在运行中暴露时，
        // 旧处理只清态不导航不提示（用户面对静默死循环）；且退避重试永远不
        // 可能自愈（密钥永远解不开）。原子序列：会话失效 → 清配对态 → 擦密钥
        // → 健康面 → 恢复事件（容器后续清快照/通知 + 导航，此处验证客户端侧
        // 序列与事件载荷）。
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(ByteArray(32) { 3 }))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val secrets = FakeSecrets()
        val nsd = FakeNsd().apply { resolveEnabled = false } // 编排首轮挂起在 NSD 等待
        val recoveries = Collections.synchronizedList(mutableListOf<PairingRecoveryReason>())
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = secrets,
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ -> throw AssertionError("密钥失效场景不得发起 WS 连接") },
        )
        backgroundScope.launch { client.pairingRecovery.collect { recoveries.add(it) } }
        runCurrent() // 编排首轮进入 NSD 等待（密钥尚未失效），事件订阅者就位

        // 运行中失效：次轮发现恢复（NSD 可解析）但密钥已不可解——编排走到
        // 取钥步骤即暴露 SecretsInvalidated，走恢复
        secrets.invalid = true
        nsd.resolveEnabled = true
        // 首轮 NSD 超时（12s）→ 退避（1s 档）→ 次轮发现成功、取钥抛异常 →
        // 恢复终止重试环
        // 首轮 NSD 超时（12s）→ 退避（1s 档含抖动）→ 次轮发现成功、取钥抛
        // SecretsInvalidated → 恢复终止重试环。advanceTimeBy 一次推越全部节点
        //（advanceUntilIdle 对此后台任务链不推进，实测零前进——故用显式窗口）
        advanceTimeBy(14_000)
        runCurrent()

        assertEquals("必须发布 CredentialInvalid 恢复事件", listOf(PairingRecoveryReason.CredentialInvalid), recoveries.toList())
        assertEquals("健康面必须亮 CredentialInvalid", PairingHealth.CredentialInvalid, client.pairingHealth.value)
        assertFalse("配对态必须清除", client.paired.value)
        assertFalse("清除必须落盘", store.paired)
        assertTrue("承载态如实 Degraded（不误报在线）", client.state.value is TransportStatus.Degraded)
        // 恢复必须终止重试环：密钥永远解不开，继续重试即静默死循环
        val attemptsAtRecovery = nsd.discoverCount
        advanceTimeBy(30_000)
        assertEquals("恢复后不得继续 NSD 重试（重试环已终止）", attemptsAtRecovery, nsd.discoverCount)
        // 擦除走 IO 调度（异步）：真实时钟轮询等待 wipe 事件（镜像 P3 测试模式）
        val deadline = System.currentTimeMillis() + 5_000
        while (!secrets.events.contains("wipe") && System.currentTimeMillis() < deadline) {
            withContext(Dispatchers.Default) { delay(20) }
        }
        assertTrue("恢复序列必须擦除本机密钥（残留只会造成永久失败重试）", secrets.events.contains("wipe"))
    }

    // ── 中继首配回退（Story 12.5 AC1/AC3）──────────────────────────

    @Test
    fun `NSD 发现超时后必须回退中继完成首配`() = runTest {
        // WHY（AC1）：出差/异地场景 NSD 永远超时——不回退中继，用户必须先
        // 回家连入局域网才能绑定（「未发现桌面设备」的现状即此限制，见调查
        // 档案 companion-public-discovery-investigation）。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        val nsd = FakeNsd().apply { resolveEnabled = false } // 跨网：局域网发现必然超时
        val relay = FakeRelay {
            // 中继首配帧序：HELLO + deviceInfo + pairingAuth（3 条 intro）
            openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 3, answerPing = true)
        }
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            relayClient = relay,
        )

        client.pairWithQr(qrWithRelay(deskPubHex))
        // t=12s NSD 超时 → 中继回退 → 握手/信任锚/帧序 → PING 探测 PONG → Success
        advanceTimeBy(15_000)

        assertEquals("中继回退后配对必须成功", PairingProgress.Success, client.pairingProgress.value)
        assertTrue("配对态必须落盘", store.paired)
        assertEquals(
            "relayAddr 必须随配对落盘（T2：后续中继环据此自然启动）",
            "ws://relay.test:7333",
            store.relayAddr,
        )
        assertEquals("中继承载必须发生（而非 NSD 直连）", 1, relay.connectCount)
    }

    @Test
    fun `NSD 与中继双双失败必须如实合并文案`() = runTest {
        // WHY（Task 3）：回退后中继也失败时若仍提示「请确认在同一网络」，
        // 跨网用户会误判为自己网络配置错误——真实原因是中继不可达/配置有误，
        // 文案必须如实合并两项失败。
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(ByteArray(32) { 5 }))
        val store = PairingStateStore(FakeSharedPreferences())
        val nsd = FakeNsd().apply { resolveEnabled = false }
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            relayClient = FakeUnreachableRelay(),
        )

        client.pairWithQr(qrWithRelay(deskPubHex))
        advanceTimeBy(15_000)

        val progress = client.pairingProgress.value
        assertTrue("双失败必须进入 Failed", progress is PairingProgress.Failed)
        assertEquals(
            "双失败文案必须如实合并（发现 + 中继）",
            "未发现桌面设备，且中继连接失败，请检查网络或中继配置",
            (progress as PairingProgress.Failed).message,
        )
        assertTrue("失败后状态如实降级（不误报在线）", client.state.value is TransportStatus.Degraded)
        assertFalse("失败不得落配对态", store.paired)
    }

    @Test
    fun `pending 等待期重连必须走中继并在确认后成功`() = runTest {
        // WHY（AC2/AC3）：中继首配入 pending 确认门后手机只能等桌面确认——
        // 重连若仍走 NSD（原行为），跨网场景永远等不到确认后的 AlreadyPaired，
        // 120s 超时如实 Failed，确认门在异地场景反而变成永久死锁。
        val deskPriv = ByteArray(32) { (it + 7).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        val nsd = FakeNsd().apply { resolveEnabled = false }
        val progressLog = Collections.synchronizedList(mutableListOf<PairingProgress>())
        // 首连：桌面入 pending、关闭连接（answerPing=false → 探测 4s 超时）；
        // 重连（needsPairingAuth=false，2 条 intro）：确认后 AlreadyPaired 应答 PING
        val sessionProviders = mutableListOf<() -> WsSession>()
        sessionProviders += { openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 3, answerPing = false) }
        sessionProviders += { openPumpedSession(backgroundScope, deskPriv, readIntroFrameCount = 2, answerPing = true) }
        var providerIndex = 0
        val relay = FakeRelay { sessionProviders[providerIndex++].invoke() }
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = nsd,
            ioDispatcher = EmptyCoroutineContext,
            relayClient = relay,
        )
        backgroundScope.launch { client.pairingProgress.collect { progressLog.add(it) } }

        client.pairWithQr(qrWithRelay(deskPubHex))
        // 12s NSD 超时 → 中继首连 → pending 探测 4s 失败 → WaitDesktopConfirm
        // → 3s 后中继重连 → PONG → Success
        advanceTimeBy(25_000)

        assertEquals("确认后重连必须配对成功", PairingProgress.Success, client.pairingProgress.value)
        assertTrue("确认后配对态必须落盘", store.paired)
        assertEquals("重连必须经中继（首连 + 确认后重连）", 2, relay.connectCount)
        assertTrue(
            "等待期必须呈现 WaitDesktopConfirm（UI 等待文案的事实源）",
            progressLog.any { it == PairingProgress.WaitDesktopConfirm },
        )
    }

    // ── T-S5 结构化配对拒绝 ─────────────────────────────────────────

    @Test
    fun `配对probe收到结构化拒绝直接Failed且不再等待桌面确认`() = runTest {
        // WHY（SPEC qr-semantics §3.2）：旧 QR/已被消费的码重扫——桌面先发
        // pairingRejected Notice 再关连接。现状「任一有效帧即存活」会误判存活
        // 落入 waitDesktopConfirm 误等 120s（用户面对无解释的转圈）；识别
        // Notice 即结构化失败（原因码进 Failed，文案层引导回桌面重新生成）。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        var wsOpens = 0
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                wsOpens++
                // 首配帧序 HELLO + deviceInfo + pairingAuth（3 条 intro）读毕即拒绝
                openPumpedSession(
                    backgroundScope, deskPriv, readIntroFrameCount = 3,
                    rejectAfterIntro = """{"type":"pairingRejected","reason":"pairingWindowClosed"}""",
                )
            },
        )

        client.pairWithQr(validQr(deskPubHex))
        advanceTimeBy(15_000)

        val progress = client.pairingProgress.value
        assertTrue("probe 识别拒绝后必须 Failed（不得停留其他进度）", progress is PairingProgress.Failed)
        progress as PairingProgress.Failed
        assertEquals(
            "结构化拒绝原因必须进 Failed（文案层映射依据，SPEC §4）",
            PairingRejection.PairingWindowClosed,
            progress.rejection,
        )
        assertEquals(
            "拒绝即终点：不得周期重连等待桌面确认（否则 120s 内 wsOpens 会增长）",
            1,
            wsOpens,
        )
        assertFalse("被拒配对不得落库配对态", store.paired)
    }

    @Test
    fun `已配对会话收到结构化拒绝触发PairingRevoked恢复且停环`() = runTest {
        // WHY（SPEC qr-semantics §3.2）：桌面侧「移除设备」后手机重连——桌面
        // 先发拒绝 Notice 再关连接。现状手机无限退避重连（用户无感知配对已
        // 被解除，每轮都重建会话又被拒）；本地 paired=true 且公钥未变 →
        // PairingRevoked 恢复事件（清态 + 健康面 + 终止编排树）。
        val deskPriv = ByteArray(32) { (it + 1).toByte() }
        val deskPubHex = Hex.encode(NoiseChannel.deriveStaticPublicKeyForTest(deskPriv))
        val store = PairingStateStore(FakeSharedPreferences())
        store.save(deskPubHex, "1122334455667788", null)
        val recoveries = Collections.synchronizedList(mutableListOf<PairingRecoveryReason>())
        var wsOpens = 0
        val client = RealConnectionClient(
            scope = backgroundScope,
            store = store,
            secrets = FakeSecrets(),
            nsd = FakeNsd(),
            ioDispatcher = EmptyCoroutineContext,
            wsOpener = { _, _, _ ->
                wsOpens++
                openPumpedSession(
                    backgroundScope, deskPriv, readIntroFrameCount = 2,
                    rejectAfterIntro = """{"type":"pairingRejected","reason":"pairingWindowClosed"}""",
                )
            },
        )
        backgroundScope.launch { client.pairingRecovery.collect { recoveries.add(it) } }

        advanceTimeBy(2_000)
        assertEquals(
            "会话循环收到拒绝必须发布 PairingRevoked 恢复事件",
            listOf(PairingRecoveryReason.PairingRevoked),
            recoveries.toList(),
        )
        assertEquals("健康面必须亮 PairingRevoked（配对屏重配原因提示）", PairingHealth.PairingRevoked, client.pairingHealth.value)
        assertFalse("配对态必须清除（桌面已不认本机）", client.paired.value)
        assertFalse("清除必须落盘", store.paired)
        assertTrue("承载态如实 Degraded（不误报在线）", client.state.value is TransportStatus.Degraded)

        // 恢复即终点：编排协程树取消，不得继续退避重连（否则每轮再收拒绝）
        advanceTimeBy(30_000)
        assertEquals("恢复后不得继续重连被拒会话（编排树已终止）", 1, wsOpens)
    }
}
