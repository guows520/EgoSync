package com.egosync.companion.connection

import kotlinx.coroutines.flow.StateFlow

/**
 * 配对进度（PairingScreen CONNECTING 三阶段的真实驱动源）：
 * 发现设备 → 交换密钥 → 验证身份 → 成功/失败。
 */
sealed interface PairingProgress {
    data object Idle : PairingProgress
    data object DiscoveringDevice : PairingProgress
    data object ExchangingKeys : PairingProgress
    data object VerifyingIdentity : PairingProgress

    /**
     * 换绑待桌面确认（Story 12.4 Task 7）：桌面已有配对设备时，新手机提交
     * 有效 nonce 后桌面会挂 pending 并关闭连接等待人工确认——手机侧如实呈现
     * 「等待桌面确认」并周期重连，确认后自动恢复。
     */
    data object WaitDesktopConfirm : PairingProgress

    data object Success : PairingProgress
    data class Failed(val message: String) : PairingProgress
}

/**
 * 配对连接器（裁决 6）：真实配对需要 QR payload 输入，而 [ConnectionClient]
 * 接口签名冻结（AC1 硬约束）——本正交接口承载 pairWithQr，不污染连接抽象。
 */
interface PairingConnector {
    val pairingProgress: StateFlow<PairingProgress>

    /** 以扫码得到的 QR JSON 发起真实配对（解析失败立即 Failed，不进入连接）。 */
    fun pairWithQr(qrJson: String)
}
