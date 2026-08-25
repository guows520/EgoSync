package com.egosync.companion.pairing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.connection.ConnectionClient
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** 首跑配对四步：欢迎说明 → 扫码取景模拟 → 连接中 → 配对成功。 */
enum class PairingStep { WELCOME, SCAN, CONNECTING, SUCCESS }

/**
 * 配对流状态机（FR-40 首跑路径）。
 * 纯步骤推进逻辑同步可测；CONNECTING→SUCCESS 的模拟延迟走 viewModelScope。
 */
class PairingViewModel(
    private val connection: ConnectionClient,
) : ViewModel() {

    private val _step = MutableStateFlow(PairingStep.WELCOME)
    val step: StateFlow<PairingStep> = _step.asStateFlow()

    private val _connectStage = MutableStateFlow(0)
    val connectStage: StateFlow<Int> = _connectStage.asStateFlow()

    /** 从欢迎页进入扫码页。 */
    fun startScan() {
        if (_step.value == PairingStep.WELCOME) _step.value = PairingStep.SCAN
    }

    /** 模拟扫码成功，进入连接中动画。 */
    fun onScanCompleted() {
        if (_step.value == PairingStep.SCAN) beginConnecting()
    }

    /** 返回上一步（扫码→欢迎）。 */
    fun back() {
        when (_step.value) {
            PairingStep.SCAN -> _step.value = PairingStep.WELCOME
            else -> Unit
        }
    }

    private fun beginConnecting() {
        _step.value = PairingStep.CONNECTING
        _connectStage.value = 0
        // 模拟握手三阶段：发现设备 → 交换密钥 → 验证身份
        viewModelScope.launch {
            delay(700)
            _connectStage.value = 1
            delay(800)
            _connectStage.value = 2
            delay(700)
            _step.value = PairingStep.SUCCESS
        }
    }

    /** 配对成功后进入主界面（持久化由容器层负责）。 */
    fun enterApp() {
        if (_step.value == PairingStep.SUCCESS) connection.completePairing()
    }
}
