package com.egosync.companion.connection

import android.content.Context
import android.os.Build
import com.egosync.companion.command.CommandChannel
import com.egosync.companion.command.CommandException
import com.egosync.companion.command.CommandSender
import com.egosync.companion.command.StreamCoordinator
import com.egosync.companion.pairing.QrPayload
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import okhttp3.OkHttpClient
import org.json.JSONObject
import kotlin.coroutines.CoroutineContext

/**
 * 真实连接客户端（Story 12.4 换装，AC1/AC3/AC4/AC5）：
 *
 * - 直连承载：NSD 发现 → WS → XX initiator 三条握手 → 信任锚校验（远端静态公钥
 *   == 桌面公钥，防中间人）→ HELLO → NOTICE(deviceInfo[, pairingAuth]) → 会话循环
 *   （30s PING 保活，桌面 120s 空闲断连留 4 倍余量）；
 * - 中继承载（[RelayClient]）：register(role=phone) → relay 鉴权 → 经转发通道
 *   跑同款 E2E 握手与帧序；
 * - [ConnectionStateMachine]：prefer-direct 滞回（直连丢失探测 3s 仍不可达才切
 *   中继；中继态持续重试直连，握手成功即切回）；双承载均失败 → Offline。
 *   评审 P1：状态机输出经收集接线进 [liveState]，UI 三态才真正可见。
 *
 * NFR-M7：本类不打印帧明文/密钥/QR 内容。
 *
 * 主构造为测试缝（P7：编排逻辑 JVM 可测）；生产装配走 [构造函数]，由
 * AppModelContainer 唯一换装点调用。
 */
class RealConnectionClient internal constructor(
    private val scope: CoroutineScope,
    internal val store: PairingStateStore,
    private val secrets: SecretsProvider,
    private val nsd: NsdDiscoverer,
    private val wsClient: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(CONNECT_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        // WS 控制层 PING：探测死链（桌面 tungstenite 自动 Pong）；app 层 PING 帧另有 30s 保活
        .pingInterval(WS_PING_INTERVAL_MS, TimeUnit.MILLISECONDS)
        .build(),
    relayClient: RelayClient? = null,
    /**
     * 密钥文件读写的调度上下文（P7 缝：生产 Dispatchers.IO；测试传
     * [kotlin.coroutines.EmptyCoroutineContext] 内联执行，虚时间调度才确定——
     * 真实 IO 线程池的续体回调会与 runTest 的 advanceTimeBy 产生竞态）。
     */
    private val ioDispatcher: CoroutineContext = Dispatchers.IO,
    private val wsOpener: suspend (String, OkHttpClient, Long) -> WsSession = ::openWsSession,
    /**
     * 快照帧消费者工厂（13.2 T3 注入缝）：每会话一个实例（残缺分帧序列不跨
     * 会话）；生产装配由 AppModelContainer 注入 SnapshotFrameHandler。
     */
    private val frameConsumerFactory: () -> FrameConsumer = { FrameConsumer.NOOP },
    /** 降级态快照信息提供者（13.2 T7）：Offline 状态的缓存存在性/数据截止时间。 */
    private val offlineInfo: () -> OfflineSnapshotInfo = { OfflineSnapshotInfo(false, null) },
    /**
     * 降级信息流源（13.2 评审 P7）：offlineInfo 为 pull 式闭包——缓存异步加载
     * 完成后需此流重发才驱动 combine 重算 Offline 值，冷启动不再误报「无缓存」。
     * 生产装配传 snapshotStore.state；默认恒 Unit（行为同旧版，测试免注入）。
     */
    private val offlineInfoTick: StateFlow<*> = MutableStateFlow(Unit),
    /**
     * 指令通道（13.3 T5）：出站指令经会话 pump 协程发送，入站
     * COMMAND_RESULT 完成对应 pending；null=指令通道不可用（fake 态）。
     */
    private val commandChannel: CommandChannel? = null,
    /** 流式聚合器（13.3 T5）：STREAM_TOKEN 分发至此。 */
    private val streamCoordinator: StreamCoordinator? = null,
) : ConnectionClient, PairingConnector, CommandSender {

    /** 生产装配：从 Android Context 组装全部依赖（AppModelContainer 唯一换装点，UX-M2）。 */
    constructor(
        context: Context,
        scope: CoroutineScope,
        frameConsumerFactory: () -> FrameConsumer = { FrameConsumer.NOOP },
        offlineInfo: () -> OfflineSnapshotInfo = { OfflineSnapshotInfo(false, null) },
        offlineInfoTick: StateFlow<*> = MutableStateFlow(Unit),
        commandChannel: CommandChannel? = null,
        streamCoordinator: StreamCoordinator? = null,
    ) : this(
        scope = scope,
        store = PairingStateStore(
            context.getSharedPreferences(PairingStateStore.PREFS_NAME, Context.MODE_PRIVATE),
        ),
        secrets = PairingSecrets(context),
        nsd = NsdDiscovery(context),
        frameConsumerFactory = frameConsumerFactory,
        offlineInfo = offlineInfo,
        offlineInfoTick = offlineInfoTick,
        commandChannel = commandChannel,
        streamCoordinator = streamCoordinator,
    )

    private val relay: RelayClient = relayClient ?: RelayClient(wsClient)

    private val liveState = MutableStateFlow<ConnectionState>(initialState())
    private val debugMode = MutableStateFlow<DebugConnectionMode?>(null)

    /** 裁决 7：Debug 档位覆盖状态输出（解除配对/进程重启复位自动）。 */
    override val state: StateFlow<ConnectionState> =
        // offlineInfoTick 入 combine（评审 P7）：缓存加载完成 → tick 重发 →
        // fillOffline/offlineState 重算，Offline 值不再滞留「无缓存」
        combine(liveState, debugMode, offlineInfoTick) { live, debug, _ ->
            debug?.let { mode ->
                when (mode) {
                    DebugConnectionMode.DIRECT -> ConnectionState.Direct
                    DebugConnectionMode.RELAY -> ConnectionState.Relay
                    DebugConnectionMode.OFFLINE ->
                        ConnectionState.Offline(snapshotAvailable = false, dataAsOf = null)
                    // 降级档呈现真实缓存态（有缓存才有数据截止时间，T7）
                    DebugConnectionMode.DEGRADED -> offlineState()
                }
            } ?: fillOffline(live)
        }.stateIn(scope, SharingStarted.Eagerly, liveState.value)

    private val _paired = MutableStateFlow(store.paired)
    override val paired: StateFlow<Boolean> = _paired.asStateFlow()

    private val _pairingProgress = MutableStateFlow<PairingProgress>(PairingProgress.Idle)
    override val pairingProgress: StateFlow<PairingProgress> = _pairingProgress.asStateFlow()

    private var sessionJob: Job? = null

    /** unpair 的异步擦除任务（P3：重配对前必须 join，防迟到擦除毁掉新配对）。 */
    private var wipeJob: Job? = null

    init {
        // 损坏配对态自愈必须先于任何协程启动同步完成：AppNavHost 的
        // startDestination 在首组合 remember{} 定格，晚了 paired 仍为 true，
        // 冷启动直落主界面＋离线遮罩（编排首轮即静默 return，永久死锁）
        healCorruptPairingIfAny()
        if (store.paired) {
            sessionJob = scope.launch { orchestrate() }
        }
    }

    // ── ConnectionClient ──────────────────────────────────────────────

    override fun setDebugMode(mode: DebugConnectionMode) {
        debugMode.value = mode
    }

    override fun completePairing() {
        _paired.value = true
    }

    override fun unpair() {
        sessionJob?.cancel()
        sessionJob = null
        nsd.stopDiscovery()
        debugMode.value = null
        // 13.3：解除配对后指令通道与流式态一并失效（pending 全失败、暂存快照丢弃）
        commandChannel?.shutdown()
        streamCoordinator?.reset()
        _paired.value = false
        _pairingProgress.value = PairingProgress.Idle
        liveState.value = ConnectionState.Direct
        // P3：擦除任务必须可追踪——pairWithQr 会先 join 它再生成新密钥
        wipeJob = scope.launch(Dispatchers.IO) {
            secrets.wipe()
            store.clear()
        }
    }

    /**
     * 冷启动自愈（覆盖安装遗留态）：paired=true 但 relayId/desktopPubkeyHex
     * 缺失或为空时，编排首轮即静默 return（runDirectLoop 取不到 relayId），叠加
     * 离线遮罩无应用内出口即永久死锁——清配对回未配对，冷启动直接落配对扫码流
     * （FR-40 重装重扫即恢复）。配对元数据同步清除；密钥文件异步擦除（镜像
     * [unpair] 的 wipeJob 模式，目标场景通常无密钥文件，wipe 为无害空操作）；
     * 快照缓存与通知清理属容器层 unpair（遮罩出口路径）职责，客户端层不可达
     * （目标场景从未建立会话，无缓存可清——评审裁定记录于规格变更日志）。
     * 速记队列属容器层，不在此清除（FR-43 无丢失）；NFR-M7：日志不含任何
     * 密钥材料。仅在 [init] 同步段调用（此时编排尚未启动，无需取消会话）。
     */
    private fun healCorruptPairingIfAny() {
        if (!(store.paired && (store.relayId.isNullOrBlank() || store.desktopPubkeyHex.isNullOrBlank()))) return
        CompanionLog.warn("Connection", "检测到损坏的配对态（配对元数据不完整），已自动清除，请重新扫码配对")
        nsd.stopDiscovery()
        debugMode.value = null
        commandChannel?.shutdown()
        streamCoordinator?.reset()
        _paired.value = false
        _pairingProgress.value = PairingProgress.Idle
        // 未配对态沿用 initialState() 既有语义（Direct，遮罩自然不显示）
        liveState.value = ConnectionState.Direct
        store.clear()
        wipeJob = scope.launch(Dispatchers.IO) { secrets.wipe() }
    }

    // ── CommandSender（13.3 T5）─────────────────────────────────────

    override val sessionActive: Boolean
        get() = commandChannel?.sessionActive == true

    override suspend fun execute(action: String, paramsJson: String): org.json.JSONObject =
        commandChannel?.execute(action, paramsJson)
            ?: throw CommandException("ConnectionError", "桌面引擎不可达")

    // ── PairingConnector ──────────────────────────────────────────────

    override fun pairWithQr(qrJson: String) {
        val payload = QrPayload.parse(qrJson)
        if (payload == null) {
            _pairingProgress.value = PairingProgress.Failed("二维码无效，请在桌面端重新生成")
            return
        }
        sessionJob?.cancel()
        sessionJob = scope.launch {
            // P3：上一次 unpair 的异步擦除可能尚未完成——join 后再生成新密钥/
            // 落新配对元数据，防止迟到的 wipe 把新配对连锅端掉
            wipeJob?.let { runCatching { it.join() } }
            runPairing(payload)
            // 配对成功并经历一次会话后，交由双承载编排维持在线
            if (_pairingProgress.value == PairingProgress.Success && store.paired) {
                orchestrate()
            }
        }
    }

    // ── 首次配对（直连路径，AC）──────────────────────────────────────

    private suspend fun runPairing(payload: QrPayload) {
        var phase = "discovery"
        try {
            _pairingProgress.value = PairingProgress.DiscoveringDevice
            nsd.discover("EgoSync-${payload.relayId.take(8)}")
            val endpoint = nsd.awaitResolved(NSD_WAIT_MS)

            phase = "connect"
            _pairingProgress.value = PairingProgress.ExchangingKeys
            // 握手+信任锚+帧序一体建立；信任锚拒绝在此处即关会话（防半开泄漏）
            val session = connectDirect(
                endpoint,
                expectedDesktopPubHex = payload.desktopStaticPubkey,
                needsPairingAuth = true,
                pairingNonce = payload.pairingNonce,
            )

            phase = "verify"
            _pairingProgress.value = PairingProgress.VerifyingIdentity

            // 探测会话是否真正建立：桌面若判定换绑 pending 会关闭连接（手机侧
            // 无法从握手结果区分），以 PING/回帧探测是唯一可靠信号。
            if (!probeSessionAlive(session)) {
                session.close()
                CompanionLog.info("Connection", "配对请求已提交，等待桌面确认换绑")
                waitDesktopConfirmAndRetry(payload)
                return
            }

            finishPairing(payload, session)
        } catch (e: SecretsInvalidatedException) {
            // P14：Keystore 失效/密文损坏 → 自愈清配对态，绝不无限重试
            handleSecretsInvalidated(machine = null)
            _pairingProgress.value = PairingProgress.Failed("本机配对密钥已失效，请重新扫码配对")
        } catch (e: CancellationException) {
            if (!currentCoroutineContext().isActive) throw e
            onPairingFailed(phase, e)
        } catch (e: Exception) {
            onPairingFailed(phase, e)
        }
    }

    /** 配对完成收尾：落盘信任锚 + 进度 Success + 转入会话循环。 */
    private suspend fun finishPairing(payload: QrPayload, session: WsSession) {
        _paired.value = true
        store.save(payload.desktopStaticPubkey, payload.relayId, payload.relayAddr)
        _pairingProgress.value = PairingProgress.Success
        liveState.value = ConnectionState.Direct
        nsd.stopDiscovery()
        CompanionLog.info("Connection", "配对成功，直连会话建立")
        startSessionLoop(session)
    }

    /**
     * 换绑等待（桌面 PENDING_PAIRING_TIMEOUT_SECS=120s 对齐）：周期重连免 nonce
     * （pending 匹配放行）；桌面确认后同公钥即 AlreadyPaired 直接进入会话；
     * 超时如实 Failed。重连期间进度维持 WaitDesktopConfirm（UI 呈现等待文案）。
     */
    private suspend fun waitDesktopConfirmAndRetry(payload: QrPayload) {
        _pairingProgress.value = PairingProgress.WaitDesktopConfirm
        val deadline = System.currentTimeMillis() + WAIT_CONFIRM_TIMEOUT_MS
        while (currentCoroutineContext().isActive && System.currentTimeMillis() < deadline) {
            delay(WAIT_CONFIRM_RETRY_MS)
            try {
                nsd.discover("EgoSync-${payload.relayId.take(8)}")
                val endpoint = nsd.awaitResolved(NSD_WAIT_MS)
                // nonce 已消费：pending 匹配（免 nonce）或已确认（AlreadyPaired）均放行
                val session = connectDirect(
                    endpoint,
                    expectedDesktopPubHex = payload.desktopStaticPubkey,
                    needsPairingAuth = false,
                    pairingNonce = null,
                )
                if (probeSessionAlive(session)) {
                    finishPairing(payload, session)
                    return
                }
                // 仍未确认：桌面再次关闭连接，继续等待
                session.close()
            } catch (e: SecretsInvalidatedException) {
                handleSecretsInvalidated(machine = null)
                _pairingProgress.value = PairingProgress.Failed("本机配对密钥已失效，请重新扫码配对")
                return
            } catch (e: CancellationException) {
                if (!currentCoroutineContext().isActive) throw e
            } catch (_: Exception) {
                // 桌面暂不可达：等待下一轮
            } finally {
                nsd.stopDiscovery()
            }
        }
        CompanionLog.warn("Connection", "等待桌面确认换绑超时")
        _pairingProgress.value = PairingProgress.Failed("等待桌面端确认超时，请重新扫码")
        liveState.value = offlineState()
    }

    /**
     * 会话存活探测：发 PING 后在预算内收到任一有效帧即认为已进入会话
     * （桌面会话循环以 Ping 帧应答）；通道关闭/超时 → false。
     */
    private suspend fun probeSessionAlive(session: WsSession): Boolean {
        if (!session.send(FrameCodec.encode(Frame.Ping, session.transport))) return false
        return try {
            val bytes = withTimeoutOrNull(PROBE_TIMEOUT_MS) { session.incoming.receive() }
            bytes != null && runCatching { FrameCodec.decode(bytes, session.transport) }.isSuccess
        } catch (_: Exception) {
            false
        }
    }

    private fun onPairingFailed(phase: String, e: Exception) {
        val message = when {
            e is IdentityVerificationException -> "桌面身份校验失败，已断开连接"
            phase == "discovery" && (e is TimeoutCancellationException || e is IOException) ->
                // P13：发现/解析失败现以 IOException 快速上抛，与超时同文案呈现
                "未发现桌面设备，请确认与桌面端在同一网络"
            else -> "配对未完成，请在桌面端重新生成二维码后重试"
        }
        _pairingProgress.value = PairingProgress.Failed(message)
        liveState.value = offlineState()
        nsd.stopDiscovery()
    }

    /** P14 自愈：清配对态回未配对（重装重扫路径），机器在线状态如实 Offline。 */
    private fun handleSecretsInvalidated(machine: ConnectionStateMachine?) {
        store.clear()
        _paired.value = false
        nsd.stopDiscovery()
        if (machine != null) {
            machine.goOffline()
        } else {
            liveState.value = offlineState()
        }
        CompanionLog.warn("Connection", "本机配对密钥已失效，已清除配对态，请重新扫码配对")
    }

    // ── 双承载编排（状态机驱动）─────────────────────────────────────

    /** 直连环 + 中继环并行，状态与滞回决策归 [ConnectionStateMachine]。 */
    private suspend fun orchestrate() = kotlinx.coroutines.coroutineScope {
        val machine = ConnectionStateMachine(
            scope = this,
            hysteresisMs = HYSTERESIS_MS,
        )
        val relaySignal = Channel<Unit>(capacity = Channel.CONFLATED)
        machine.onIntent { intent ->
            when (intent) {
                is ConnectionStateMachine.Intent.StartRelay -> relaySignal.trySend(Unit)
            }
        }
        // P1：状态机输出接线——machine.state 收集进 liveState，UI 订阅的三态
        // 才真正随承载变化（此前 machine 为编排局部实例，输出被整体丢弃）；
        // T7：Offline 值经 fillOffline 填充真实缓存存在性/数据截止时间
        launch { machine.state.collect { liveState.value = fillOffline(it) } }
        // P17：relayAddr 不再启动时快照固化——runRelayLoop 每次尝试现读
        val relayLoop = if (store.relayAddr != null) {
            launch { runRelayLoop(machine, relaySignal) }
        } else {
            null
        }
        runDirectLoop(machine)
        relayLoop?.cancel()
    }

    private suspend fun runDirectLoop(machine: ConnectionStateMachine) {
        val backoff = BackoffPolicy()
        while (currentCoroutineContext().isActive && store.paired) {
            val relayId = store.relayId ?: return
            try {
                nsd.discover("EgoSync-${relayId.take(8)}")
                val endpoint = nsd.awaitResolved(NSD_WAIT_MS)
                val session = connectDirect(
                    endpoint,
                    expectedDesktopPubHex = store.desktopPubkeyHex,
                    needsPairingAuth = false,
                    pairingNonce = null,
                )
                machine.onDirectEstablished()
                backoff.reset()
                CompanionLog.info("Connection", "直连会话建立")
                startSessionLoop(session)
                machine.onDirectLost(store.relayAddr != null)
                CompanionLog.info("Connection", "直连会话结束")
            } catch (e: SecretsInvalidatedException) {
                handleSecretsInvalidated(machine)
                return
            } catch (e: TimeoutCancellationException) {
                // P1：NSD/连接超时是 CancellationException 子类——被下面的
                // CancellationException 分支吞掉时状态机永远收不到 onDirectLost，
                // UI 停在 Direct 误报在线（AC5 击穿）。必须在此通知。
                machine.onDirectLost(store.relayAddr != null)
            } catch (e: CancellationException) {
                if (!currentCoroutineContext().isActive) throw e
            } catch (_: Exception) {
                machine.onDirectLost(store.relayAddr != null)
            }
            nsd.stopDiscovery()
            delay(backoff.nextDelayMs())
        }
    }

    private suspend fun runRelayLoop(
        machine: ConnectionStateMachine,
        signal: Channel<Unit>,
    ) {
        val backoff = BackoffPolicy()
        while (currentCoroutineContext().isActive && store.paired) {
            signal.receive() // 仅滞回窗满（直连确认不可达）后被请求
            // P17：每次尝试现读配置——陈旧地址不再固化在编排启动时
            val relayAddr = store.relayAddr ?: return
            val relayId = store.relayId ?: return
            var established = false
            try {
                val staticPrivate = withContext(ioDispatcher) { secrets.loadOrCreateStaticPrivateKey() }
                val session = relay.connect(relayAddr, relayId, staticPrivate)
                try {
                    e2eHandshake(session, staticPrivate)
                    verifyTrustAnchor(session, store.desktopPubkeyHex)
                    sendIntroFrames(session, needsPairingAuth = false, pairingNonce = null)
                    machine.onRelayEstablished()
                    established = true
                    backoff.reset()
                    CompanionLog.info("Connection", "中继会话建立")
                    coroutineScope {
                        val sessionJob = launch { startSessionLoop(session) }
                        // P5：直连恢复即拆除中继会话（单会话语义，防双活会话
                        // 并存——13.x 数据帧出现后将导致重复投递）
                        val directWatcher = launch {
                            machine.state.first { it is ConnectionState.Direct }
                            sessionJob.cancelAndJoin()
                        }
                        sessionJob.join()
                        directWatcher.cancel()
                    }
                } finally {
                    // P5 附带修复：握手/信任锚失败路径此前不关会话（连接泄漏）
                    session.close()
                }
            } catch (e: SecretsInvalidatedException) {
                handleSecretsInvalidated(machine)
                return
            } catch (e: TimeoutCancellationException) {
                onRelayAttemptEnded(machine, established)
            } catch (e: CancellationException) {
                if (!currentCoroutineContext().isActive) throw e
                onRelayAttemptEnded(machine, established)
            } catch (_: Exception) {
                onRelayAttemptEnded(machine, established)
            }
            delay(backoff.nextDelayMs())
        }
    }

    /** 中继尝试收尾：建立过 → onRelayLost；从未建立 → onRelayEstablishmentFailed（P2）。 */
    private fun onRelayAttemptEnded(machine: ConnectionStateMachine, established: Boolean) {
        if (established) machine.onRelayLost() else machine.onRelayEstablishmentFailed()
    }

    // ── 共享协议序列（直连/中继同构）────────────────────────────────

    private class IdentityVerificationException : Exception()

    /** 信任锚：远端静态公钥必须等于期望桌面公钥（不等即断开，防中间人）。 */
    private fun verifyTrustAnchor(session: WsSession, expectedHex: String?) {
        val expected = expectedHex ?: throw IdentityVerificationException()
        val actualHex = Hex.encode(session.channel.remoteStaticPublicKey())
        if (actualHex != expected.lowercase()) {
            throw IdentityVerificationException()
        }
    }

    /**
     * 直连承载完整建立（AC）：NSD 端点 WS → XX initiator 握手 → 信任锚校验 →
     * 帧序。任一步失败即关闭会话再上抛——信任锚拒绝的冒名会话不得遗留半开
     * 连接（评审 B 用例暴露的泄漏：校验原本在 close 守护之外）。
     */
    private suspend fun connectDirect(
        endpoint: NsdEndpoint,
        expectedDesktopPubHex: String?,
        needsPairingAuth: Boolean,
        pairingNonce: String?,
    ): WsSession {
        val staticPrivate = withContext(ioDispatcher) { secrets.loadOrCreateStaticPrivateKey() }
        val session = withTimeout(CONNECT_TIMEOUT_MS) {
            wsOpener("ws://${endpoint.host}:${endpoint.port}", wsClient, CONNECT_TIMEOUT_MS)
        }
        try {
            e2eHandshake(session, staticPrivate)
            verifyTrustAnchor(session, expectedDesktopPubHex)
            sendIntroFrames(session, needsPairingAuth = needsPairingAuth, pairingNonce = pairingNonce)
            return session
        } catch (e: Throwable) {
            session.close()
            throw e
        }
    }

    /** E2E XX initiator 三条握手（m1 出 → m2 入 → m3 出），每步超时预算（P4）。 */
    private suspend fun e2eHandshake(session: WsSession, staticPrivate: ByteArray) {
        val channel = NoiseChannel.initiator(staticPrivate)
        // P12：send=false 即套接字已死——立即失败，不等 10s 步超时才暴露
        if (!session.send(channel.writeHandshakeMessage())) throw IOException("握手消息发送失败")
        val m2 = withTimeout(HANDSHAKE_STEP_MS) { session.incoming.receive() }
        channel.readHandshakeMessage(m2)
        if (!session.send(channel.writeHandshakeMessage())) throw IOException("握手消息发送失败")
        session.channel = channel
        session.transport = channel.split()
    }

    /** 握手后应用层帧序：HELLO → NOTICE(deviceInfo) →（新配对）NOTICE(pairingAuth)。 */
    private fun sendIntroFrames(session: WsSession, needsPairingAuth: Boolean, pairingNonce: String?) {
        if (!session.send(FrameCodec.encode(Frame.Hello(FrameCodec.PROTOCOL_VERSION), session.transport))) {
            throw IOException("HELLO 发送失败")
        }
        val deviceInfo = JSONObject()
            .put("type", "deviceInfo")
            .put("deviceName", Build.MODEL ?: "Android")
            .toString()
        if (!session.send(FrameCodec.encode(Frame.Notice(deviceInfo), session.transport))) {
            throw IOException("NOTICE 发送失败")
        }
        if (needsPairingAuth && pairingNonce != null) {
            // 3s 窗口内必须到达（APP_NOTICE_WINDOW_SECS）：紧随 HELLO 立即发送
            val auth = JSONObject()
                .put("type", "pairingAuth")
                .put("nonce", pairingNonce)
                .toString()
            if (!session.send(FrameCodec.encode(Frame.Notice(auth), session.transport))) {
                throw IOException("NOTICE 发送失败")
            }
        }
    }

    /** 会话循环：30s 无入站即发 PING 保活；协议错误/通道关闭即退出（上层进退避）。 */
    private suspend fun startSessionLoop(session: WsSession) = kotlinx.coroutines.coroutineScope {
        // T3：每会话一个帧消费者——承载切换/重连时会话循环重建，残缺分帧序列不跨会话
        val frameConsumer = frameConsumerFactory()
        // 13.3 T5：指令出站 pump（每会话绑定一次；pending 表跨会话存活）
        val commandBinding = commandChannel?.bind()
        val pump = commandBinding?.let { binding ->
            launch {
                for (frame in binding.outbound) {
                    // 发送失败即 pump 退出——在途指令由 unbind 失败或看门狗兜底
                    if (!session.send(FrameCodec.encode(frame, session.transport))) break
                }
            }
        }
        try {
            while (currentCoroutineContext().isActive) {
                val bytes = withTimeoutOrNull(PING_INTERVAL_MS) { session.incoming.receive() }
                if (bytes == null) {
                    if (session.incoming.isClosedForReceive) break
                    if (!session.send(FrameCodec.encode(Frame.Ping, session.transport))) break
                    continue
                }
                val frame = try {
                    FrameCodec.decode(bytes, session.transport)
                } catch (e: FrameCodec.FrameCodecException) {
                    break // 协议错误：断开（AC：状态回到 Offline，不误报）
                }
                when (frame) {
                    // 桌面 PONG 亦为 Ping 帧（companion_connection.rs：PING 应答发送 Ping）
                    is Frame.Ping -> Unit
                    is Frame.Hello -> Unit
                    is Frame.Notice -> Unit
                    // 13.2 T3：快照帧交注入的消费者（载荷级错误由消费者内部吞掉，
                    // 不杀会话——下一个全量序列自愈）
                    // 评审 P6：解析移出主线程——本循环跑在 scope（Main.immediate）上，
                    // 10MB 快照的 base64 解码/拼接/JSON 解析在主线程会掉帧乃至 ANR
                    is Frame.Snapshot -> withContext(ioDispatcher) { frameConsumer.onFrame(frame) }
                    is Frame.StateDelta -> withContext(ioDispatcher) { frameConsumer.onFrame(frame) }
                    // 13.3 T5：指令帧分发（pending 表跨会话存活，故消费者不随会话重建）
                    is Frame.CommandResult -> withContext(ioDispatcher) {
                        commandChannel?.onCommandResult(frame.data)
                    }
                    is Frame.StreamToken -> withContext(ioDispatcher) {
                        streamCoordinator?.onStreamToken(frame.data)
                    }
                    // 手机不接收 COMMAND 帧（桌面→手机不发起指令）；防御性忽略
                    is Frame.Command -> Unit
                }
            }
        } catch (e: CancellationException) {
            session.close()
            throw e
        } catch (_: Exception) {
            // 通道关闭/收发失败：会话终止
        } finally {
            pump?.cancel()
            commandBinding?.let { commandChannel?.unbind(it) }
            // 会话终止清流式门（评审 C10）：闪断重连时活跃流残留会让重连后的
            // 所有快照（含首帧全量）无限暂存，UI 冻结在断连前陈旧数据上——
            // reset 同时 flush 暂存快照，流式状态由 VM 侧收口为「连接中断」
            streamCoordinator?.reset()
            session.close()
        }
    }

    private fun initialState(): ConnectionState =
        if (store.paired) {
            // 启动即未连上——如实 Offline（AC5：不误报在线），缓存存在性经 T7 填充
            offlineState()
        } else {
            ConnectionState.Direct
        }

    /** Offline 态填充真实缓存存在性/数据截止时间（T7：原硬编码 (false, null)）。 */
    private fun offlineState(): ConnectionState {
        val info = offlineInfo()
        return ConnectionState.Offline(
            snapshotAvailable = info.snapshotAvailable,
            dataAsOf = info.dataAsOf,
        )
    }

    /** 非 Offline 态原样透传；Offline 态填真实缓存值。 */
    private fun fillOffline(state: ConnectionState): ConnectionState =
        if (state is ConnectionState.Offline) offlineState() else state

    private companion object {
        const val NSD_WAIT_MS = 12_000L
        const val CONNECT_TIMEOUT_MS = 10_000L
        const val HANDSHAKE_STEP_MS = 10_000L
        const val PING_INTERVAL_MS = 30_000L
        const val WS_PING_INTERVAL_MS = 15_000L
        const val HYSTERESIS_MS = 3_000L

        /** 配对会话存活探测预算（桌面应答 PING 即视为进入会话）。 */
        const val PROBE_TIMEOUT_MS = 4_000L

        /** 换绑等待总预算（对齐桌面 PENDING_PAIRING_TIMEOUT_SECS=120s）与重连间隔。 */
        const val WAIT_CONFIRM_TIMEOUT_MS = 120_000L
        const val WAIT_CONFIRM_RETRY_MS = 3_000L
    }
}
