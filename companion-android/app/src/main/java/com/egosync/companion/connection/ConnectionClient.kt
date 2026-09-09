package com.egosync.companion.connection

import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * 承载/展示面（SPEC state-and-recovery-model §1，取代旧 ConnectionState）：
 * 「展示什么连接状态」与「能否写」（commandReady）解耦——滞回/竞窗内可展示
 * Direct 但 binding 已空（写控件一律以 commandReady 为准，不读展示态）。
 */
sealed interface TransportStatus {
    /** 冷启动后、尚未建立过会话的宽限态（[CONNECT_GRACE_MS] 计时中）。 */
    data object Connecting : TransportStatus

    /** 本进程内曾建立过会话后失去、重连中（宽限计时重新开始）。 */
    data object Reconnecting : TransportStatus

    /** 局域网直连会话已建立。 */
    data object Direct : TransportStatus

    /** 中继会话已建立。 */
    data object Relay : TransportStatus

    /** 连接宽限耗尽：缓存可浏览 + 本地能力可用 + 引擎写操作禁用（重连不停止）。 */
    data class Degraded(
        val snapshotAvailable: Boolean,
        val dataAsOf: String?,
    ) : TransportStatus
}

/**
 * 配对/凭据健康面（SPEC state-model §1，T-S3 落类型，失效信号 T-S4 接线）：
 * 驱动重配入口与恢复事件——网络类失败（TransportStatus 降级但健康面 Ok）
 * 不得出现重配入口。
 */
sealed interface PairingHealth {
    data object Ok : PairingHealth
    /** 已配对元数据在、包裹私钥文件缺失（冷启动可同步检测）。 */
    data object CredentialMissing : PairingHealth
    /** 私钥不可解 / 运行中密钥失效。 */
    data object CredentialInvalid : PairingHealth
    /** 桌面授权失效：结构化拒绝 + 本地已配对且公钥未变。 */
    data object PairingRevoked : PairingHealth
    /** 桌面身份锚变化：信任锚校验失败。 */
    data object TrustMismatch : PairingHealth
}

/**
 * 配对/凭据失效的一次性恢复事件（SPEC state-model §5.2，T-S4）：运行中
 * （已离开冷启动）检测到配对信任链断裂时发布——容器据此执行原子序列后半
 * （清快照/通知 + 原因提示）并导航回配对流。冷启动检测经 [PairingHealth]
 * 状态承载（一次性事件在冷启动无订阅者会丢，状态不会）。
 */
sealed interface PairingRecoveryReason {
    /** 已配对元数据在、私钥文件缺失（运行中；冷启动走健康态不发事件）。 */
    data object CredentialMissing : PairingRecoveryReason

    /** 私钥不可解 / Keystore 失效。 */
    data object CredentialInvalid : PairingRecoveryReason

    /** 桌面结构化拒绝 + 本地已配对且公钥未变（T-S5 接线）。 */
    data object PairingRevoked : PairingRecoveryReason

    /** 信任锚校验失败：桌面身份变化。 */
    data object TrustMismatch : PairingRecoveryReason

    /** 对应健康面（[PairingHealth]）值。 */
    val health: PairingHealth
        get() = when (this) {
            CredentialMissing -> PairingHealth.CredentialMissing
            CredentialInvalid -> PairingHealth.CredentialInvalid
            PairingRevoked -> PairingHealth.PairingRevoked
            TrustMismatch -> PairingHealth.TrustMismatch
        }
}

/** 连接宽限（SPEC state-model §3）：Connecting/Reconnecting 20s 内不降级。 */
const val CONNECT_GRACE_MS = 20_000L

/** Debug 状态模拟预设（设置页隐藏入口驱动）：六档覆盖承载态全谱。 */
enum class DebugConnectionMode(val label: String) {
    DIRECT("局域网直连"),
    RELAY("中继转发"),
    CONNECTING("连接中（宽限）"),
    RECONNECTING("重连中（宽限）"),
    OFFLINE("离线 · 无缓存"),
    DEGRADED("降级 · 只读缓存"),
}

/**
 * 降级态快照信息（Story 13.2 T7）：快照缓存存在性 + 数据截止时间。
 * 由快照存储提供（`SnapshotStore.offlineInfo()`），连接层经注入缝读取——
 * Degraded 状态不再硬编码 (false, null)。
 */
data class OfflineSnapshotInfo(
    val snapshotAvailable: Boolean,
    val dataAsOf: String?,
)

/**
 * 连接客户端抽象——未来接入真实连接层的替换点。
 *
 * 真实实现职责（见 architecture.md 手机伴侣增量章节）：
 * NSD 发现 → WS 直连桌面 / WS 连中继按 relay_id 转发，同一加密帧协议双承载；
 * 断线重连以最新快照补齐。本原型中由 [FakeConnectionClient] 以 StateFlow 驱动。
 */
interface ConnectionClient {

    /** 连接状态流：UI（状态指示条/降级横幅/「我的」页配对设备卡）订阅此流实时响应。 */
    val state: StateFlow<TransportStatus>

    /**
     * 会话就绪面（T-S2，SPEC state-and-recovery-model §1/§4.2）：出站指令
     * 通道已绑定。写操作（发送按钮/写控件 enabled、flush 守门）唯一判据——
     * 不得以 `state is Direct/Relay`（展示态）推断：滞回/竞窗内展示在线但
     * binding 已空，点了才报「桌面引擎不可达」。
     */
    val commandReady: StateFlow<Boolean>

    /** 配对/凭据健康面：Ok 之外的值驱动重配入口与恢复事件（T-S4 接线）。 */
    val pairingHealth: StateFlow<PairingHealth>

    /** 运行中配对/凭据失效的一次性恢复事件（T-S4：容器清理协调 + 导航）。 */
    val pairingRecovery: SharedFlow<PairingRecoveryReason>

    /** 是否已完成配对（决定首跑进入配对流还是主界面）。 */
    val paired: StateFlow<Boolean>

    /** Debug 预览：六档承载态间手动切换。 */
    fun setDebugMode(mode: DebugConnectionMode)

    /** 配对流程完成（配对成功页调用）。 */
    fun completePairing()

    /** 解除配对（设置页调用）：回到未配对初始态。 */
    fun unpair()
}
