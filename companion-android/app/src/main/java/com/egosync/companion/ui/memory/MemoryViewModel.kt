package com.egosync.companion.ui.memory

import androidx.lifecycle.ViewModel
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.MemoryCategory
import com.egosync.companion.sync.MemoryItem
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class MemoryUiState(
    val role: RoleCard? = null,
    /** 剩余记忆（FR-9 遗忘后从列表移除；mock 内存态，进程重启还原）。 */
    val memories: List<MemoryItem> = emptyList(),
    /** 类别筛选；null=全部（镜像桌面 category undefined）。 */
    val category: MemoryCategory? = null,
    /** 展开来源区的记忆 id（FR-8 互斥单开）；null=全收起。 */
    val expandedMemoryId: String? = null,
    /** 遗忘确认面板所记记忆 id（FR-9）；null=无确认中。 */
    val confirmingMemoryId: String? = null,
) {
    /** 可见记忆：task_status 永不显示（镜像桌面 visibleMemories 过滤）+ 类别筛选。 */
    val visibleMemories: List<MemoryItem>
        get() = memories.filter {
            it.category != MemoryCategory.TASK_STATUS &&
                (category == null || it.category == category)
        }

    companion object {
        fun sample(roleId: String = "role-pm") = MemoryUiState(
            role = SnapshotStore.roles.find { it.id == roleId },
            memories = SnapshotStore.memoriesOf(roleId),
        )
    }
}

/**
 * 记忆屏状态机（FR-8 查询溯源 / FR-9 选择性遗忘）：按角色载入 mock 记忆，
 * 类别筛选 / 来源展开 / 遗忘确认流。遗忘为内存态移除（mock 无后端调用）；
 * 接入真实连接层后改为 COMMAND 帧驱动。
 */
class MemoryViewModel(container: AppModelContainer, roleId: String) : ViewModel() {

    private val _uiState = MutableStateFlow(
        MemoryUiState(
            role = container.snapshotStore.roles.find { it.id == roleId },
            memories = container.snapshotStore.memoriesOf(roleId),
        )
    )
    val uiState: StateFlow<MemoryUiState> = _uiState.asStateFlow()

    /** 类别筛选（镜像桌面 setSelectedCategory；切换时重置展开/确认态，镜像桌面 useEffect）。 */
    fun setCategory(category: MemoryCategory?) {
        _uiState.update {
            it.copy(category = category, expandedMemoryId = null, confirmingMemoryId = null)
        }
    }

    /** 展开/收起来源区（镜像桌面 toggleSource：互斥单开）。 */
    fun toggleSource(memoryId: String) {
        _uiState.update {
            it.copy(
                expandedMemoryId = if (it.expandedMemoryId == memoryId) null else memoryId,
            )
        }
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

    /** 确认遗忘：内存态移除 + 清理展开/确认态（镜像桌面 finalizeForgotten）。 */
    fun confirmForget(memoryId: String) {
        _uiState.update {
            it.copy(
                memories = it.memories.filterNot { memory -> memory.id == memoryId },
                confirmingMemoryId = null,
                expandedMemoryId = null,
            )
        }
    }
}
