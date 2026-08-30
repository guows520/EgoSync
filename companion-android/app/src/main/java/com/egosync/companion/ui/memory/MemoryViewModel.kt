package com.egosync.companion.ui.memory

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.command.CommandEnvelope
import com.egosync.companion.command.CommandException
import com.egosync.companion.command.CommandSender
import com.egosync.companion.sync.MemoryCategory
import com.egosync.companion.sync.MemoryItem
import com.egosync.companion.sync.MemorySourceMessage
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.json.JSONObject

data class MemoryUiState(
    val role: RoleCard? = null,
    /** 当前角色记忆（指令通道现查，不接快照——AC5 防呆不变）。 */
    val memories: List<MemoryItem> = emptyList(),
    /** 类别筛选；null=全部（镜像桌面 category undefined）。 */
    val category: MemoryCategory? = null,
    /** 展开来源区的记忆 id（FR-8 互斥单开）；null=全收起。 */
    val expandedMemoryId: String? = null,
    /** 遗忘确认面板所记记忆 id（FR-9）；null=无确认中。 */
    val confirmingMemoryId: String? = null,
    /** 列表加载中（进屏/筛选切换/重试）。 */
    val loading: Boolean = false,
    /** 列表加载错误（null=无）；带重试入口。 */
    val error: String? = null,
    /** 来源消息（按记忆 id 懒加载，展开时现查）。 */
    val sourcesByMemoryId: Map<String, List<MemorySourceMessage>> = emptyMap(),
    /** 来源加载中的记忆 id 集。 */
    val loadingSourceIds: Set<String> = emptySet(),
) {
    /** 可见记忆：task_status 永不显示（镜像桌面 visibleMemories 过滤）+ 类别筛选。 */
    val visibleMemories: List<MemoryItem>
        get() = memories.filter {
            it.category != MemoryCategory.TASK_STATUS &&
                (category == null || it.category == category)
        }

    companion object {
        fun sample(roleId: String = "role-pm") = MemoryUiState(
            // Preview 样例：记忆内容永不入快照（AC5），sample 呈现空态占位
            role = com.egosync.companion.ui.previewRoles.find { it.id == roleId },
            memories = emptyList(),
        )
    }
}

/**
 * 记忆屏状态机（FR-8 查询溯源 / FR-9 选择性遗忘；Story 13.3 指令通道换装）。
 * 13.2 契约保持：记忆内容永不进入快照（AC5）——`memoriesOf` 恒空，
 * 列表/来源/遗忘全部经 `COMMAND(memory.list / memory.sources / memory.forget)`
 * 现查现显。角色卡随快照即时刷新（AC2）；查询失败有重试入口，不悬挂。
 */
class MemoryViewModel(
    private val store: SnapshotStore,
    roleId: String,
    private val commands: CommandSender?,
    private val onError: (String) -> Unit = {},
) : ViewModel() {

    private val targetRoleId: String = roleId

    /** 在途列表现查任务：新查询取消旧的——快速切换类别时旧回执不得覆盖新结果（评审 C17）。 */
    private var reloadJob: kotlinx.coroutines.Job? = null

    private val _uiState = MutableStateFlow(
        MemoryUiState(role = store.roles.find { it.id == roleId })
    )
    val uiState: StateFlow<MemoryUiState> = _uiState.asStateFlow()

    init {
        // AC2：快照全量替换即时刷新角色卡（记忆列表本身不受影响，AC5 恒空）
        viewModelScope.launch {
            store.state.collect { state ->
                when {
                    state.loaded -> _uiState.update {
                        it.copy(role = store.roles.find { r -> r.id == targetRoleId })
                    }
                    // unpair/密钥失效自愈（store.clear 不导航）：角色卡一并清空（评审 P2）
                    else -> _uiState.update { it.copy(role = null) }
                }
            }
        }
        reload()
    }

    /** 列表现查（进屏/筛选切换/失败重试共用）。 */
    fun reload() {
        val category = _uiState.value.category
        _uiState.update { it.copy(loading = true, error = null) }
        // 取消在途查询：旧回执晚于新回执到达时覆盖最新列表，显示错误记忆（评审 C17）
        reloadJob?.cancel()
        reloadJob = viewModelScope.launch {
            val sender = commands ?: run {
                _uiState.update { it.copy(loading = false, error = "桌面引擎不可达") }
                return@launch
            }
            try {
                val result = sender.execute(
                    "memory.list",
                    CommandEnvelope.buildParams(
                        listOf(
                            "roleId" to targetRoleId,
                            "category" to category?.name?.lowercase(),
                        ),
                    ),
                )
                val arr = result.optJSONArray("memories")
                val memories = if (arr == null) emptyList() else {
                    (0 until arr.length()).mapNotNull { i -> arr.optJSONObject(i)?.toMemoryItem() }
                }
                _uiState.update { it.copy(loading = false, error = null, memories = memories) }
            } catch (e: CommandException) {
                // 现查失败：保留旧列表 + 错误态（重试入口仍可用），不悬挂
                _uiState.update { it.copy(loading = false, error = e.message ?: "查询失败") }
            }
        }
    }

    /** 类别筛选（镜像桌面 setSelectedCategory；切换时重置展开/确认态并现查）。 */
    fun setCategory(category: MemoryCategory?) {
        _uiState.update {
            it.copy(category = category, expandedMemoryId = null, confirmingMemoryId = null)
        }
        reload()
    }

    /** 展开/收起来源区（镜像桌面 toggleSource：互斥单开）；展开时懒加载来源消息。 */
    fun toggleSource(memoryId: String) {
        val current = _uiState.value
        if (current.expandedMemoryId == memoryId) {
            _uiState.update { it.copy(expandedMemoryId = null) }
            return
        }
        _uiState.update { it.copy(expandedMemoryId = memoryId) }
        if (current.sourcesByMemoryId.containsKey(memoryId)) return
        loadSources(memoryId)
    }

    private fun loadSources(memoryId: String) {
        _uiState.update { it.copy(loadingSourceIds = it.loadingSourceIds + memoryId) }
        viewModelScope.launch {
            val sender = commands ?: run {
                _uiState.update { it.copy(loadingSourceIds = it.loadingSourceIds - memoryId) }
                return@launch
            }
            try {
                val result = sender.execute(
                    "memory.sources",
                    CommandEnvelope.buildParams(listOf("memoryId" to memoryId)),
                )
                val arr = result.optJSONArray("messages")
                val messages = if (arr == null) emptyList() else {
                    (0 until arr.length()).mapNotNull { i -> arr.optJSONObject(i)?.toSourceMessage() }
                }
                _uiState.update {
                    it.copy(
                        sourcesByMemoryId = it.sourcesByMemoryId + (memoryId to messages),
                        loadingSourceIds = it.loadingSourceIds - memoryId,
                    )
                }
            } catch (e: CommandException) {
                _uiState.update { it.copy(loadingSourceIds = it.loadingSourceIds - memoryId) }
                // 内联错误区（带重试入口）是唯一反馈——不再叠发全局 snackbar（评审 C18）
            }
        }
    }

    /** 来源加载失败后的显式重试（内联「重试」入口；失败不缓存，直接重查）。 */
    fun retrySources(memoryId: String) {
        if (memoryId in _uiState.value.loadingSourceIds) return
        loadSources(memoryId)
    }

    fun openForgetConfirm(memoryId: String) {
        _uiState.update { it.copy(confirmingMemoryId = memoryId) }
    }

    /** 再想想：仅收起确认面板（镜像桌面 cancelForget）。 */
    fun cancelForget(memoryId: String) {
        _uiState.update { state ->
            if (state.confirmingMemoryId == memoryId) state.copy(confirmingMemoryId = null) else state
        }
    }

    /** 确认遗忘（FR-9）：`COMMAND(memory.forget)` 桌面执行；ack 后移出列表并清理展开/确认态。 */
    fun confirmForget(memoryId: String) {
        viewModelScope.launch {
            val sender = commands ?: run {
                onError("桌面引擎不可达")
                return@launch
            }
            try {
                sender.execute(
                    "memory.forget",
                    CommandEnvelope.buildParams(listOf("memoryId" to memoryId)),
                )
            } catch (e: CommandException) {
                // 遗忘失败：保留记忆卡（可重试），不静默假装已遗忘
                onError("遗忘失败：${e.message}")
                return@launch
            }
            _uiState.update {
                it.copy(
                    memories = it.memories.filterNot { memory -> memory.id == memoryId },
                    confirmingMemoryId = null,
                    expandedMemoryId = null,
                )
            }
        }
    }

    // ── 桌面 JSON → UI 模型（记忆仅公开溯源字段） ─────────────────────

    private fun JSONObject.toMemoryItem(): MemoryItem? {
        val id = optString("id")
        if (id.isEmpty()) return null
        val category = when (optString("category")) {
            "fact" -> MemoryCategory.FACT
            "preference" -> MemoryCategory.PREFERENCE
            "cognition_update" -> MemoryCategory.COGNITION_UPDATE
            "task_status" -> MemoryCategory.TASK_STATUS
            else -> return null // 未知类别：不造假映射，跳过（桌面新增类别时再扩）
        }
        return MemoryItem(
            id = id,
            roleId = optString("roleId"),
            category = category,
            content = optString("content"),
            createdAt = optString("createdAt"),
        )
    }

    private fun JSONObject.toSourceMessage(): MemorySourceMessage? {
        val id = optString("id")
        if (id.isEmpty()) return null
        return MemorySourceMessage(
            id = id,
            role = optString("role", "user"),
            content = optString("content"),
            createdAt = optString("createdAt"),
            isSource = optBoolean("isSource", true),
        )
    }
}
