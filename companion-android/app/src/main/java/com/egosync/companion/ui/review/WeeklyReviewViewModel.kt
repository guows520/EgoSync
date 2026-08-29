package com.egosync.companion.ui.review

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.BigRockPlanItem
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** 周复盘阶段（FR-17，镜像桌面 WeeklyReviewModal phase: 'review' | 'plan'）。 */
enum class ReviewPhase { REVIEW, PLAN }

/** 角色规划态（镜像桌面 RolePlanState { roleId, items: string[] }）。 */
data class RolePlanState(val roleId: String, val items: List<String>)

/**
 * 周复盘 UI 状态（FR-17）：阶段 + 规划输入态 + 建议加载/保存态。
 * 纯函数镜像桌面 useBigRockPlanning（状态机）+ WeeklyReviewModal L80-115（规划操作）。
 */
data class WeeklyReviewUiState(
    val phase: ReviewPhase = ReviewPhase.REVIEW,
    /** 每角色规划项；初始化每角色 1 个空行（镜像桌面 planStates init items: ['']）。 */
    val planStates: List<RolePlanState> = emptyList(),
    /** 建议映射；null=未加载完成（镜像桌面 suggestions: RoleBigRockSuggestions[] | null）。 */
    val suggestions: Map<String, List<String>>? = null,
    val isLoadingSuggestions: Boolean = false,
    val isSaving: Boolean = false,
) {
    /** 取某角色建议；null/缺席返回空（空触发「手动填写本周大石头」占位分支）。 */
    fun suggestionsOf(roleId: String): List<String> = suggestions?.get(roleId).orEmpty()

    /** 修改某角色第 index 行文案（镜像桌面 updatePlanItems）。 */
    fun updateItem(roleId: String, index: Int, text: String): WeeklyReviewUiState = copy(
        planStates = planStates.map { ps ->
            if (ps.roleId != roleId) ps
            else ps.copy(items = ps.items.mapIndexed { i, t -> if (i == index) text else t })
        },
    )

    /** 移除某角色第 index 行（UI 仅在 items.size>1 时渲染移除按钮，镜像桌面 items.length>1 才显示 X）。 */
    fun removeItem(roleId: String, index: Int): WeeklyReviewUiState = copy(
        planStates = planStates.map { ps ->
            if (ps.roleId != roleId) ps
            else ps.copy(items = ps.items.filterIndexed { i, _ -> i != index })
        },
    )

    /** 添加空行（镜像桌面 updatePlanItems(roleId, [...ps.items, ''])）。 */
    fun addItem(roleId: String): WeeklyReviewUiState = copy(
        planStates = planStates.map { ps ->
            if (ps.roleId != roleId) ps else ps.copy(items = ps.items + "")
        },
    )

    /**
     * 采纳建议（镜像桌面 adoptSuggestion L84-97）：填首个空行，无空行则追加。
     * 二者必须保持「要么改写空行不新增、要么追加」的语义——只追加不找空行会让空行
     * 永远滞留在列表头部（用户视觉上看到空行夹在已填项之间）。
     */
    fun adoptSuggestion(roleId: String, suggestion: String): WeeklyReviewUiState = copy(
        planStates = planStates.map { ps ->
            if (ps.roleId != roleId) return@map ps
            val emptyIdx = ps.items.indexOfFirst { it.trim().isEmpty() }
            if (emptyIdx >= 0) {
                ps.copy(items = ps.items.mapIndexed { i, t -> if (i == emptyIdx) suggestion else t })
            } else {
                ps.copy(items = ps.items + suggestion)
            }
        },
    )

    /** 收集非空规划项（镜像桌面 handleConfirmPlan 过滤 title.trim() 后构造 BigRockPlanItem）。 */
    fun plannedItems(): List<BigRockPlanItem> = planStates.flatMap { ps ->
        ps.items.mapNotNull { raw ->
            val title = raw.trim()
            if (title.isEmpty()) null else BigRockPlanItem(ps.roleId, title)
        }
    }
}

/**
 * 周复盘 VM（FR-17）：规划输入为本地内存态；建议/保存动作待指令通道（13.3）。
 * 复盘成绩单数据（review）由 Route 经快照取数直传 Screen。
 */
class WeeklyReviewViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        WeeklyReviewUiState(
            planStates = container.snapshotStore.roles.map { RolePlanState(it.id, listOf("")) },
        )
    )
    val uiState: StateFlow<WeeklyReviewUiState> = _uiState.asStateFlow()

    init {
        // AC2 + 评审 P12：planStates 随快照角色重建——VM 可能构造于快照到达前（roles 空 →
        // planStates 空），或 STATE_DELTA 新增角色；缺失角色的 updateItem/addItem/removeItem
        // 全为 no-op，用户键入会被静默拒绝。已有角色的输入按 roleId 保留。
        viewModelScope.launch {
            container.snapshotStore.state.collect { state ->
                if (state.loaded) {
                    _uiState.update { st ->
                        val existing = st.planStates.associateBy { it.roleId }
                        st.copy(
                            planStates = container.snapshotStore.roles.map { role ->
                                existing[role.id] ?: RolePlanState(role.id, listOf(""))
                            }
                        )
                    }
                } else {
                    // unpair/密钥失效自愈（store.clear 不导航）：规划态一并清空（评审 P2）
                    _uiState.update { it.copy(planStates = emptyList()) }
                }
            }
        }
    }

    /**
     * 进 plan 阶段（镜像桌面 phase==='plan'）：建议属生成性内容，快照口径无
     * （§5 裁决）——置空态「待接指令通道」，可先手动填写；不再模拟 LLM 加载延时。
     */
    fun enterPlanPhase() {
        _uiState.update {
            it.copy(
                phase = ReviewPhase.PLAN,
                isLoadingSuggestions = false,
                suggestions = emptyMap(),
            )
        }
    }

    fun enterReviewPhase() {
        _uiState.update { it.copy(phase = ReviewPhase.REVIEW) }
    }

    fun updateItem(roleId: String, index: Int, text: String) = _uiState.update { it.updateItem(roleId, index, text) }
    fun removeItem(roleId: String, index: Int) = _uiState.update { it.removeItem(roleId, index) }
    fun addItem(roleId: String) = _uiState.update { it.addItem(roleId) }
    fun adoptSuggestion(roleId: String, suggestion: String) = _uiState.update { it.adoptSuggestion(roleId, suggestion) }

    /**
     * 确认规划（镜像桌面 handleConfirmPlan L99-115：过滤空白 → savePlan → onClose）。
     * 全空白时点击无效果（镜像桌面 items.length===0 return）；保存中重复点击跳过。
     * mock 用 delay 模拟保存；完成后调 [onSaved]（Route 接 pop 返回，等价桌面关 Modal）。
     */
    fun confirmPlan(onSaved: () -> Unit) {
        val state = _uiState.value
        if (state.isSaving) return
        if (state.plannedItems().isEmpty()) return
        _uiState.update { it.copy(isSaving = true) }
        viewModelScope.launch {
            delay(SAVE_DELAY_MILLIS)
            _uiState.update { it.copy(isSaving = false) }
            onSaved()
        }
    }

    private companion object {
        /** mock 保存耗时（毫秒）：镜像桌面 isSaving 期间「保存中...」过渡。 */
        const val SAVE_DELAY_MILLIS = 1_000L
    }
}
