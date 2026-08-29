package com.egosync.companion.ui.tasks

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.sync.CreateTaskInput
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TASK_OWNER_BUTLER_KEY
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.sync.TaskOwner
import com.egosync.companion.sync.ownerKey
import com.egosync.companion.ui.theme.Quadrant
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class TasksUiState(
    val tasks: List<TaskItem> = emptyList(),
    /** 后台正在异步智能分类的任务 id，在卡片上显示「分类中」过渡态（镜像 useTasks.ts classifyingIds）。 */
    val classifyingIds: Set<String> = emptySet(),
    /** 象限筛选：null = 全部（镜像桌面 TaskOverviewTab quadrantFilter: TaskQuadrant | 'all'）。 */
    val quadrantFilter: Quadrant? = null,
    /** 归属筛选（多选排除语义，镜像桌面 TaskOverviewTab.tsx:168 deselectedOwners）：含 key = 该归属被排除。 */
    val deselectedOwners: Set<String> = emptySet(),
    /** 只看大石头（镜像桌面 showBigRocksOnly）：与象限/归属筛选叠加。 */
    val showBigRocksOnly: Boolean = false,
) {
    fun grouped(): Map<Quadrant, List<TaskItem>> {
        val visible = tasks.filter { task ->
            // 分类中任务不参与象限过滤：智能判断占位象限（Q3）≠ 最终归类（Q2），
            // 若参与过滤，选中目标象限的用户在 4 秒分类窗口内完全看不到刚建的任务，
            // 违背「立即上屏并挂徽章」契约（大石头/归属是用户刚选定的真实属性，仍参与过滤）
            (quadrantFilter == null || task.quadrant == quadrantFilter || task.id in classifyingIds) &&
                (!showBigRocksOnly || task.bigRock) &&
                !deselectedOwners.contains(task.ownerKey())
        }
        return Quadrant.entries.associateWith { q -> visible.filter { it.quadrant == q } }
    }

    /** task:classified 事件到达：用已归类任务替换卡片并清除「分类中」标记（镜像 useTasks.ts:39-45）。 */
    fun applyClassified(classified: TaskItem): TasksUiState = copy(
        tasks = tasks.map { if (it.id == classified.id) classified else it },
        classifyingIds = classifyingIds - classified.id,
    )

    /**
     * 切换某归属的勾选（镜像桌面 TaskOverviewTab.tsx:189-199 toggleOwner）：
     * 默认全选态下点单个归属 = 只看它（排除其余全部）；已有排除时按加入/移出排除集处理。
     */
    fun toggleOwner(ownerKey: String, allOwnerKeys: Set<String>): TasksUiState = copy(
        deselectedOwners = when {
            deselectedOwners.isEmpty() -> allOwnerKeys - ownerKey
            ownerKey in deselectedOwners -> deselectedOwners - ownerKey
            else -> deselectedOwners + ownerKey
        },
    )

    /** 全部归属总开关（镜像桌面 toggleAllOwners:201-205）：默认全选 → 全部排除；已有排除 → 恢复全选。 */
    fun toggleAllOwners(allOwnerKeys: Set<String>): TasksUiState = copy(
        deselectedOwners = if (deselectedOwners.isEmpty()) allOwnerKeys else emptySet(),
    )

    /**
     * 新建任务落表（镜像 useTasks.ts:129-140 createTask）：任务立即上屏；
     * 智能判断（pendingClassify=true）先标记「分类中」，等 task:classified 事件经 [applyClassified] 归类。
     */
    fun applyCreated(created: TaskItem, pendingClassify: Boolean): TasksUiState = copy(
        tasks = tasks + created,
        classifyingIds = if (pendingClassify) classifyingIds + created.id else classifyingIds,
    )

    companion object {
        fun sample() = TasksUiState(tasks = com.egosync.companion.ui.previewTasks)
    }
}

/**
 * 四象限任务 mock 状态：勾选完成 = 指令交桌面引擎执行（FR-41），
 * 原型仅本地翻转；真实层将改为 COMMAND 帧发送。
 */
class TasksViewModel(private val store: SnapshotStore) : ViewModel() {

    private val _uiState = MutableStateFlow(
        TasksUiState(tasks = store.tasks + seedTask)
    )
    val uiState: StateFlow<TasksUiState> = _uiState.asStateFlow()

    /** 快照同一性守卫（评审 P1）：仅快照对象变更（首次加载/STATE_DELTA 全量替换）才重建
     *  任务列表——无守卫时任何 state 重发都会把已分类的 seedTask 回滚为 Q3 占位、
     *  nextTaskId 回退（applyClassified 按 id 替换会改错/找不到卡片，正是其注释
     *  自述要防的故障模式）。 */
    private var lastSnapshot: DesktopSnapshot? = store.state.value.snapshot

    /** 新建任务 id 序列起点：从快照任务（含 seed）最大 t-N 推导——新增种子免手工同步，
     *  否则静默 id 冲突会让 applyClassified 按 id 替换时改错卡片（原 companion 静态变量换装为实例态）。 */
    private var nextTaskId = deriveNextTaskId()

    private fun deriveNextTaskId(): Int =
        (store.tasks + seedTask)
            .maxOf { it.id.removePrefix("t-").toIntOrNull() ?: 0 } + 1

    init {
        // AC2：快照全量替换即时刷新任务列表（本地暂存〔勾选/新建〕被覆盖为已知过渡态，
        // 指令通道 13.3 收口）；classifyingIds 属本地交互态，保留不清
        viewModelScope.launch {
            store.state.collect { state ->
                when {
                    state.loaded && state.snapshot !== lastSnapshot -> {
                        lastSnapshot = state.snapshot
                        rebuildTasks()
                    }
                    // unpair/密钥失效自愈（store.clear 不导航）：任务列表与分类中态
                    // 一并清空，不残留已解配桌面的数据（评审 P2）
                    !state.loaded -> {
                        lastSnapshot = null
                        _uiState.update { it.copy(tasks = emptyList(), classifyingIds = emptySet()) }
                    }
                }
            }
        }
        // FR-23 mock：模拟桌面「新建任务未指定象限 → 后台异步分类」过渡态——
        // seed 一条占位象限（Q3）任务并标记分类中，延时后模拟 task:classified
        // 事件到达，归类到 Q2 并清除标记。纯内存态，进程重启还原。
        _uiState.update { state ->
            state.copy(classifyingIds = setOf(seedTask.id))
        }
        viewModelScope.launch {
            delay(CLASSIFY_DELAY_MILLIS)
            // 事件载荷从当前卡片构造（只改象限）：桌面事件载荷来自后端（已知完成态），
            // 移动 mock 若用陈旧常量整卡替换，会回滚分类窗口内用户的本地勾选。
            // 任务已不在列表（mock 防御，正常不可达）：不做任何事，避免过期快照复活
            _uiState.update { state ->
                val current = state.tasks.firstOrNull { it.id == seedTask.id } ?: return@update state
                state.applyClassified(current.copy(quadrant = Quadrant.Q2))
            }
        }
    }

    /** 用快照 tasks 域重建列表：seedTask 若已被 task:classified 归类则保持已分类象限
     *  不回退为 Q3 占位；nextTaskId 取历史最大，不因重建回退造成 id 复用。 */
    private fun rebuildTasks() {
        val classifiedSeed = _uiState.value.tasks.firstOrNull { it.id == seedTask.id }
        val seed = if (classifiedSeed != null && classifiedSeed.quadrant != seedTask.quadrant) {
            classifiedSeed
        } else {
            seedTask
        }
        _uiState.update { it.copy(tasks = store.tasks + seed) }
        nextTaskId = maxOf(nextTaskId, deriveNextTaskId())
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

    /** 归属筛选切换（镜像桌面 toggleOwner 语义；选项集 = 管家 + 全部角色，对齐桌面 ownerOptions 顺序）。 */
    fun toggleOwner(ownerKey: String) {
        _uiState.update { it.toggleOwner(ownerKey, ownerKeys()) }
    }

    /** 归属「全部」开关（镜像桌面 toggleAllOwners 语义）。 */
    fun toggleAllOwners() {
        _uiState.update { it.toggleAllOwners(ownerKeys()) }
    }

    fun toggleBigRocksOnly() {
        _uiState.update { it.copy(showBigRocksOnly = !it.showBigRocksOnly) }
    }

    /**
     * 新建任务（FR-23，镜像桌面 TaskModal handleSubmit → useTasks.createTask）：
     * 指定象限直接归组；未指定（智能判断）复用 classifyingIds 过渡态，延时后模拟
     * task:classified 事件归类。原型在 mock 层本地追加（同 seedTask 先例）；
     * 真实层将改为 COMMAND 帧发送桌面引擎。title 由表单层校验非空后传入。
     */
    fun createTask(input: CreateTaskInput) {
        val id = "t-$nextTaskId"
        nextTaskId += 1
        // 归属展示名（桌面 CrossRoleTask.roleName 由后端联表给出；mock 就地查 roles）
        val roleName = if (input.ownerType == TaskOwner.BUTLER) "管家"
        else store.roles.firstOrNull { it.id == input.roleId }?.name ?: "未知角色"
        val created = TaskItem(
            id = id,
            title = input.title.trim(),
            // 智能判断占位象限（同 seedTask 约定），事件到达后归类
            quadrant = input.quadrant ?: Quadrant.Q3,
            roleName = roleName,
            ownerType = input.ownerType,
            roleId = input.roleId,
            due = input.due?.trim()?.takeIf { it.isNotEmpty() },
            bigRock = input.bigRock,
        )
        _uiState.update { it.applyCreated(created, pendingClassify = input.quadrant == null) }
        if (input.quadrant == null) {
            viewModelScope.launch {
                delay(CLASSIFY_DELAY_MILLIS)
                // 事件载荷从当前卡片构造（只改象限），不回滚分类窗口内的本地勾选——同 init seed 约定；
                // mock 分类结果固定 Q2（原型简化，真实层由桌面引擎分类后经事件下发）
                _uiState.update { state ->
                    val current = state.tasks.firstOrNull { it.id == id } ?: return@update state
                    state.applyClassified(current.copy(quadrant = Quadrant.Q2))
                }
            }
        }
    }

    /** 归属选项键全集：管家 + 全部角色 id（镜像桌面 ownerOptions 的 key 集）。 */
    private fun ownerKeys(): Set<String> =
        setOf(TASK_OWNER_BUTLER_KEY) + store.roles.map { it.id }

    private companion object {
        /** 模拟后台分类耗时（毫秒）：进屏后约 4 秒收到 task:classified 事件。 */
        const val CLASSIFY_DELAY_MILLIS = 4_000L

        /** seed 任务：语义为「从对话/速记新建、未指定象限」，占位象限 Q3，事件到达后归类 Q2。 */
        val seedTask = TaskItem(
            id = "t-9",
            title = "整理客户反馈要点",
            quadrant = Quadrant.Q3,
            roleName = "产品经理",
            ownerType = TaskOwner.ROLE,
            roleId = "role-pm",
            due = null,
            bigRock = false,
        )
    }
}
