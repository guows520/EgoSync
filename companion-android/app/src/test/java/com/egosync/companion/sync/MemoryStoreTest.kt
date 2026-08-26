package com.egosync.companion.sync

import com.egosync.companion.ui.icons.RoleIcons
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定组 3 记忆/提案 mock 契约：per-role 隔离、task_status 不可见、
 * 来源消息标注与提案白名单合法性（弹窗回显走 normalize 的前提是种子本身合法）。
 */
class MemoryStoreTest {

    @Test
    fun memoriesOf_returnsOnlyThatRole() {
        // 记忆屏按单角色进入：跨角色泄漏会把别人的记忆显示到当前角色名下
        val pm = SnapshotStore.memoriesOf("role-pm")
        assertTrue(pm.isNotEmpty())
        assertTrue(pm.all { it.roleId == "role-pm" })
        assertTrue(SnapshotStore.memoriesOf("role-unknown").isEmpty())
    }

    @Test
    fun everyRoleHasVisibleMemories() {
        // 三角色都可从仪表盘进入记忆屏：任一角色空屏会让入口看起来坏了
        listOf("role-pm", "role-father", "role-learner").forEach { roleId ->
            val visible = SnapshotStore.memoriesOf(roleId)
                .filter { it.category != MemoryCategory.TASK_STATUS }
            assertTrue("role $roleId 应有可见记忆", visible.isNotEmpty())
        }
    }

    @Test
    fun taskStatusMemories_existToLockExclusionContract() {
        // 桌面 visibleMemories 契约：任务状态记忆不进记忆面板（属任务域）
        // mock 需保留一条 task_status 种子，否则该过滤契约失去防回归锚点
        assertTrue(SnapshotStore.memories.any { it.category == MemoryCategory.TASK_STATUS })
    }

    @Test
    fun memorySources_flagExactlyOneSourceMessage() {
        // 来源高亮契约（桌面 isSource 着色）：每段来源对话恰好一条被标为记忆出处
        SnapshotStore.memorySources.forEach { (memoryId, messages) ->
            assertTrue("$memoryId 来源非空", messages.isNotEmpty())
            assertEquals("$memoryId 应恰好一条 isSource", 1, messages.count { it.isSource })
        }
    }

    @Test
    fun memorySources_rolesAreUserOrAssistant() {
        // 角色标签映射（用户/助手）不落 raw 值：mock 必须只用两种合法发言角色
        SnapshotStore.memorySources.values.flatten().forEach { message ->
            assertTrue(
                "发言角色应为 user/assistant：${message.role}",
                message.role == "user" || message.role == "assistant",
            )
        }
    }

    @Test
    fun roleProposal_seedIsWhitelisted() {
        // 提案回显走 normalize 白名单回退：种子必须合法，否则弹窗回显默认值而非提案值
        val proposal = SnapshotStore.roleProposal
        assertEquals(proposal.icon, RoleIcons.normalizeIconId(proposal.icon))
        assertEquals(proposal.color, RoleIcons.normalizeColorHex(proposal.color))
        assertTrue(proposal.name.isNotBlank())
    }

    @Test
    fun formatMemoryTime_isoToDesktopFormat() {
        // 桌面时间格式契约：ISO 8601（UTC）→ yyyy/MM/dd HH:mm；非法输入原样返回
        assertEquals("2026/08/24 09:12", formatMemoryTime("2026-08-24T09:12:00Z"))
        assertEquals("not-a-date", formatMemoryTime("not-a-date"))
    }
}
