package com.egosync.companion.ui.dashboard

import androidx.lifecycle.ViewModel
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.ActivityWindow
import com.egosync.companion.sync.MetricType
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class DashboardUiState(
    val roles: List<RoleCard> = SnapshotStore.roles,
    // ── FR-38 活动统计 ──
    /** scope 下拉："all"=全部 / "butler"=管家 / 角色 id（镜像桌面 useDashboard.scope 三型）。 */
    val metricsScopeId: String = "all",
    /** 统计时间窗：默认全部日期。 */
    val activityWindow: ActivityWindow = ActivityWindow.All,
) {
    val averageEnergy: Int
        get() = if (roles.isEmpty()) 0 else roles.sumOf { it.energy } / roles.size

    val totalPending: Int
        get() = roles.sumOf { it.pendingCount }

    /** 当前 scope+时间窗下的活动指标（mock 同步聚合，无加载/错误态）。 */
    val activityMetrics: Map<MetricType, Int>
        get() = SnapshotStore.activityMetrics(metricsScopeId, activityWindow)

    companion object {
        fun sample() = DashboardUiState()
    }
}

/**
 * 仪表盘 mock 状态：角色卡列表来自快照（只读渲染）；
 * 活动统计筛选（FR-38）由 scope+时间窗派生，数据为 SnapshotStore 分桶账本。
 */
class DashboardViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        DashboardUiState(roles = container.snapshotStore.roles)
    )
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    /** FR-38：切换统计范围（全部/管家/某角色）。 */
    fun setMetricsScope(scopeId: String) {
        _uiState.update { it.copy(metricsScopeId = scopeId) }
    }

    /** FR-38：切换时间窗（预设或自定义区间）。 */
    fun setActivityWindow(window: ActivityWindow) {
        _uiState.update { it.copy(activityWindow = window) }
    }
}
