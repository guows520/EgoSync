package com.egosync.companion.connection

import kotlinx.coroutines.flow.StateFlow

/**
 * 连接状态三态（FR-40）：局域网直连 / 中继转发 / 离线。
 *
 * Offline 携带降级信息（FR-43）：
 * - snapshotAvailable = true：持有最后已知快照（降级态，只读缓存 + 数据截止时间）
 * - snapshotAvailable = false：无可用缓存（完全离线）
 */
sealed interface ConnectionState {

    /** 局域网内自动发现并直连桌面引擎（NSD）。 */
    data object Direct : ConnectionState

    /** 出网经云中继的端到端加密转发通道。 */
    data object Relay : ConnectionState

    /** 桌面引擎不可达，进入降级态。 */
    data class Offline(
        val snapshotAvailable: Boolean,
        val dataAsOf: String?,
    ) : ConnectionState

    val engineAvailable: Boolean
        get() = this !is Offline
}

/** Debug 状态模拟的四档预设（设置页隐藏入口驱动）。 */
enum class DebugConnectionMode(val label: String) {
    DIRECT("局域网直连"),
    RELAY("中继转发"),
    OFFLINE("离线 · 无缓存"),
    DEGRADED("降级 · 只读缓存"),
}

/**
 * 降级态快照信息（Story 13.2 T7）：快照缓存存在性 + 数据截止时间。
 * 由快照存储提供（`SnapshotStore.offlineInfo()`），连接层经注入缝读取——
 * Offline 状态不再硬编码 (false, null)。
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

    /** 连接状态流：UI（降级遮罩/引擎可用性/「我的」页配对设备卡）订阅此流实时响应。 */
    val state: StateFlow<ConnectionState>

    /** 是否已完成配对（决定首跑进入配对流还是主界面）。 */
    val paired: StateFlow<Boolean>

    /** Debug 预览：在 direct / relay / offline / degraded 四态间手动切换。 */
    fun setDebugMode(mode: DebugConnectionMode)

    /** 配对流程完成（配对成功页调用）。 */
    fun completePairing()

    /** 解除配对（设置页调用）：回到未配对初始态。 */
    fun unpair()
}
