package com.egosync.companion.ui.tasks

import com.egosync.companion.sync.TaskItem
import com.egosync.companion.ui.theme.Quadrant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定 FR-23 筛选/分类中契约（桌面基线 useTasks.ts + TaskOverviewTab.tsx）：
 * 象限筛选只缩小可见视野不改数据；task:classified 事件必须「替换卡片 + 清标记」
 * 二者同时发生——只清标记不替换，任务会永远停在占位象限；只替换不清，徽章永不消失。
 */
class TasksUiStateTest {

    private fun task(id: String, quadrant: Quadrant) = TaskItem(
        id = id,
        title = "任务$id",
        quadrant = quadrant,
        roleName = "产品经理",
        due = null,
        bigRock = false,
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
}
