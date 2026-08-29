package com.egosync.companion.ui.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.ActivityWindow
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.MetricType
import com.egosync.companion.sync.RoleCard
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class DashboardUiState(
    val roles: List<RoleCard> = emptyList(),
    // ── FR-38 活动统计 ──
    /** scope 下拉："all"=全部 / "butler"=管家 / 角色 id（镜像桌面 useDashboard.scope 三型）。 */
    val metricsScopeId: String = "all",
    /** 统计时间窗：默认全部日期。 */
    val activityWindow: ActivityWindow = ActivityWindow.All,
    /** 指标提供者（VM 注入快照派生；null=无数据源全指标「—」。copy 语义保留注入）。 */
    val metricsProvider: ((String, ActivityWindow) -> Map<MetricType, Int?>)? = null,
    /** 快照键（对象同一性，评审 P11）：仅 metrics 变化而 roles 列表等值的快照也要驱动
     *  发射/重组，否则经 getter 拉取的全局指标（如记忆数量）不刷新。 */
    val snapshotKey: DesktopSnapshot? = null,
) {
    val averageEnergy: Int
        get() = if (roles.isEmpty()) 0 else roles.sumOf { it.energy } / roles.size

    val totalPending: Int
        get() = roles.sumOf { it.pendingCount }

    /** 当前 scope+时间窗下的活动指标（快照全量口径；不可得指标为 null → UI 显示「—」）。 */
    val activityMetrics: Map<MetricType, Int?>
        get() = metricsProvider?.invoke(metricsScopeId, activityWindow)
            ?: MetricType.entries.associateWith { null }

    companion object {
        fun sample() = DashboardUiState(
            roles = com.egosync.companion.ui.previewRoles,
        )
    }
}

/**
 * 仪表盘状态：角色卡列表来自快照（STATE_DELTA 即时刷新，AC2）；
 * 活动统计筛选（FR-38）由 scope 派生（快照全量口径）。
 */
class DashboardViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        DashboardUiState(
            roles = container.snapshotStore.roles,
            metricsProvider = container.snapshotStore::activityMetrics,
        )
    )
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    init {
        // AC2：快照全量替换即时刷新角色卡；snapshotKey 随快照对象变更驱动指标重组（评审 P11）
        viewModelScope.launch {
            var lastSnapshot = container.snapshotStore.state.value.snapshot
            container.snapshotStore.state.collect { state ->
                when {
                    state.loaded && state.snapshot !== lastSnapshot -> {
                        lastSnapshot = state.snapshot
                        _uiState.update {
                            it.copy(roles = container.snapshotStore.roles, snapshotKey = state.snapshot)
                        }
                    }
                    // unpair/密钥失效自愈（store.clear 不导航）：清空角色卡与指标键，
                    // 不残留已解配桌面的数据（评审 P2）
                    !state.loaded -> {
                        lastSnapshot = null
                        _uiState.update { it.copy(roles = emptyList(), snapshotKey = null) }
                    }
                }
            }
        }
    }

    /** FR-38：切换统计范围（全部/管家/某角色）。 */
    fun setMetricsScope(scopeId: String) {
        _uiState.update { it.copy(metricsScopeId = scopeId) }
    }

    /** FR-38：切换时间窗（预设或自定义区间；13.2 呈现快照全量口径）。 */
    fun setActivityWindow(window: ActivityWindow) {
        _uiState.update { it.copy(activityWindow = window) }
    }
}
