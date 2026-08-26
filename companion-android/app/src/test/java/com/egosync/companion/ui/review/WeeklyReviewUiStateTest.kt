package com.egosync.companion.ui.review

import com.egosync.companion.sync.BigRockPlanItem
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定 FR-17 规划状态机（桌面基线 WeeklyReviewModal.tsx L80-115 + useBigRockPlanning）：
 * 采纳必须优先填空行——只追加不改空行会让空行永远滞留在已填项之间；
 * 确认收集必须过滤空白行——桌面 handleConfirmPlan 只把 trim 后非空的项发给 savePlan，
 * 空白混入会为角色创建出空白大石头任务。
 */
class WeeklyReviewUiStateTest {

    private fun state(vararg items: String, roleId: String = "role-pm") = WeeklyReviewUiState(
        planStates = listOf(RolePlanState(roleId, items.toList())),
    )

    @Test
    fun adoptSuggestion_fillsFirstEmptySlot_withoutAddingRow() {
        // 桌面语义：优先改写首个空行，行数不变（只追加不改空行 → 空行滞留列表头部）
        val next = state("", "").adoptSuggestion("role-pm", "读完第 3 章")
        assertEquals(listOf("读完第 3 章", ""), next.planStates.single().items)
    }

    @Test
    fun adoptSuggestion_appendsWhenNoEmptySlot() {
        // 无空行时追加——保证采纳永不丢内容
        val next = state("已有项").adoptSuggestion("role-pm", "新建议")
        assertEquals(listOf("已有项", "新建议"), next.planStates.single().items)
    }

    @Test
    fun removeItem_removesTargetedRowOnly_andKeepsOtherRoleUntouched() {
        // X 按行定向移除：目标角色删对行，其余角色与文案顺序原样保留
        val s = WeeklyReviewUiState(
            planStates = listOf(
                RolePlanState("role-pm", listOf("a", "b", "c")),
                RolePlanState("role-father", listOf("x")),
            ),
        )
        val next = s.removeItem("role-pm", 1)
        assertEquals(listOf("a", "c"), next.planStates[0].items)
        assertEquals(listOf("x"), next.planStates[1].items)
    }

    @Test
    fun addItem_appendsEmptyRow() {
        // 「添加」永远在行尾追加空行，供下一个大石头填写
        val next = state("a").addItem("role-pm")
        assertEquals(listOf("a", ""), next.planStates.single().items)
    }

    @Test
    fun updateItem_rewritesTargetedRow_only() {
        // 输入行按 (roleId, index) 定向更新：改错行/串角色会让用户输入互相覆盖
        val s = WeeklyReviewUiState(
            planStates = listOf(
                RolePlanState("role-pm", listOf("a", "b")),
                RolePlanState("role-father", listOf("x")),
            ),
        )
        val next = s.updateItem("role-pm", 1, "改后")
        assertEquals(listOf("a", "改后"), next.planStates[0].items)
        assertEquals(listOf("x"), next.planStates[1].items)
    }

    @Test
    fun plannedItems_filtersBlankRows_andTrims() {
        // 桌面 handleConfirmPlan：trim 后非空才构造 BigRockPlanItem——空白行不得变成空白任务
        val s = WeeklyReviewUiState(
            planStates = listOf(
                RolePlanState("role-pm", listOf("  读第 3 章 ", "", "写笔记")),
                RolePlanState("role-father", listOf("   ")),
            ),
        )
        assertEquals(
            listOf(
                BigRockPlanItem("role-pm", "读第 3 章"),
                BigRockPlanItem("role-pm", "写笔记"),
            ),
            s.plannedItems(),
        )
    }

    @Test
    fun plannedItems_allBlank_isEmpty_soConfirmIsNoOp() {
        // 全空白 → plannedItems 为空是 VM confirmPlan「点击无效果」守卫的全部依据
        // （镜像桌面 items.length===0 return）：守卫若失守会保存出空规划
        assertTrue(state("", "  ").plannedItems().isEmpty())
    }
}
