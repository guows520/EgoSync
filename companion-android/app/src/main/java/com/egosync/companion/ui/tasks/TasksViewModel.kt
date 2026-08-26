package com.egosync.companion.ui.tasks

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.ui.theme.Quadrant
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class TasksUiState(
    val tasks: List<TaskItem> = SnapshotStore.tasks,
    /** 后台正在异步智能分类的任务 id，在卡片上显示「分类中」过渡态（镜像 useTasks.ts classifyingIds）。 */
    val classifyingIds: Set<String> = emptySet(),
    /** 象限筛选：null = 全部（镜像桌面 TaskOverviewTab quadrantFilter: TaskQuadrant | 'all'）。 */
    val quadrantFilter: Quadrant? = null,
) {
    fun grouped(): Map<Quadrant, List<TaskItem>> {
        val visible = tasks.filter { quadrantFilter == null || it.quadrant == quadrantFilter }
        return Quadrant.entries.associateWith { q -> visible.filter { it.quadrant == q } }
    }

    /** task:classified 事件到达：用已归类任务替换卡片并清除「分类中」标记（镜像 useTasks.ts:39-45）。 */
    fun applyClassified(classified: TaskItem): TasksUiState = copy(
        tasks = tasks.map { if (it.id == classified.id) classified else it },
        classifyingIds = classifyingIds - classified.id,
    )

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

    init {
        // FR-23 mock：模拟桌面「新建任务未指定象限 → 后台异步分类」过渡态——
        // seed 一条占位象限（Q3）任务并标记分类中，延时后模拟 task:classified
        // 事件到达，归类到 Q2 并清除标记。纯内存态，进程重启还原。
        _uiState.update { state ->
            state.copy(
                tasks = state.tasks + seedTask,
                classifyingIds = setOf(seedTask.id),
            )
        }
        viewModelScope.launch {
            delay(CLASSIFY_DELAY_MILLIS)
            // 事件载荷从当前卡片构造（只改象限）：桌面事件载荷来自后端（已知完成态），
            // 移动 mock 若用陈旧常量整卡替换，会回滚分类窗口内用户的本地勾选
            _uiState.update { state ->
                val current = state.tasks.firstOrNull { it.id == seedTask.id } ?: seedTask
                state.applyClassified(current.copy(quadrant = Quadrant.Q2))
            }
        }
    }

    fun toggleTask(taskId: String) {
        _uiState.update { state ->
            state.copy(
                tasks = state.tasks.map {
                    if (it.id == taskId) it.copy(done = !it.done) else it
                }
            )
        }
    }

    fun selectQuadrantFilter(quadrant: Quadrant?) {
        _uiState.update { it.copy(quadrantFilter = quadrant) }
    }

    private companion object {
        /** 模拟后台分类耗时（毫秒）：进屏后约 4 秒收到 task:classified 事件。 */
        const val CLASSIFY_DELAY_MILLIS = 4_000L

        /** seed 任务：语义为「从对话/速记新建、未指定象限」，占位象限 Q3，事件到达后归类 Q2。 */
        val seedTask = TaskItem(
            id = "t-9",
            title = "整理客户反馈要点",
            quadrant = Quadrant.Q3,
            roleName = "产品经理",
            due = null,
            bigRock = false,
        )
    }
}
