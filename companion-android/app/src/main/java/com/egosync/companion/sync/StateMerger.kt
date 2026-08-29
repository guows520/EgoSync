package com.egosync.companion.sync

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 快照元数据（随 [StateMerger.state] 暴露，AC3/AC4 消费）。
 */
data class SnapshotMetadata(
    val generatedAt: String,
    /** 数据截止时间（截断时）；未截断为 null。 */
    val dataCutoffAt: String?,
    val truncated: Boolean,
    /** 被截断的域名清单（conversations/briefings/weeklyReviews）。 */
    val truncatedDomains: List<String>,
)

/**
 * 快照态：null = 未收到任何快照（冷启动且无缓存）。
 */
data class SnapshotState(
    val snapshot: DesktopSnapshot?,
) {
    val loaded: Boolean get() = snapshot != null

    val metadata: SnapshotMetadata?
        get() = snapshot?.let {
            SnapshotMetadata(
                generatedAt = it.generatedAt,
                dataCutoffAt = it.dataCutoffAt,
                truncated = it.truncated,
                truncatedDomains = it.truncatedDomains,
            )
        }

    companion object {
        val Empty = SnapshotState(snapshot = null)
    }
}

/**
 * StateMerger——SNAPSHOT 与 STATE_DELTA 的**同一处理**（全量替换，13.1 裁决 2）：
 * 两者载荷同为全量快照，仅触发时机不同（建连 vs 写信号 debounce 后）。
 * 域级 diff 是明确 deferred 项，严禁自作主张做字段级合并。
 *
 * 行为契约镜像桌面测试：
 * - `write_signal_pushes_state_delta`（STATE_DELTA 全量替换）
 * - `reconnect_receives_latest_snapshot_after_gap`（重连全量替换）
 * - STATE_DELTA 先于 SNAPSHOT 到达（同载荷全量，任意先到皆正确）
 */
class StateMerger {

    private val _state = MutableStateFlow<SnapshotState>(SnapshotState.Empty)
    val state: StateFlow<SnapshotState> = _state.asStateFlow()

    /** 全量替换并返回新状态。 */
    fun applySnapshot(snapshot: DesktopSnapshot): SnapshotState {
        val next = SnapshotState(snapshot = snapshot)
        _state.value = next
        return next
    }

    /** 清空快照态（解除配对配套：内存态归零，等待下次 SNAPSHOT）。 */
    fun reset() {
        _state.value = SnapshotState.Empty
    }
}
