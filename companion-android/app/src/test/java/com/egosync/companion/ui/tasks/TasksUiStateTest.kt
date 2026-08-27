package com.egosync.companion.ui.tasks

import com.egosync.companion.sync.TASK_OWNER_BUTLER_KEY
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.sync.TaskOwner
import com.egosync.companion.ui.theme.Quadrant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定 FR-23 筛选/分类中/新建契约（桌面基线 useTasks.ts + TaskOverviewTab.tsx + TaskModal.tsx）：
 * 象限/归属/大石头三筛选独立叠加不互相旁路；task:classified 事件「替换卡片 + 清标记」原子发生；
 * 新建智能判断走 classifying 过渡态、显式象限直接归组。toggleOwner/toggleAllOwners 逐字镜像桌面语义。
 */
class TasksUiStateTest {

    private fun task(
        id: String,
        quadrant: Quadrant,
        ownerType: TaskOwner = TaskOwner.ROLE,
        roleId: String? = "role-pm",
        bigRock: Boolean = false,
    ) = TaskItem(
        id = id,
        title = "任务$id",
        quadrant = quadrant,
        roleName = "产品经理",
        ownerType = ownerType,
        roleId = roleId,
        due = null,
        bigRock = bigRock,
    )

    @Test
    fun grouped_unfiltered_keepsEveryQuadrantVisible() {
        // 「全部」是默认态：筛选机制只能缩小视野，不许悄悄重排或丢任务
        val state = TasksUiState(
            tasks = listOf(task("a", Quadrant.Q1), task("b", Quadrant.Q4)),
            quadrantFilter = null,
        )
        val grouped = state.grouped()
        assertEquals(listOf(task("a", Quadrant.Q1)), grouped[Quadrant.Q1])
        assertEquals(listOf(task("b", Quadrant.Q4)), grouped[Quadrant.Q4])
    }

    @Test
    fun grouped_filtered_showsOnlySelectedQuadrant() {
        // 选 Q4 就不该看到别的象限——这是筛选交互存在的全部意义
        val state = TasksUiState(
            tasks = listOf(task("a", Quadrant.Q1), task("b", Quadrant.Q4)),
            quadrantFilter = Quadrant.Q4,
        )
        val grouped = state.grouped()
        // associateWith 保证四象限键齐全，getValue 非空
        assertTrue(grouped.getValue(Quadrant.Q1).isEmpty())
        assertEquals(listOf(task("b", Quadrant.Q4)), grouped.getValue(Quadrant.Q4))
    }

    @Test
    fun grouped_filteredEmptyQuadrant_yieldsAllEmptyGroups() {
        // 筛到无任务象限 → 全分组为空，是 UI 空态文案分支的前提
        val state = TasksUiState(
            tasks = listOf(task("a", Quadrant.Q4)),
            quadrantFilter = Quadrant.Q1,
        )
        assertTrue(state.grouped().values.all { it.isEmpty() })
    }

    @Test
    fun applyClassified_replacesTaskAndClearsItsMark() {
        // 桌面事件语义：归类后任务出现在正确象限且徽章消失（替换与清标记必须原子）
        val pending = task("a", Quadrant.Q3)
        val state = TasksUiState(
            tasks = listOf(pending, task("b", Quadrant.Q1)),
            classifyingIds = setOf("a", "b"),
        )
        val classified = state.applyClassified(pending.copy(quadrant = Quadrant.Q2))

        assertEquals(Quadrant.Q2, classified.tasks.first { it.id == "a" }.quadrant)
        assertEquals(setOf("b"), classified.classifyingIds)
    }

    @Test
    fun applyClassified_keepsUnmarkedTasksUntouched() {
        // 事件按 id 定向替换：未涉及的任务（含同列表其它任务）原样保留
        val state = TasksUiState(
            tasks = listOf(task("a", Quadrant.Q1), task("b", Quadrant.Q2)),
        )
        val next = state.applyClassified(task("c", Quadrant.Q4))

        assertEquals(state.tasks, next.tasks)
        assertTrue(next.classifyingIds.isEmpty())
    }

    @Test
    fun eventPayload_builtFromCurrentCard_preservesLocalCompletion() {
        // 分类窗口内勾选不得被事件回滚：VM 的 mock 事件载荷必须从当前卡片构造
        // （只改象限），否则用户在 4 秒窗口内的勾选会被陈旧常量静默还原
        val pending = task("a", Quadrant.Q3)
        val state = TasksUiState(
            tasks = listOf(pending),
            classifyingIds = setOf("a"),
        )
        val toggled = TasksUiState(
            tasks = listOf(pending.copy(done = true)),
            classifyingIds = setOf("a"),
        )
        val current = toggled.tasks.first { it.id == "a" }
        val next = toggled.applyClassified(current.copy(quadrant = Quadrant.Q2))

        assertEquals(Quadrant.Q2, next.tasks.first { it.id == "a" }.quadrant)
        assertTrue(next.tasks.first { it.id == "a" }.done)
        assertTrue(next.classifyingIds.isEmpty())
    }

    // ── 归属筛选（多选排除语义，镜像桌面 deselectedOwners）────────────────

    @Test
    fun grouped_deselectedOwner_excludesOnlyThatOwnersTasks() {
        // 归属筛选是「排除」而非「单选」：排除管家后其余角色的任务必须原样保留，
        // 否则用户会以为被排除者之外的角色也丢了任务
        val state = TasksUiState(
            tasks = listOf(
                task("butler", Quadrant.Q1, ownerType = TaskOwner.BUTLER, roleId = null),
                task("pm", Quadrant.Q1),
                task("father", Quadrant.Q1, roleId = "role-father"),
            ),
            deselectedOwners = setOf(TASK_OWNER_BUTLER_KEY),
        )
        val visibleIds = state.grouped().values.flatten().map { it.id }
        assertEquals(listOf("pm", "father"), visibleIds)
    }

    @Test
    fun grouped_allOwnersDeselected_showsEmptyGroups() {
        // 全排除是 toggleAllOwners 的合法落点（桌面语义），空态文案依赖全空分组判定
        val state = TasksUiState(
            tasks = listOf(task("a", Quadrant.Q1)),
            deselectedOwners = setOf(TASK_OWNER_BUTLER_KEY, "role-pm"),
        )
        assertTrue(state.grouped().values.all { it.isEmpty() })
    }

    @Test
    fun toggleOwner_fromDefault_soloSelectsThatOwner_thenDeselectsIt() {
        // 桌面交互契约：默认全选态点某归属 = 一次点击「只看它」（省去逐个取消 N-1 次）；
        // 再点一次该归属 = 全部排除（空态）。若做成简单取反，两条路径都会错
        val allKeys = setOf(TASK_OWNER_BUTLER_KEY, "role-pm", "role-father")
        val state = TasksUiState(deselectedOwners = emptySet())

        val solo = state.toggleOwner("role-pm", allKeys)
        assertEquals(allKeys - "role-pm", solo.deselectedOwners)

        val none = solo.toggleOwner("role-pm", allKeys)
        assertEquals(allKeys, none.deselectedOwners)

        // 独选态点另一归属 = 扩展选择（该归属移出排除集）：从「只看 PM」变「PM + 管家」——
        // 这是最常用的中间路径（else 分支：deselectedOwners + ownerKey），错则用户每点多一个角色都全排除
        val expanded = solo.toggleOwner(TASK_OWNER_BUTLER_KEY, allKeys)
        assertEquals(setOf("role-father"), expanded.deselectedOwners)
    }

    @Test
    fun toggleAllOwners_fromDefault_deselectsAll_fromPartial_reselectsAll() {
        // 「全部」按钮双态方向必须与桌面一致：默认态点击 = 全排除；任意排除态点击 = 恢复全选。
        // 方向做反的话，移动端同名按钮与桌面行为恰好相反
        val allKeys = setOf(TASK_OWNER_BUTLER_KEY, "role-pm")
        assertEquals(allKeys, TasksUiState().toggleAllOwners(allKeys).deselectedOwners)
        assertEquals(
            emptySet<String>(),
            TasksUiState(deselectedOwners = setOf("role-pm")).toggleAllOwners(allKeys).deselectedOwners,
        )
    }

    @Test
    fun grouped_bigRockQuadrantAndOwnerFilters_stack() {
        // 三筛选维度必须独立叠加（象限 + 大石头 + 归属），任何一维不得旁路另一维——
        // 这是「只看大石头 + Q2 + 排除管家」组合筛选存在的全部意义
        val state = TasksUiState(
            tasks = listOf(
                task("keep", Quadrant.Q2, bigRock = true),
                task("x-owner", Quadrant.Q2, ownerType = TaskOwner.BUTLER, roleId = null, bigRock = true),
                task("x-bigrock", Quadrant.Q2, bigRock = false),
                task("x-quadrant", Quadrant.Q3, bigRock = true),
            ),
            quadrantFilter = Quadrant.Q2,
            deselectedOwners = setOf(TASK_OWNER_BUTLER_KEY),
            showBigRocksOnly = true,
        )
        val grouped = state.grouped()
        assertEquals(listOf("keep"), grouped.getValue(Quadrant.Q2).map { it.id })
        assertTrue(grouped.getValue(Quadrant.Q3).isEmpty())
    }

    // ── 新建任务（智能判断 → classifying 过渡态，镜像 useTasks.createTask）──

    @Test
    fun applyCreated_autoQuadrant_marksClassifyingImmediately() {
        // 智能判断不能让用户等分类结果：任务必须立即上屏并挂「分类中」徽章，
        // 归类由 task:classified 事件异步完成（桌面 useTasks.ts:131-138 同款契约）
        val created = task("new", Quadrant.Q3)
        val state = TasksUiState(tasks = emptyList()).applyCreated(created, pendingClassify = true)

        assertTrue(state.tasks.contains(created))
        assertTrue(state.classifyingIds.contains("new"))
    }

    @Test
    fun applyCreated_explicitQuadrant_landsInGroupWithoutClassifying() {
        // 显式象限 = 手动归组（桌面 manualOverride 语义），不得进异步分类——
        // 否则用户刚选好的象限会被 4 秒后的 mock 分类事件悄悄改掉
        val created = task("new", Quadrant.Q1)
        val state = TasksUiState(tasks = emptyList()).applyCreated(created, pendingClassify = false)

        assertEquals(listOf(created), state.grouped().getValue(Quadrant.Q1))
        assertTrue(state.classifyingIds.isEmpty())
    }

    @Test
    fun grouped_classifyingTask_ignoresQuadrantFilter() {
        // 智能判断占位象限（Q3）≠ 最终归类（Q2）：若分类中任务参与象限过滤，
        // 选中 Q2 的用户在 4 秒分类窗口内完全看不到刚建的任务——违背「立即上屏」契约
        val state = TasksUiState(
            tasks = listOf(task("c", Quadrant.Q3)),
            classifyingIds = setOf("c"),
            quadrantFilter = Quadrant.Q2,
        )
        assertEquals(listOf("c"), state.grouped().values.flatten().map { it.id })
    }

    @Test
    fun createTaskTitleError_blank_returnsMessage_nonBlank_returnsNull() {
        // 空标题提交必须被拦下并给出与桌面一致的行内报错（TaskModal.tsx:104 逐字镜像），
        // 纯空格同样是空——这是「提交时校验」分支的唯一守门人
        assertEquals("任务内容不能为空", createTaskTitleError(""))
        assertEquals("任务内容不能为空", createTaskTitleError("   "))
        assertEquals(null, createTaskTitleError("准备 OKR"))
    }
}
