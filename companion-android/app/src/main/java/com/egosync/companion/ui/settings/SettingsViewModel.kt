package com.egosync.companion.ui.settings

import androidx.lifecycle.ViewModel
import com.egosync.companion.AppModelContainer
import com.egosync.companion.connection.DebugConnectionMode
import com.egosync.companion.sync.ProactivityLevel
import com.egosync.companion.sync.RoleCard
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
import com.egosync.companion.ui.theme.ThemeMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class SettingsUiState(
    val themeMode: ThemeMode = ThemeMode.DARK,
    val whisperEnabled: Boolean = true,
    val tapEnabled: Boolean = true,
    val knockEnabled: Boolean = true,
    /** 隐藏"状态模拟"入口是否已解锁（连点版本号 7 次）。 */
    val debugUnlocked: Boolean = false,
    val debugMode: DebugConnectionMode = DebugConnectionMode.DIRECT,
    // ── FR-12 主动性级别节 ──
    /** 主动性节可操作的角色清单（快照只读渲染）。 */
    val proactivityRoles: List<RoleCard> = emptyList(),
    /** 角色选择 chip 行当前选中角色 id（默认首位角色；空快照取 ""）。 */
    val proactivityRoleId: String = "",
    /** 每角色主动性档位：内存态 mock，默认 moderate 镜像桌面；进程重启即回默认（不做持久化）。 */
    val proactivityLevels: Map<String, ProactivityLevel> = emptyMap(),
) {
    /** 当前选中角色的档位（分段控件回显）。 */
    val selectedProactivity: ProactivityLevel
        get() = proactivityLevels[proactivityRoleId] ?: ProactivityLevel.MODERATE

    /** 应用快照角色列表（冷启动无快照取空，避免 `roles.first().id` 崩溃）；
     *  保留已有角色的档位，新角色默认 moderate。 */
    fun applyRoles(roles: List<RoleCard>): SettingsUiState = copy(
        proactivityRoles = roles,
        proactivityRoleId = if (roles.any { it.id == proactivityRoleId }) proactivityRoleId
            else roles.firstOrNull()?.id ?: "",
        proactivityLevels = roles.associate { it.id to (proactivityLevels[it.id] ?: ProactivityLevel.MODERATE) },
    )

    companion object {
        fun sample() = SettingsUiState(
            proactivityRoles = com.egosync.companion.ui.previewRoles,
            proactivityRoleId = com.egosync.companion.ui.previewRoles.first().id,
            proactivityLevels = com.egosync.companion.ui.previewRoles.associate { it.id to ProactivityLevel.MODERATE },
        )
    }
}

/**
 * 设置页状态：主题切换 / 通知级别（应用内语义）/ 配对设备 / 解除配对 / 状态模拟。
 */
class SettingsViewModel(
    private val container: AppModelContainer,
) : ViewModel() {

    private val _uiState = MutableStateFlow(
        SettingsUiState(themeMode = container.themeMode.value).applyRoles(container.snapshotStore.roles)
    )
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    init {
        // AC2：快照全量替换即时刷新角色清单（保留内存态主动性档位，新角色默认 moderate）
        viewModelScope.launch {
            container.snapshotStore.state.collect { state ->
                when {
                    // AC2：快照全量替换即时刷新角色清单（保留内存态主动性档位，新角色默认 moderate）
                    state.loaded -> _uiState.update { it.applyRoles(container.snapshotStore.roles) }
                    // unpair/密钥失效自愈（store.clear 不导航）：清空角色清单与档位（评审 P2）
                    else -> _uiState.update { it.applyRoles(emptyList()) }
                }
            }
        }
    }

    private var versionTaps = 0

    fun setTheme(mode: ThemeMode) {
        container.setThemeMode(mode)
        _uiState.update { it.copy(themeMode = mode) }
    }

    fun toggleNoticeLevel(level: com.egosync.companion.sync.NoticeLevel) {
        _uiState.update { state ->
            when (level) {
                com.egosync.companion.sync.NoticeLevel.WHISPER ->
                    state.copy(whisperEnabled = !state.whisperEnabled)
                com.egosync.companion.sync.NoticeLevel.TAP ->
                    state.copy(tapEnabled = !state.tapEnabled)
                com.egosync.companion.sync.NoticeLevel.KNOCK ->
                    state.copy(knockEnabled = !state.knockEnabled)
            }
        }
    }

    /** FR-12：切换主动性节内选中的角色（分段控件回显该角色当前档位）。 */
    fun selectProactivityRole(roleId: String) {
        _uiState.update { it.copy(proactivityRoleId = roleId) }
    }

    /** FR-12：设置当前选中角色的主动性档位；仅影响该角色，其余角色档位不变。
     *  冷启动快照未加载时无选中角色：档位无处落，写入 "" 键会被首个 applyRoles
     *  丢弃（用户选择静默回退 moderate，评审 P7）——直接忽略。 */
    fun setProactivityLevel(level: ProactivityLevel) {
        _uiState.update { state ->
            if (state.proactivityRoleId.isEmpty()) return@update state
            state.copy(proactivityLevels = state.proactivityLevels + (state.proactivityRoleId to level))
        }
    }

    /** Debug 预览：四态手动切换，驱动降级态与操作禁用态实时变化。 */
    fun selectDebugMode(mode: DebugConnectionMode) {
        container.connection.setDebugMode(mode)
        _uiState.update { it.copy(debugMode = mode) }
    }

    /** 连点版本号 7 次解锁隐藏入口（Android 开发者选项惯例）。 */
    fun onVersionTapped(): String? {
        versionTaps++
        val remaining = VERSION_TAPS_TO_UNLOCK - versionTaps
        return if (_uiState.value.debugUnlocked) {
            null
        } else if (remaining <= 0) {
            _uiState.update { it.copy(debugUnlocked = true) }
            "状态模拟已开启"
        } else {
            "再点 $remaining 次开启状态模拟"
        }
    }

    fun unpair() {
        container.unpair()
    }

    private companion object {
        const val VERSION_TAPS_TO_UNLOCK = 7
    }
}
