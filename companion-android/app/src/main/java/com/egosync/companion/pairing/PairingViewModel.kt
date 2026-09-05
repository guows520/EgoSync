package com.egosync.companion.pairing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.connection.ConnectionClient
import com.egosync.companion.connection.PairingConnector
import com.egosync.companion.connection.PairingProgress
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** 首跑配对四步：欢迎说明 → 扫码取景 → 连接中 → 配对成功。 */
enum class PairingStep { WELCOME, SCAN, CONNECTING, SUCCESS }

/**
 * 配对流状态机（FR-40 首跑路径）。
 *
 * 真实路径（[pairing] 非空）：扫码解析通过 → [PairingConnector.pairWithQr]
 * 接管握手；进度驱动 CONNECTING 三阶段；PendingRebind（换绑待桌面确认）时
 * 停留 CONNECTING 并如实呈现「等待桌面确认」；成功 SUCCESS、失败回 SCAN 并
 * 展示原因。开发/Preview/既有单测路径（[pairing] 为空）保留模拟延迟。
 */
class PairingViewModel(
    private val connection: ConnectionClient,
    private val pairing: PairingConnector? = null,
) : ViewModel() {

    private val _step = MutableStateFlow(PairingStep.WELCOME)
    val step: StateFlow<PairingStep> = _step.asStateFlow()

    private val _connectStage = MutableStateFlow(0)
    val connectStage: StateFlow<Int> = _connectStage.asStateFlow()

    /** 换绑或中继首配（12.5 AC2）待桌面确认时为 true——CONNECTING 步呈现等待文案。 */
    private val _waitDesktopConfirm = MutableStateFlow(false)
    val waitDesktopConfirm: StateFlow<Boolean> = _waitDesktopConfirm.asStateFlow()

    /** 扫码的 QR 是否携带中继地址（12.5 AC1）——发现阶段行文案按此如实渲染。 */
    private val _qrHasRelay = MutableStateFlow(false)
    val qrHasRelay: StateFlow<Boolean> = _qrHasRelay.asStateFlow()

    /** 配对失败原因（SCAN 步展示，便于用户定位「为何没配上」）。 */
    private val _pairingError = MutableStateFlow<String?>(null)
    val pairingError: StateFlow<String?> = _pairingError.asStateFlow()

    /** 从欢迎页进入扫码页。 */
    fun startScan() {
        if (_step.value == PairingStep.WELCOME) _step.value = PairingStep.SCAN
    }

    /** 模拟扫码成功（开发/测试入口），进入连接中动画。 */
    fun onScanCompleted() {
        if (_step.value == PairingStep.SCAN) beginConnecting(simulated = pairing == null)
    }

    /**
     * 真实扫码回调（Story 12.4）：解析通过才进入连接中，残缺 QR 整体拒绝
     * （停留扫码页）；真实握手进度由 [PairingConnector] 驱动。
     */
    fun onQrScanned(qrJson: String) {
        if (_step.value != PairingStep.SCAN) return
        val parsed = QrPayload.parse(qrJson)
        if (parsed == null) {
            _pairingError.value = "二维码无效，请在桌面端重新生成"
            return
        }
        _pairingError.value = null
        _qrHasRelay.value = parsed.relayAddr != null
        if (pairing == null) {
            beginConnecting(simulated = true)
        } else {
            beginConnecting(simulated = false)
            pairing.pairWithQr(qrJson)
        }
    }

    /** 返回上一步（扫码→欢迎）。 */
    fun back() {
        when (_step.value) {
            PairingStep.SCAN -> _step.value = PairingStep.WELCOME
            else -> Unit
        }
    }

    private fun beginConnecting(simulated: Boolean) {
        _step.value = PairingStep.CONNECTING
        _connectStage.value = 0
        _waitDesktopConfirm.value = false
        if (simulated) {
            // 开发/Preview：模拟握手三阶段（发现设备 → 交换密钥 → 验证身份）
            viewModelScope.launch {
                delay(700)
                _connectStage.value = 1
                delay(800)
                _connectStage.value = 2
                delay(700)
                _step.value = PairingStep.SUCCESS
            }
        }
    }

    init {
        // 真实路径：把 PairingConnector 进度映射到 UI 阶段/状态（模拟路径跳过）
        val connector = pairing
        if (connector != null) {
            viewModelScope.launch {
            connector.pairingProgress.collect { progress ->
                when (progress) {
                    PairingProgress.Idle -> Unit
                    PairingProgress.DiscoveringDevice -> {
                        _connectStage.value = 0
                        _waitDesktopConfirm.value = false
                    }
                    PairingProgress.ExchangingKeys -> _connectStage.value = 1
                    PairingProgress.VerifyingIdentity -> _connectStage.value = 2
                    PairingProgress.WaitDesktopConfirm -> _waitDesktopConfirm.value = true
                    PairingProgress.Success -> {
                        _waitDesktopConfirm.value = false
                        _pairingError.value = null
                        _step.value = PairingStep.SUCCESS
                    }
                    is PairingProgress.Failed -> {
                        _waitDesktopConfirm.value = false
                        _pairingError.value = progress.message
                        _step.value = PairingStep.SCAN
                    }
                }
            }
            }
        }
    }
}
