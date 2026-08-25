package com.egosync.companion.connection

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 假连接客户端：纯内存 StateFlow 驱动，零网络。
 *
 * 接入真实连接层时，用 NSD 发现 + WS 直连/中继的实现替换本类
 * （保持 [ConnectionClient] 接口签名不变即可，UI 层零改动）。
 */
class FakeConnectionClient(
    initialPaired: Boolean = false,
) : ConnectionClient {

    private val _state = MutableStateFlow<ConnectionState>(ConnectionState.Direct)
    override val state: StateFlow<ConnectionState> = _state.asStateFlow()

    private val _paired = MutableStateFlow(initialPaired)
    override val paired: StateFlow<Boolean> = _paired.asStateFlow()

    override fun setDebugMode(mode: DebugConnectionMode) {
        _state.value = when (mode) {
            DebugConnectionMode.DIRECT -> ConnectionState.Direct
            DebugConnectionMode.RELAY -> ConnectionState.Relay
            DebugConnectionMode.OFFLINE ->
                ConnectionState.Offline(snapshotAvailable = false, dataAsOf = null)
            DebugConnectionMode.DEGRADED ->
                ConnectionState.Offline(snapshotAvailable = true, dataAsOf = "今天 08:15")
        }
    }

    override fun completePairing() {
        _paired.value = true
        _state.value = ConnectionState.Direct
    }

    override fun unpair() {
        _paired.value = false
        _state.value = ConnectionState.Direct
    }
}
