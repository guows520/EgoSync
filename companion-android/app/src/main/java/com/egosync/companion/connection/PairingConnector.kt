package com.egosync.companion.connection

import kotlinx.coroutines.flow.StateFlow
import org.json.JSONObject

/**
 * 结构化配对拒绝（T-S5，SPEC qr-semantics §3.2）：桌面拒绝点经已建立
 * transport 发送的 Notice 判别信息（`{"type":"pairingRejected","reason":...}`）。
 * 原因码结构化进 [PairingProgress.Failed]，用户文案由配对 VM 层映射——
 * 连接层不拼文案。
 */
enum class PairingRejection(val reasonCode: String) {
    /** 二维码已使用或已过期（早期准入拒绝 / 窗口已关时的 nonce 校验拒绝）。 */
    PairingWindowClosed("pairingWindowClosed"),

    /** 二维码已失效——配对码不匹配（窗口开但 nonce 不一致：旧码/并发消费）。 */
    NonceConsumed("nonceConsumed");

    companion object {
        /** 识别 Notice 载荷：非 pairingRejected 或未知 reason 返回 null（回退现状）。 */
        fun fromNoticeData(data: String): PairingRejection? = runCatching {
            val json = JSONObject(data)
            if (json.optString("type") == "pairingRejected") {
                entries.firstOrNull { it.reasonCode == json.optString("reason") }
            } else {
                null
            }
        }.getOrNull()
    }
}

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

    /**
     * 失败：[rejection] 非空为桌面结构化拒绝（T-S5，文案层据此映射），否则
     * [message] 为网络/本地类失败描述（既有如实文案，非重配暗示）。
     */
    data class Failed(val message: String? = null, val rejection: PairingRejection? = null) : PairingProgress
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
