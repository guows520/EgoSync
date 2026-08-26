package com.egosync.companion.ui.dashboard

import androidx.lifecycle.ViewModel
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class DashboardUiState(
    val roles: List<RoleCard> = SnapshotStore.roles,
) {
    val averageEnergy: Int
        get() = if (roles.isEmpty()) 0 else roles.sumOf { it.energy } / roles.size

    val totalPending: Int
        get() = roles.sumOf { it.pendingCount }

    companion object {
        fun sample() = DashboardUiState()
    }
}

/** 仪表盘 mock 状态：角色卡列表来自快照（只读渲染）。 */
class DashboardViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        DashboardUiState(roles = container.snapshotStore.roles)
    )
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()
}
