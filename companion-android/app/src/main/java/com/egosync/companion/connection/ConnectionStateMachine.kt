package com.egosync.companion.connection

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.util.concurrent.CopyOnWriteArrayList

/**
 * 承载状态机（SPEC state-and-recovery-model §2，AC4/AC5 演进）：
 * Connecting / Reconnecting / Direct / Relay / Degraded + prefer-direct 滞回 + 20s 宽限。
 *
 * - 冷启动初值 Connecting（取代旧「第一帧即 Offline」），[graceMs] 宽限耗尽仍
 *   无会话 → Degraded（降级 ≠ 停止重连，恢复即回在线）；
 * - 在线态失去 → Reconnecting 并重启宽限；连接**尝试失败**不重置宽限——否则
 *   退避重试每秒失败一次会把 Degraded 无限推迟；
 * - 直连丢失且有中继可用：3s 探测窗（[hysteresisMs]，防抖动）——窗内直连恢复
 *   （[onDirectEstablished]）则取消切换；窗满仍不可达才发 [Intent.StartRelay]
 *   （滞回窗内 Direct 展示保持，写操作就绪由 commandReady 单独守门）；
 * - 中继态下直连重试失败（[onDirectLost]）不扰动状态——切回直连只认握手成功；
 * - Degraded 下重连失败保持 Degraded（不回 Connecting/Reconnecting 闪烁横幅）。
 *
 * 事件由连接编排协程串行喂入；探测窗与宽限用注入 [sleep]（时间可控测试）。
 */
class ConnectionStateMachine(
    private val scope: CoroutineScope,
    private val hysteresisMs: Long = 3_000,
    private val graceMs: Long = CONNECT_GRACE_MS,
    private val sleep: suspend (Long) -> Unit = { delay(it) },
) {
    sealed interface Intent {
        /** 滞回窗满、直连确认不可达 → 请求建立中继承载。 */
        data object StartRelay : Intent
    }

    private val _state = MutableStateFlow<TransportStatus>(TransportStatus.Connecting)
    val state: StateFlow<TransportStatus> = _state.asStateFlow()

    // 意图用同步观察者而非 SharedFlow：发射确定性（测试无需赌调度时序），
    // 生产侧注册一个回调即可（编排协程直连）
    private val intentListeners = CopyOnWriteArrayList<(Intent) -> Unit>()

    fun onIntent(listener: (Intent) -> Unit) {
        intentListeners.add(listener)
    }

    private fun emit(intent: Intent) {
        intentListeners.forEach { it(intent) }
    }

    @Volatile
    private var directUp = false

    private var probeJob: Job? = null

    private var graceJob: Job? = null

    init {
        startGrace()
    }

    /** 20s 宽限：耗尽仍无会话 → Degraded；会话建立（Direct/Relay）即取消。 */
    private fun startGrace() {
        graceJob?.cancel()
        graceJob = scope.launch {
            sleep(graceMs)
            if (!directUp && _state.value !is TransportStatus.Direct && _state.value !is TransportStatus.Relay) {
                _state.value = TransportStatus.Degraded(snapshotAvailable = false, dataAsOf = null)
            }
        }
    }

    fun onDirectEstablished() {
        directUp = true
        probeJob?.cancel()
        probeJob = null
        graceJob?.cancel()
        graceJob = null
        _state.value = TransportStatus.Direct
    }

    fun onDirectLost(relayAvailable: Boolean) {
        directUp = false
        if (_state.value is TransportStatus.Relay) {
            // 中继态下直连重试失败：不扰动状态（AC4 切回只认直连握手成功）
            return
        }
        if (!relayAvailable) {
            goUnreachable()
            return
        }
        if (probeJob != null) return // 探测已在进行
        // 滞回驻留：探测窗内保持既有状态不闪变（写操作就绪由 commandReady 守门）
        probeJob = scope.launch {
            sleep(hysteresisMs)
            if (!directUp) {
                emit(Intent.StartRelay)
            }
            probeJob = null
        }
    }

    fun onRelayEstablished() {
        probeJob?.cancel()
        probeJob = null
        // P18：直连恢复瞬间到达的「迟到中继成功」不得覆盖 Direct——否则状态机
        // 自替换健康直连会话并瞬时误报（prefer-direct：直连在线时中继让位）
        if (directUp) return
        graceJob?.cancel()
        graceJob = null
        _state.value = TransportStatus.Relay
    }

    fun onRelayLost() {
        // 中继也断 → 双承载不可达（AC5 不误报在线）
        if (_state.value is TransportStatus.Relay) {
            goUnreachable()
        }
    }

    /**
     * 中继尝试失败且从未建立过（P2）：直连已丢失而中继又连不上 → 双承载皆不可
     * 达。[onRelayLost] 只覆盖「建立后丢失」，本方法补「从未建立」路径——缺它
     * 则状态机在滞回窗满后永久驻留 Direct、双断不降级（AC5 击穿）。
     */
    fun onRelayEstablishmentFailed() {
        if (directUp) return // 直连已恢复：中继失败不扰动（同 onRelayLost 语义）
        goUnreachable()
    }

    /** 终局降级（unpair/自愈清理路径）：不经宽限直接如实 Degraded。 */
    fun goDegraded() {
        probeJob?.cancel()
        probeJob = null
        graceJob?.cancel()
        graceJob = null
        _state.value = TransportStatus.Degraded(snapshotAvailable = false, dataAsOf = null)
    }

    /**
     * 承载不可达（会话失去/从未建立）：在线态 → Reconnecting 并重启宽限；
     * Connecting/Reconnecting/Degraded 保持自身——宽限不因重试失败重置（防活锁）。
     */
    private fun goUnreachable() {
        probeJob?.cancel()
        probeJob = null
        when (_state.value) {
            is TransportStatus.Direct, is TransportStatus.Relay -> {
                _state.value = TransportStatus.Reconnecting
                startGrace()
            }
            is TransportStatus.Connecting -> Unit // 冷启动宽限自 init 持续
            is TransportStatus.Reconnecting -> Unit // 宽限已在跑
            is TransportStatus.Degraded -> Unit // 降级下重连失败保持降级
        }
    }
}
