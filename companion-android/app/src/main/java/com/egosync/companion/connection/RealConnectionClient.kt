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
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.channels.BufferOverflow
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

    private val liveState = MutableStateFlow<TransportStatus>(initialState())
    private val debugMode = MutableStateFlow<DebugConnectionMode?>(null)

    /** 裁决 7：Debug 档位覆盖状态输出（解除配对/进程重启复位自动）。 */
    override val state: StateFlow<TransportStatus> =
        // offlineInfoTick 入 combine（评审 P7）：缓存加载完成 → tick 重发 →
        // fillDegraded/degradedState 重算，Degraded 值不再滞留「无缓存」
        combine(liveState, debugMode, offlineInfoTick) { live, debug, _ ->
            debug?.let { mode ->
                when (mode) {
                    DebugConnectionMode.DIRECT -> TransportStatus.Direct
                    DebugConnectionMode.RELAY -> TransportStatus.Relay
                    DebugConnectionMode.CONNECTING -> TransportStatus.Connecting
                    DebugConnectionMode.RECONNECTING -> TransportStatus.Reconnecting
                    DebugConnectionMode.OFFLINE ->
                        TransportStatus.Degraded(snapshotAvailable = false, dataAsOf = null)
                    // 降级档呈现真实缓存态（有缓存才有数据截止时间，T7）
                    DebugConnectionMode.DEGRADED -> degradedState()
                }
            } ?: fillDegraded(live)
        }.stateIn(scope, SharingStarted.Eagerly, liveState.value)

    private val _paired = MutableStateFlow(store.paired)
    override val paired: StateFlow<Boolean> = _paired.asStateFlow()

    /** 健康面（T-S3 落类型）：真实失效信号（凭据/信任/授权）T-S4 接线。 */
    private val _pairingHealth = MutableStateFlow<PairingHealth>(PairingHealth.Ok)
    override val pairingHealth: StateFlow<PairingHealth> = _pairingHealth.asStateFlow()

    /** 恢复事件（T-S4）：一次性，运行中失效发布（冷启动走健康态，见 [PairingRecoveryReason]）。 */
    private val _pairingRecovery = MutableSharedFlow<PairingRecoveryReason>(
        extraBufferCapacity = 8,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    override val pairingRecovery: SharedFlow<PairingRecoveryReason> = _pairingRecovery.asSharedFlow()

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
        _pairingHealth.value = PairingHealth.Ok
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
        _pairingHealth.value = PairingHealth.Ok
        liveState.value = TransportStatus.Direct
        // P3：擦除任务必须可追踪——pairWithQr 会先 join 它再生成新密钥
        wipeJob = scope.launch(Dispatchers.IO) {
            secrets.wipe()
            store.clear()
        }
    }

    /**
     * 冷启动自愈（覆盖安装遗留态）：
     * - 配对元数据不完整（paired=true 但 relayId/desktopPubkeyHex 缺失或为空）：
     *   编排首轮即静默 return，叠加离线遮罩无应用内出口即永久死锁——清配对回
     *   未配对，冷启动直接落配对扫码流（FR-40 重装重扫即恢复）。
     * - T-S4（SPEC state-model §5.1）：paired=true 但包裹私钥文件缺失（重装/
     *   系统数据清理残留）——不得静默生成新身份顶替（配对信任链是桌面公钥
     *   记录 ↔ 本机私钥的配对，换身份必须重新扫码）：如实清配对态 + 健康面亮
     *   [PairingHealth.CredentialMissing] 供配对屏展示原因；冷启动无订阅者在
     *   跑，一次性恢复事件不发布（健康态承载原因）。
     * 两条路径均：配对元数据同步清除；密钥文件异步擦除（镜像 [unpair] 的
     * wipeJob 模式，目标场景通常无密钥文件，wipe 为无害空操作）；快照缓存与
     * 通知清理属容器层 unpair（遮罩出口路径）职责，客户端层不可达（目标场景
     * 从未建立会话，无缓存可清——评审裁定记录于规格变更日志）。离线待发箱
     * 属容器层，不在此清除（FR-43 无丢失）；NFR-M7：日志不含任何密钥材料。
     * 仅在 [init] 同步段调用（此时编排尚未启动，无需取消会话）。
     */
    private fun healCorruptPairingIfAny() {
        if (!store.paired) return
        val metadataCorrupt = store.relayId.isNullOrBlank() || store.desktopPubkeyHex.isNullOrBlank()
        val keyMissing = !metadataCorrupt && !secrets.hasStaticPrivateKey()
        if (!metadataCorrupt && !keyMissing) return
        if (keyMissing) {
            CompanionLog.warn("Connection", "已配对但本机密钥缺失，清配对态回重扫路径（不静默换身份）")
        } else {
            CompanionLog.warn("Connection", "检测到损坏的配对态（配对元数据不完整），已自动清除，请重新扫码配对")
        }
        nsd.stopDiscovery()
        debugMode.value = null
        commandChannel?.shutdown()
        streamCoordinator?.reset()
        _paired.value = false
        _pairingProgress.value = PairingProgress.Idle
        if (keyMissing) _pairingHealth.value = PairingHealth.CredentialMissing
        // 未配对态沿用 initialState() 既有语义（Direct，指示条不显示）
        liveState.value = TransportStatus.Direct
        store.clear()
        wipeJob = scope.launch(Dispatchers.IO) { secrets.wipe() }
    }

    // ── CommandSender（13.3 T5）─────────────────────────────────────

    /**
     * 会话/就绪面（T-S2）：转发 [CommandChannel.sessionActive]——bind/unbind/
     * shutdown 同步发布；无指令通道（fake 态）恒 false。写操作判据唯一事实源。
     */
    override val sessionActive: StateFlow<Boolean> =
        commandChannel?.sessionActive ?: MutableStateFlow(false)

    /** ConnectionClient 侧就绪面（同一事实源的两个接口投影）。 */
    override val commandReady: StateFlow<Boolean>
        get() = sessionActive

    override suspend fun execute(action: String, paramsJson: String, commandId: String?): org.json.JSONObject =
        commandChannel?.execute(action, paramsJson, commandId)
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

    // ── 首次配对（直连优先，中继回退，Story 12.5 AC1）────────────────

    private suspend fun runPairing(payload: QrPayload) {
        var phase = "discovery"
        var viaRelay = false
        // T-S4：配对路径显式生成新身份（此前 loadOrCreate 会在已配对残态上静默
        // 复用/顶替，身份边界含糊）；等待桌面确认期间复用同一身份（pending 匹配
        // 以首次提交的公钥为准，重试再生成即失配）
        val pairingStaticPrivate = withContext(ioDispatcher) { secrets.createForPairingStaticPrivateKey() }
        try {
            _pairingProgress.value = PairingProgress.DiscoveringDevice
            val session = try {
                nsd.discover("EgoSync-${payload.relayId.take(8)}")
                val endpoint = nsd.awaitResolved(NSD_WAIT_MS)
                phase = "connect"
                _pairingProgress.value = PairingProgress.ExchangingKeys
                // 握手+信任锚+帧序一体建立；信任锚拒绝在此处即关会话（防半开泄漏）
                connectDirect(
                    endpoint,
                    expectedDesktopPubHex = payload.desktopStaticPubkey,
                    needsPairingAuth = true,
                    pairingNonce = payload.pairingNonce,
                    staticPrivate = pairingStaticPrivate,
                )
            } catch (e: Exception) {
                // NSD 局域网发现失败/超时且 QR 携带 relayAddr → 回退中继完成首配
                //（顺序回退：NSD 先、12s 预算后转中继，镜像连接期 prefer-direct）。
                // phase 维持 "discovery"：中继也失败时文案如实合并（发现+中继双失败）。
                // 真实取消（协程已失活）与非发现类失败原样上抛，不吞不降级。
                val discoveryFailure = (e is TimeoutCancellationException || e is IOException) &&
                    currentCoroutineContext().isActive
                if (payload.relayAddr == null || !discoveryFailure) throw e
                // NFR-M7：只记 relayId 截断前缀，不打印地址全量/QR 内容
                CompanionLog.info("Connection", "局域网未发现桌面，回退中继配对（relay=${payload.relayId.take(8)}…）")
                _pairingProgress.value = PairingProgress.ExchangingKeys
                viaRelay = true
                connectViaRelay(payload, needsPairingAuth = true, pairingNonce = payload.pairingNonce, staticPrivate = pairingStaticPrivate)
                    .also { CompanionLog.info("Connection", "中继配对通道已建立") }
            }

            phase = "verify"
            _pairingProgress.value = PairingProgress.VerifyingIdentity

            // 探测会话是否真正建立：桌面若判定待确认（换绑或中继首配 pending，
            // 12.5 AC2）会关闭连接（手机侧无法从握手结果区分），以 PING/回帧
            // 探测是唯一可靠信号。
            // T-S5（SPEC qr-semantics §3.2）：桌面拒绝点先发结构化 Notice 再关
            // 连接——probe 识别即结构化失败，绝不落入 waitDesktopConfirm
            //（旧 QR 场景现状会误等桌面确认 120s）。
            when (val probe = probeSession(session)) {
                is ProbeResult.Rejected -> {
                    session.close()
                    _pairingProgress.value = PairingProgress.Failed(rejection = probe.rejection)
                    return
                }
                ProbeResult.Alive -> finishPairing(payload, session, viaRelay)
                ProbeResult.Dead -> {
                    session.close()
                    CompanionLog.info("Connection", "配对请求已提交，等待桌面确认")
                    waitDesktopConfirmAndRetry(payload, pairingStaticPrivate)
                    return
                }
            }
        } catch (e: CancellationException) {
            if (!currentCoroutineContext().isActive) throw e
            onPairingFailed(phase, e, payload.relayAddr != null)
        } catch (e: Exception) {
            onPairingFailed(phase, e, payload.relayAddr != null)
        }
    }

    /** 配对完成收尾：落盘信任锚 + 进度 Success + 转入会话循环（承载态如实标记）。 */
    private suspend fun finishPairing(payload: QrPayload, session: WsSession, viaRelay: Boolean) {
        _paired.value = true
        _pairingHealth.value = PairingHealth.Ok
        store.save(payload.desktopStaticPubkey, payload.relayId, payload.relayAddr)
        _pairingProgress.value = PairingProgress.Success
        // T-S2 时序不变量（state-model §4.2）：bind（commandReady=true）先于承载态发布
        val commandBinding = commandChannel?.bind()
        liveState.value = if (viaRelay) TransportStatus.Relay else TransportStatus.Direct
        nsd.stopDiscovery()
        CompanionLog.info("Connection", "配对成功，${if (viaRelay) "中继" else "直连"}会话建立")
        startSessionLoop(session, commandBinding)
    }

    /**
     * 等待桌面确认（换绑或中继首配 pending，120s 与桌面超时对齐）：周期重连免
     * nonce（pending 匹配放行）；桌面确认后同公钥即 AlreadyPaired 直接进入会话；
     * 超时如实 Failed。QR 携带 relayAddr 时重连走中继（Story 12.5 AC3），NSD
     * 直连重试保留（relayAddr 为空时）。重连期间进度维持 WaitDesktopConfirm。
     */
    private suspend fun waitDesktopConfirmAndRetry(payload: QrPayload, staticPrivate: ByteArray) {
        _pairingProgress.value = PairingProgress.WaitDesktopConfirm
        val deadline = System.currentTimeMillis() + WAIT_CONFIRM_TIMEOUT_MS
        while (currentCoroutineContext().isActive && System.currentTimeMillis() < deadline) {
            delay(WAIT_CONFIRM_RETRY_MS)
            var viaRelay = false
            try {
                val session = if (payload.relayAddr != null) {
                    viaRelay = true
                    connectViaRelay(payload, needsPairingAuth = false, pairingNonce = null, staticPrivate = staticPrivate)
                } else {
                    nsd.discover("EgoSync-${payload.relayId.take(8)}")
                    val endpoint = nsd.awaitResolved(NSD_WAIT_MS)
                    connectDirect(
                        endpoint,
                        expectedDesktopPubHex = payload.desktopStaticPubkey,
                        needsPairingAuth = false,
                        pairingNonce = null,
                        staticPrivate = staticPrivate,
                    )
                }
                when (val probe = probeSession(session)) {
                    is ProbeResult.Rejected -> {
                        // T-S5：等待期桌面窗口关闭/码被消费——结构化失败即终点，
                        // 不再周期重连（下一轮也只会再收拒绝）
                        session.close()
                        _pairingProgress.value = PairingProgress.Failed(rejection = probe.rejection)
                        return
                    }
                    ProbeResult.Alive -> {
                        finishPairing(payload, session, viaRelay)
                        return
                    }
                    ProbeResult.Dead -> Unit
                }
                // 仍未确认：桌面再次关闭连接，继续等待
                session.close()
            } catch (e: CancellationException) {
                if (!currentCoroutineContext().isActive) throw e
            } catch (_: Exception) {
                // 桌面暂不可达：等待下一轮
            } finally {
                nsd.stopDiscovery()
            }
        }
        CompanionLog.warn("Connection", "等待桌面确认配对超时")
        _pairingProgress.value = PairingProgress.Failed("等待桌面端确认超时，请重新扫码")
        liveState.value = degradedState()
    }

    /** probe 结果（T-S5）：存活 / 通道死 / 桌面结构化拒绝（先 Notice 后关闭）。 */
    private sealed interface ProbeResult {
        data object Alive : ProbeResult
        data object Dead : ProbeResult
        data class Rejected(val rejection: PairingRejection) : ProbeResult
    }

    /**
     * 会话存活探测：发 PING 后在预算内收到任一有效帧即认为已进入会话
     * （桌面会话循环以 Ping 帧应答）；通道关闭/超时 → [ProbeResult.Dead]。
     * T-S5：首帧若为 pairingRejected Notice → [ProbeResult.Rejected]（桌面
     * 拒绝点先发 Notice 再关连接；旧桌面不发 Notice，行为回退现状）。
     */
    private suspend fun probeSession(session: WsSession): ProbeResult {
        if (!session.send(FrameCodec.encode(Frame.Ping, session.transport))) return ProbeResult.Dead
        return try {
            val bytes = withTimeoutOrNull(PROBE_TIMEOUT_MS) { session.incoming.receive() }
                ?: return ProbeResult.Dead
            val frame = runCatching { FrameCodec.decode(bytes, session.transport) }
                .getOrElse { return ProbeResult.Dead }
            val notice = frame as? Frame.Notice ?: return ProbeResult.Alive
            PairingRejection.fromNoticeData(notice.data)?.let { ProbeResult.Rejected(it) }
                ?: ProbeResult.Alive
        } catch (_: Exception) {
            ProbeResult.Dead
        }
    }

    private fun onPairingFailed(phase: String, e: Exception, hasRelayAddr: Boolean) {
        val message = when {
            e is IdentityVerificationException -> "桌面身份校验失败，已断开连接"
            phase == "discovery" && (e is TimeoutCancellationException || e is IOException) ->
                if (hasRelayAddr) {
                    // NSD 与中继双双失败（回退已触发，Story 12.5 Task 3）：合并如实提示
                    "未发现桌面设备，且中继连接失败，请检查网络或中继配置"
                } else {
                    // P13：发现/解析失败现以 IOException 快速上抛，与超时同文案呈现
                    "未发现桌面设备，请确认与桌面端在同一网络"
                }
            else -> "配对未完成，请在桌面端重新生成二维码后重试"
        }
        _pairingProgress.value = PairingProgress.Failed(message)
        liveState.value = degradedState()
        nsd.stopDiscovery()
    }

    /**
     * 配对/凭据失效的运行中恢复（T-S4，SPEC state-model §5.2 原子序列）：
     * 会话/指令/流式失效 → 清配对态 → 擦除本机身份（桌面侧已不认或密钥已损，
     * 残留只会造成永久失败重试）→ 置健康面 → 发布恢复事件（容器执行原子序列
     * 后半：清快照/通知 + 原因提示 + 导航回配对流）。待发箱不在此清除
     * （FR-43 无丢失，重新配对后继续投递）。替代旧 handleSecretsInvalidated
     * （仅清态无声无导航，用户面对静默死循环）。
     */
    private fun handleCredentialRecovery(reason: PairingRecoveryReason, machine: ConnectionStateMachine?) {
        if (machine != null) {
            machine.goDegraded()
        } else {
            liveState.value = degradedState()
        }
        nsd.stopDiscovery()
        debugMode.value = null
        commandChannel?.shutdown()
        streamCoordinator?.reset()
        _paired.value = false
        _pairingProgress.value = PairingProgress.Idle
        _pairingHealth.value = reason.health
        store.clear()
        wipeJob = scope.launch(Dispatchers.IO) { secrets.wipe() }
        _pairingRecovery.tryEmit(reason)
        CompanionLog.warn("Connection", "配对/凭据失效触发恢复：${reason::class.simpleName}，已清除配对态，请重新扫码配对")
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
        // P1：状态机输出接线——machine.state 收集进 liveState，UI 订阅的承载态
        // 才真正随会话变化（此前 machine 为编排局部实例，输出被整体丢弃）；
        // T7/T-S3：Degraded 值经 fillDegraded 填充真实缓存存在性/数据截止时间
        launch { machine.state.collect { liveState.value = fillDegraded(it) } }
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
                // T-S4：已配对重连只取既有身份——缺文件/不可解即配对信任链断裂，
                // 类型化上抛走恢复（不静默生成新身份顶替）
                val staticPrivate = withContext(ioDispatcher) { secrets.loadExistingStaticPrivateKey() }
                val session = connectDirect(
                    endpoint,
                    expectedDesktopPubHex = store.desktopPubkeyHex,
                    needsPairingAuth = false,
                    pairingNonce = null,
                    staticPrivate = staticPrivate,
                )
                // T-S2 时序不变量（state-model §4.2）：bind（commandReady=true）
                // 先于在线发布——UI 不得观察到「Direct 但不可发送」的竞窗
                val commandBinding = commandChannel?.bind()
                machine.onDirectEstablished()
                backoff.reset()
                CompanionLog.info("Connection", "直连会话建立")
                startSessionLoop(session, commandBinding)
                machine.onDirectLost(store.relayAddr != null)
                CompanionLog.info("Connection", "直连会话结束")
            } catch (e: IdentityVerificationException) {
                // T-S4（SPEC state-model §5.2）：已配对上下文信任锚失败 = 桌面身份
                // 变化（重装）——退避重试永远不可能成功，走恢复终止循环
                handleCredentialRecovery(PairingRecoveryReason.TrustMismatch, machine)
                return
            } catch (e: SecretsInvalidatedException) {
                handleCredentialRecovery(PairingRecoveryReason.CredentialInvalid, machine)
                return
            } catch (e: MissingSecretsException) {
                handleCredentialRecovery(PairingRecoveryReason.CredentialMissing, machine)
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
                // T-S4：同 runDirectLoop——已配对重连只取既有身份
                val staticPrivate = withContext(ioDispatcher) { secrets.loadExistingStaticPrivateKey() }
                val session = relay.connect(relayAddr, relayId, staticPrivate)
                try {
                    // Story 12.5：中继握手改用 m1 重发版（对端转发态 churn 空窗内
                    // 收敛，m2 正常到达时不触发重发、行为不变）；其后与直连同构
                    e2eHandshakeRelay(session, staticPrivate)
                    connectOverSession(
                        session,
                        store.desktopPubkeyHex,
                        needsPairingAuth = false,
                        pairingNonce = null,
                    )
                    // T-S2 时序不变量：bind（commandReady=true）先于中继在线发布
                    val commandBinding = commandChannel?.bind()
                    machine.onRelayEstablished()
                    established = true
                    backoff.reset()
                    CompanionLog.info("Connection", "中继会话建立")
                    coroutineScope {
                        val sessionJob = launch { startSessionLoop(session, commandBinding) }
                        // P5：直连恢复即拆除中继会话（单会话语义，防双活会话
                        // 并存——13.x 数据帧出现后将导致重复投递）
                        val directWatcher = launch {
                            machine.state.first { it is TransportStatus.Direct }
                            sessionJob.cancelAndJoin()
                        }
                        sessionJob.join()
                        directWatcher.cancel()
                    }
                } finally {
                    // P5 附带修复：握手/信任锚失败路径此前不关会话（连接泄漏）
                    session.close()
                }
            } catch (e: IdentityVerificationException) {
                // T-S4：同 runDirectLoop——桌面身份变化非网络抖动，恢复终止
                handleCredentialRecovery(PairingRecoveryReason.TrustMismatch, machine)
                return
            } catch (e: SecretsInvalidatedException) {
                handleCredentialRecovery(PairingRecoveryReason.CredentialInvalid, machine)
                return
            } catch (e: MissingSecretsException) {
                handleCredentialRecovery(PairingRecoveryReason.CredentialMissing, machine)
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

    // ── 共享协议序列（直连/中继同构，Story 12.5 抽取）────────────────

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
     * 承载建立后的共享序列（Story 12.5）：信任锚校验 → app 层帧序——
     * 直连 [connectDirect] 与中继 [connectViaRelay] 仅承载建立方式不同。
     */
    private fun connectOverSession(
        session: WsSession,
        expectedDesktopPubHex: String?,
        needsPairingAuth: Boolean,
        pairingNonce: String?,
    ) {
        verifyTrustAnchor(session, expectedDesktopPubHex)
        sendIntroFrames(session, needsPairingAuth, pairingNonce)
    }

    /**
     * 直连承载完整建立（AC）：NSD 端点 WS → XX initiator 握手 → 信任锚校验 →
     * 帧序。任一步失败即关闭会话再上抛——信任锚拒绝的冒名会话不得遗留半开
     * 连接（评审 B 用例暴露的泄漏：校验原本在 close 守护之外）。
     * T-S4：身份密钥由调用方注入（配对路径 createForPairing / 重连路径
     * loadExisting 的分流在调用点，连接函数本身不再触存储）。
     */
    private suspend fun connectDirect(
        endpoint: NsdEndpoint,
        expectedDesktopPubHex: String?,
        needsPairingAuth: Boolean,
        pairingNonce: String?,
        staticPrivate: ByteArray,
    ): WsSession {
        val session = withTimeout(CONNECT_TIMEOUT_MS) {
            wsOpener("ws://${endpoint.host}:${endpoint.port}", wsClient, CONNECT_TIMEOUT_MS)
        }
        try {
            e2eHandshake(session, staticPrivate)
            connectOverSession(session, expectedDesktopPubHex, needsPairingAuth, pairingNonce)
            return session
        } catch (e: Throwable) {
            session.close()
            throw e
        }
    }

    /**
     * 中继承载的配对/重连建立（Story 12.5 AC1/AC3）：register phone 槽 → relay
     * 鉴权（[RelayClient.connect]）→ 中继版 E2E 握手 → 共享序列。与直连仅
     * 承载建立方式不同（[connectOverSession]）。任一步失败即关闭会话再上抛。
     * T-S4：身份密钥由调用方注入（语义见 [connectDirect]）。
     */
    private suspend fun connectViaRelay(
        payload: QrPayload,
        needsPairingAuth: Boolean,
        pairingNonce: String?,
        staticPrivate: ByteArray,
    ): WsSession {
        val relayAddr = payload.relayAddr
            ?: throw IllegalStateException("中继回退仅应在 QR 携带 relayAddr 时调用")
        val session = relay.connect(relayAddr, payload.relayId, staticPrivate)
        try {
            e2eHandshakeRelay(session, staticPrivate)
            connectOverSession(session, payload.desktopStaticPubkey, needsPairingAuth, pairingNonce)
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

    /**
     * 中继版 E2E 握手（Story 12.5）：与 [e2eHandshake] 同构，但 m1 在 m2 等待
     * 期间周期重发——relay 无离线投递（对端不在即丢弃），桌面中继客户端转发
     * 态有超时+退避的 churn 空窗；手机超时断开还会触发 relay 对端 close 通知、
     * 杀死刚重建的桌面槽位，单发 m1 会与桌面重连形成活锁（桌面集成测试 14 次
     * 重试全 miss 的实测证据）。重发间隔远大于 m2 RTT；withTimeoutOrNull 取消
     * 竞窗可能吞掉的 m2 以非阻塞捞回兜底，重复投递由桌面 responder 帧位错判
     * 拒收兜底（整次重试）。直连不重发：TCP 建连即对端在场（AC5 零回归）。
     */
    private suspend fun e2eHandshakeRelay(session: WsSession, staticPrivate: ByteArray) {
        val channel = NoiseChannel.initiator(staticPrivate)
        val m1 = channel.writeHandshakeMessage()
        if (!session.send(m1)) throw IOException("握手消息发送失败")
        val m2: ByteArray = withTimeout(HANDSHAKE_STEP_MS) {
            var received: ByteArray? = null
            while (received == null) {
                val candidate = withTimeoutOrNull(RELAY_M1_RESEND_MS) { session.incoming.receive() }
                if (candidate != null) {
                    received = candidate
                } else {
                    received = session.incoming.tryReceive().getOrNull()
                    if (received == null && !session.send(m1)) throw IOException("握手消息发送失败")
                }
            }
            received
        }
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

    /**
     * 会话循环：30s 无入站即发 PING 保活；协议错误/通道关闭即退出（上层进退避）。
     * [commandBinding] 由调用方在发布在线态**之前** bind（T-S2 时序不变量：
     * commandReady=true 先于 Direct/Relay）；本循环 finally 负责 unbind——
     * 先于上层状态机丢失处理，commandReady=false 先行。
     */
    private suspend fun startSessionLoop(session: WsSession, commandBinding: CommandChannel.Binding?) = kotlinx.coroutines.coroutineScope {
        // T3：每会话一个帧消费者——承载切换/重连时会话循环重建，残缺分帧序列不跨会话
        val frameConsumer = frameConsumerFactory()
        // 13.3 T5：指令出站 pump（pending 表跨会话存活，binding 由调用方注入）
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
                    is Frame.Notice -> {
                        // T-S5（SPEC qr-semantics §3.2）：已配对上下文收到结构化
                        // 拒绝 = 桌面已移除本机（本地公钥未变，非信任锚失败）→
                        // PairingRevoked 恢复 + 终止整个编排协程树（重连只会
                        // 每轮再收拒绝）。旧桌面不发此 Notice → 回退忽略现状。
                        val rejection = PairingRejection.fromNoticeData(frame.data)
                        if (rejection != null && store.paired) {
                            handleCredentialRecovery(PairingRecoveryReason.PairingRevoked, machine = null)
                            sessionJob?.cancel()
                            return@coroutineScope
                        }
                    }
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

    private fun initialState(): TransportStatus =
        if (store.paired) {
            // T-S3（SPEC state-model §3）：冷启动第一帧 Connecting——20s 宽限内
            // 不降级不遮断（取代旧「启动即 Offline」）；宽限耗尽由状态机转 Degraded
            TransportStatus.Connecting
        } else {
            TransportStatus.Direct
        }

    /** Degraded 态填充真实缓存存在性/数据截止时间（T7：原硬编码 (false, null)）。 */
    private fun degradedState(): TransportStatus {
        val info = offlineInfo()
        return TransportStatus.Degraded(
            snapshotAvailable = info.snapshotAvailable,
            dataAsOf = info.dataAsOf,
        )
    }

    /** 非 Degraded 态原样透传；Degraded 态填真实缓存值。 */
    private fun fillDegraded(state: TransportStatus): TransportStatus =
        if (state is TransportStatus.Degraded) degradedState() else state

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

        /** 中继 E2E m1 重发间隔（Story 12.5）：覆盖桌面转发态 churn 空窗、远小于 m2 RTT。 */
        const val RELAY_M1_RESEND_MS = 1_000L
    }
}
