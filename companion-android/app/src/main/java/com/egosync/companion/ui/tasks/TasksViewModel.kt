package com.egosync.companion.ui.tasks

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.command.CommandEnvelope
import com.egosync.companion.command.CommandException
import com.egosync.companion.command.CommandSender
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
import org.json.JSONObject

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
            // 分类中任务不参与象限过滤：智能判断占位象限（Q2）≠ 最终归类，
            // 若参与过滤，选中目标象限的用户在分类窗口内完全看不到刚建的任务，
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
 * 四象限任务状态机（Story 13.3 换装：mock 本地翻转 → 指令通道）。
 * 勾选/新建经 `COMMAND(task.toggle/task.create)` 由桌面引擎真实执行：
 * 勾选 = 乐观翻转 + 失败回滚；新建 = 乐观追加（分类中过渡态）+ ack 换真 id，
 * 象限归类经 task:classified → STATE_DELTA 收敛（移除 4s 固定 Q2 mock）。
 */
class TasksViewModel(
    private val store: SnapshotStore,
    private val commands: CommandSender?,
    private val onError: (String) -> Unit = {},
) : ViewModel() {

    private val _uiState = MutableStateFlow(TasksUiState(tasks = store.tasks))
    val uiState: StateFlow<TasksUiState> = _uiState.asStateFlow()

    /** 快照同一性守卫（评审 P1）：仅快照对象变更（首次加载/STATE_DELTA 全量替换）才重建
     *  任务列表——无守卫时任何 state 重发都会回滚已归类的卡片。 */
    private var lastSnapshot: DesktopSnapshot? = store.state.value.snapshot

    /** 新建任务临时 id 序列起点（local- 前缀防与桌面 id 冲突；ack 后替换为真 id）。 */
    private var nextLocalId = 1

    /** 在途乐观勾选（toggle ack 未回）：id → 翻转目标态；快照重建时保持本地翻转。 */
    private val inFlightToggles = mutableMapOf<String, Boolean>()

    init {
        // AC2：快照全量替换即时刷新任务列表；classifyingIds 属本地交互态，保留不清
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
    }

    /** 用快照 tasks 域重建列表；在途乐观勾选保持（快照旧态不回滚翻转）；
     *  分类中收敛仅靠 15s 超时兜底（评审 D1-A：桌面未指定象限默认落库 Q2，
     *  「象限脱离 Q3」判据在真实链路 2s 内即误清，不可用）。 */
    private fun rebuildTasks() {
        val snapshotTasks = store.tasks
        _uiState.update { state ->
            var tasks = snapshotTasks
            // 乐观暂存的任务在快照出现前保留（ack 换 id 前 STATE_DELTA 不含它）
            for (pending in state.tasks) {
                if (pending.id.startsWith(LOCAL_ID_PREFIX) && tasks.none { it.id == pending.id }) {
                    tasks = tasks + pending
                }
            }
            // 在途勾选保持乐观态：toggle 写信号快照（约 2s）到达前，无关 STATE_DELTA
            // 会用桌面旧态覆盖翻转——视觉回滚后再次跳变（评审 C14）
            tasks = tasks.map { t ->
                inFlightToggles[t.id]?.let { flipped -> t.copy(done = flipped) } ?: t
            }
            // 分类中：任务已从快照消失（桌面删除）时清标记防悬挂；归类收敛
            // 由 15s 超时兜底（task:classified 无独立事件通道，经 STATE_DELTA 无法区分）
            val classifying = state.classifyingIds.filter { id ->
                tasks.any { it.id == id }
            }.toSet()
            state.copy(tasks = tasks, classifyingIds = classifying)
        }
    }

    fun toggleTask(taskId: String) {
        val current = _uiState.value.tasks.find { it.id == taskId } ?: return
        // 连点守卫：同任务在途（ack 未回）期间不受理二次翻转——多次乐观翻转与
        // 回滚交错会让本地态偏离桌面且无收敛信号（评审 C14）
        if (inFlightToggles.containsKey(taskId)) return
        val original = current.done
        val flipped = !original
        inFlightToggles[taskId] = flipped
        // 乐观翻转（既有交互即时反馈）；失败回滚到点击前基线 + 中文提示
        _uiState.update { state ->
            state.copy(
                tasks = state.tasks.map { if (it.id == taskId) it.copy(done = flipped) else it },
            )
        }
        viewModelScope.launch {
            val sender = commands
            if (sender == null) {
                rollbackToggle(taskId, original)
                onError("桌面引擎不可达")
                return@launch
            }
            try {
                sender.execute(
                    "task.toggle",
                    CommandEnvelope.buildParams(
                        listOf("taskId" to taskId, "isCompleted" to flipped),
                    ),
                )
                // 成功：STATE_DELTA 收敛终态（任务勾选事件经写信号推送快照）
            } catch (e: CommandException) {
                rollbackToggle(taskId, original)
                onError("任务操作失败：${e.message}")
            } finally {
                inFlightToggles.remove(taskId)
            }
        }
    }

    /** 回滚到点击前基线（非盲翻——连续点击交错时盲翻会滚到从未存在的状态）。 */
    private fun rollbackToggle(taskId: String, restoreTo: Boolean) {
        _uiState.update { state ->
            state.copy(
                tasks = state.tasks.map {
                    if (it.id == taskId) it.copy(done = restoreTo) else it
                },
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
     * 乐观追加 + `COMMAND(task.create)`；ack 返回桌面 Task（替换临时 id）。
     * 智能判断（未指定象限）保持 classifyingIds 过渡态，归类经 STATE_DELTA 收敛
     * （task:classified 写信号触发快照）；分类上限 12s 后超时清标记兜底。
     */
    fun createTask(input: CreateTaskInput) {
        val tempId = "$LOCAL_ID_PREFIX${nextLocalId++}"
        // 归属展示名（桌面 CrossRoleTask.roleName 由后端联表给出；乐观期就地查 roles）
        val roleName = if (input.ownerType == TaskOwner.BUTLER) "管家"
        else store.roles.firstOrNull { it.id == input.roleId }?.name ?: "未知角色"
        val optimistic = TaskItem(
            id = tempId,
            title = input.title.trim(),
            // 智能判断占位象限：Q2（对齐桌面默认落库值 normalized_quadrant
            // unwrap_or("Q2")——评审 D1-A：占位 Q3 在首个 STATE_DELTA 即被误判
            // 「已归类」清掉分类中标记；占位 Q2 时 ack/快照象限一致，无假跳变）
            quadrant = input.quadrant ?: Quadrant.Q2,
            roleName = roleName,
            ownerType = input.ownerType,
            roleId = input.roleId,
            due = input.due?.trim()?.takeIf { it.isNotEmpty() },
            bigRock = input.bigRock,
        )
        _uiState.update { it.applyCreated(optimistic, pendingClassify = input.quadrant == null) }

        viewModelScope.launch {
            val sender = commands ?: run {
                // fake 态：移除乐观卡片 + 显式失败（不伪造桌面任务）
                _uiState.update { state -> state.copy(tasks = state.tasks.filterNot { it.id == tempId }) }
                onError("桌面引擎不可达")
                return@launch
            }
            try {
                val result = sender.execute(
                    "task.create",
                    CommandEnvelope.buildParams(
                        listOf(
                            "title" to input.title.trim(),
                            "ownerType" to if (input.ownerType == TaskOwner.BUTLER) "butler" else "role",
                            "roleId" to input.roleId,
                            "deadline" to input.due?.trim()?.takeIf { it.isNotEmpty() },
                            "quadrant" to input.quadrant?.code,
                            "isBigRock" to input.bigRock.takeIf { it },
                        ),
                    ),
                )
                replaceWithDesktopTask(tempId, result, pendingClassify = input.quadrant == null)
            } catch (e: CommandException) {
                _uiState.update { state -> state.copy(tasks = state.tasks.filterNot { it.id == tempId }) }
                onError("任务创建失败：${e.message}")
            }
        }
    }

    /** ack Task 替换乐观临时卡片；未指定象限时保分类中态，归类由 15s 超时兜底收口。 */
    private fun replaceWithDesktopTask(tempId: String, result: JSONObject, pendingClassify: Boolean) {
        val desktopId = result.optString("id")
        if (desktopId.isEmpty()) {
            // ack 缺 id：契约破坏——乐观卡移除 + 显式失败，不留永不收敛的孤儿（评审 C15）
            _uiState.update { state -> state.copy(tasks = state.tasks.filterNot { it.id == tempId }) }
            onError("桌面未返回任务 id")
            return
        }
        // 未知象限不造假映射（与 MemoryViewModel 同立场）：保留乐观占位 Q2，
        // 由后续 STATE_DELTA 收敛——静默映射 Q3 会把契约破坏伪装成正常数据
        val quadrant = Quadrant.entries.firstOrNull { it.code == result.optString("quadrant") }
        _uiState.update { state ->
            // 快照已含同 id 任务（写信号快照先于 ack 到达的竞速）：仅去重乐观卡，
            // 否则替换后列表出现两张同 id 卡片（LazyColumn key 冲突，评审 C15）
            if (state.tasks.any { it.id == desktopId }) {
                state.copy(
                    tasks = state.tasks.filterNot { it.id == tempId },
                    classifyingIds = state.classifyingIds - tempId +
                        if (pendingClassify) setOf(desktopId) else emptySet(),
                )
            } else {
                val replaced = state.tasks.map { t ->
                    if (t.id == tempId) {
                        t.copy(
                            id = desktopId,
                            quadrant = quadrant ?: t.quadrant,
                            done = result.optBoolean("isCompleted", false),
                        )
                    } else t
                }
                state.copy(
                    tasks = replaced,
                    // 已指定象限：无需分类；智能判断：保持分类中（象限可能仍在后台变化）
                    classifyingIds = state.classifyingIds - tempId +
                        if (pendingClassify) setOf(desktopId) else emptySet(),
                )
            }
        }
        if (pendingClassify) {
            viewModelScope.launch {
                // 桌面 task_classifier 固定 12s 截止——到期分类已终态（含降级 Q2），清标记兜底
                delay(CLASSIFY_TIMEOUT_MILLIS)
                _uiState.update { it.copy(classifyingIds = it.classifyingIds - desktopId) }
            }
        }
    }

    /** 归属选项键全集：管家 + 全部角色 id（镜像桌面 ownerOptions 的 key 集）。 */
    private fun ownerKeys(): Set<String> =
        setOf(TASK_OWNER_BUTLER_KEY) + store.roles.map { it.id }

    private companion object {
        /** 本地乐观任务 id 前缀（与桌面 id 空间隔离）。 */
        const val LOCAL_ID_PREFIX = "local-"

        /** 桌面智能分类固定截止（task_classifier 12s deadline）+ 快照 debounce 余量。 */
        const val CLASSIFY_TIMEOUT_MILLIS = 15_000L
    }
}
