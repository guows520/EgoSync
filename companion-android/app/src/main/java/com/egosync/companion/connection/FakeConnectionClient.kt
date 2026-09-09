package com.egosync.companion.connection

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 假连接客户端：纯内存 StateFlow 驱动，零网络。
 *
 * 接入真实连接层时，用 NSD 发现 + WS 直连/中继的实现替换本类
 * （保持 [ConnectionClient] 接口签名不变即可，UI 层零改动）。
 * T-S3：state 输出 [TransportStatus] 六档承载态，commandReady 同步翻转。
 */
class FakeConnectionClient(
    initialPaired: Boolean = false,
) : ConnectionClient {

    private val _state = MutableStateFlow<TransportStatus>(TransportStatus.Direct)
    override val state: StateFlow<TransportStatus> = _state.asStateFlow()

    /** T-S2：Debug 档位同步驱动 commandReady（DIRECT/RELAY=true，其余档=false）。 */
    private val _commandReady = MutableStateFlow(true)
    override val commandReady: StateFlow<Boolean> = _commandReady.asStateFlow()

    /** 健康面（T-S3 落类型）：失效信号 T-S4 接线，fake 恒 Ok。 */
    private val _pairingHealth = MutableStateFlow(PairingHealth.Ok)
    override val pairingHealth: StateFlow<PairingHealth> = _pairingHealth.asStateFlow()

    /** 恢复事件（T-S4）：fake 无真实失效源，恒静默。 */
    private val _pairingRecovery = MutableSharedFlow<PairingRecoveryReason>(extraBufferCapacity = 8)
    override val pairingRecovery: SharedFlow<PairingRecoveryReason> = _pairingRecovery.asSharedFlow()

    private val _paired = MutableStateFlow(initialPaired)
    override val paired: StateFlow<Boolean> = _paired.asStateFlow()

    override fun setDebugMode(mode: DebugConnectionMode) {
        _state.value = when (mode) {
            DebugConnectionMode.DIRECT -> TransportStatus.Direct
            DebugConnectionMode.RELAY -> TransportStatus.Relay
            DebugConnectionMode.CONNECTING -> TransportStatus.Connecting
            DebugConnectionMode.RECONNECTING -> TransportStatus.Reconnecting
            DebugConnectionMode.OFFLINE ->
                TransportStatus.Degraded(snapshotAvailable = false, dataAsOf = null)
            DebugConnectionMode.DEGRADED ->
                TransportStatus.Degraded(snapshotAvailable = true, dataAsOf = "今天 08:15")
        }
        _commandReady.value = mode == DebugConnectionMode.DIRECT || mode == DebugConnectionMode.RELAY
    }

    override fun completePairing() {
        _paired.value = true
        _state.value = TransportStatus.Direct
        _commandReady.value = true
    }

    override fun unpair() {
        _paired.value = false
        _state.value = TransportStatus.Direct
        _commandReady.value = true
    }
}
