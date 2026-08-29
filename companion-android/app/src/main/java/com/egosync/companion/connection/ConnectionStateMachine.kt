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
 * 三态连接状态机（AC4/AC5）：Direct / Relay / Offline + prefer-direct 滞回。
 *
 * - 直连丢失且有中继可用：进入 3s 探测窗（[hysteresisMs]，防抖动）——窗内直连
 *   恢复（[onDirectEstablished]）则取消切换；窗满仍不可达才发 [Intent.StartRelay]；
 * - 中继态下直连重试失败（[onDirectLost]）不扰动状态——切回直连只认握手成功；
 * - 双承载均失败（[onRelayLost] 建立后丢失 / [onRelayEstablishmentFailed] 从未
 *   建立）→ Offline(false, null)，不误报有缓存（快照属 13.x）。
 *
 * 事件由连接编排协程串行喂入；探测窗用注入 [sleep]（时间可控测试）。
 */
class ConnectionStateMachine(
    private val scope: CoroutineScope,
    private val hysteresisMs: Long = 3_000,
    private val sleep: suspend (Long) -> Unit = { delay(it) },
) {
    sealed interface Intent {
        /** 滞回窗满、直连确认不可达 → 请求建立中继承载。 */
        data object StartRelay : Intent
    }

    private val _state = MutableStateFlow<ConnectionState>(
        ConnectionState.Offline(snapshotAvailable = false, dataAsOf = null),
    )
    val state: StateFlow<ConnectionState> = _state.asStateFlow()

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

    fun onDirectEstablished() {
        directUp = true
        probeJob?.cancel()
        probeJob = null
        _state.value = ConnectionState.Direct
    }

    fun onDirectLost(relayAvailable: Boolean) {
        directUp = false
        if (_state.value is ConnectionState.Relay) {
            // 中继态下直连重试失败：不扰动状态（AC4 切回只认直连握手成功）
            return
        }
        if (!relayAvailable) {
            goOffline()
            return
        }
        if (probeJob != null) return // 探测已在进行
        // 滞回驻留：探测窗内保持既有状态不闪变
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
        _state.value = ConnectionState.Relay
    }

    fun onRelayLost() {
        // 中继也断 → 双承载不可达：Offline 且如实无缓存（AC5 不误报）
        if (_state.value is ConnectionState.Relay) {
            goOffline()
        }
    }

    /**
     * 中继尝试失败且从未建立过（P2）：直连已丢失而中继又连不上 → 双承载皆不可
     * 达 → Offline。[onRelayLost] 只覆盖「建立后丢失」，本方法补「从未建立」
     * 路径——缺它则状态机在 Direct 滞回窗内永久驻留、双断不报离线（AC5 击穿）。
     */
    fun onRelayEstablishmentFailed() {
        if (directUp) return // 直连已恢复：中继失败不扰动（同 onRelayLost 语义）
        goOffline()
    }

    fun goOffline() {
        probeJob?.cancel()
        probeJob = null
        _state.value = ConnectionState.Offline(snapshotAvailable = false, dataAsOf = null)
    }
}
