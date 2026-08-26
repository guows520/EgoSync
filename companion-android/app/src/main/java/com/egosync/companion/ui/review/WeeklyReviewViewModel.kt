package com.egosync.companion.ui.review

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.BigRockPlanItem
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
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
    val planStates: List<RolePlanState> = SnapshotStore.roles.map { RolePlanState(it.id, listOf("")) },
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
 * 周复盘 VM（FR-17）：mock 跑建议加载与保存状态机，纯内存态，进程重启还原。
 * 接真实连接层后，加载/保存动作将走 SNAPSHOT/COMMAND 帧；UI 层零改动。
 */
class WeeklyReviewViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(WeeklyReviewUiState())
    val uiState: StateFlow<WeeklyReviewUiState> = _uiState.asStateFlow()

    /** 在途建议加载协程：重进 plan 阶段前取消，防陈旧协程提前写状态打破新一轮占位时序（同 ChatViewModel.streamJob 模式）。 */
    private var loadJob: Job? = null

    /**
     * 进 plan 阶段并加载建议（镜像桌面 phase==='plan' useEffect 每次 loadSuggestions）：
     * 每次「规划下周大石头」入口均先显「正在思考建议...」占位，再延时出建议。
     */
    fun enterPlanPhase() {
        loadJob?.cancel()
        _uiState.update {
            it.copy(phase = ReviewPhase.PLAN, isLoadingSuggestions = true, suggestions = null)
        }
        loadJob = viewModelScope.launch {
            delay(LOAD_DELAY_MILLIS)
            _uiState.update {
                it.copy(isLoadingSuggestions = false, suggestions = container.snapshotStore.bigRockSuggestions)
            }
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
        /** mock 建议加载耗时（毫秒）：镜像桌面 LLM「正在思考建议...」过渡。 */
        const val LOAD_DELAY_MILLIS = 1_500L
        /** mock 保存耗时（毫秒）：镜像桌面 isSaving 期间「保存中...」过渡。 */
        const val SAVE_DELAY_MILLIS = 1_000L
    }
}
