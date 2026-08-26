package com.egosync.companion.ui.tasks

import androidx.lifecycle.ViewModel
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.ui.theme.Quadrant
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class TasksUiState(
    val tasks: List<TaskItem> = SnapshotStore.tasks,
) {
    fun grouped(): Map<Quadrant, List<TaskItem>> = Quadrant.entries.associateWith { q ->
        tasks.filter { it.quadrant == q }
    }

    companion object {
        fun sample() = TasksUiState()
    }
}

/**
 * 四象限任务 mock 状态：勾选完成 = 指令交桌面引擎执行（FR-41），
 * 原型仅本地翻转；真实层将改为 COMMAND 帧发送。
 */
class TasksViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        TasksUiState(tasks = container.snapshotStore.tasks)
    )
    val uiState: StateFlow<TasksUiState> = _uiState.asStateFlow()

    fun toggleTask(taskId: String) {
        _uiState.update { state ->
            state.copy(
                tasks = state.tasks.map {
                    if (it.id == taskId) it.copy(done = !it.done) else it
                }
            )
        }
    }
}
