package com.egosync.companion.sync

import com.egosync.companion.ui.icons.RoleIcons
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 13.2 记忆契约锁定（AC5）：记忆内容**永不进入快照**——
 * memoriesOf 恒为空（无论快照是否加载），记忆页呈现「待接指令通道」占位，
 * 查询/溯源/遗忘走指令通道（13.3）。若未来有人把记忆内容塞进快照派生，
 * 本测试立即报错，防止敏感内容静默越过信任边界。
 */
class MemoryStoreTest {

    private fun storeWithSnapshot(): SnapshotStore {
        val store = SnapshotStore()
        // 最小合法快照（角色/会话俱全）——即便快照富数据，记忆仍必须为空
        store.applySnapshot(
            DesktopSnapshot(
                schemaVersion = 1,
                generatedAt = "2026-08-25T08:00:00Z",
                dataCutoffAt = null,
                truncated = false,
                truncatedDomains = emptyList(),
                roles = listOf(
                    SnapshotRole("role-pm", "产品经理", "target", "#4F46E5", "", "", 82, "moderate"),
                ),
                tasks = emptyList(),
                dashboard = SnapshotDashboard(
                    statuses = emptyList(),
                    metrics = SnapshotMetrics(0, 0, 0, 0, "2026-08-25T08:00:00Z"),
                ),
                conversations = emptyList(),
                briefings = emptyList(),
                weeklyReviews = emptyList(),
                notifications = emptyList(),
            )
        )
        return store
    }

    @Test
    fun memoriesOf_alwaysEmpty_evenWithRichSnapshot() {
        // AC5：快照含角色/任务/会话等富数据，但记忆内容不得随之下发
        val store = storeWithSnapshot()
        assertTrue(store.memoriesOf("role-pm").isEmpty())
        assertTrue(store.state.value.loaded) // 前置：快照确已加载，排除「空因为没数据」
    }

    @Test
    fun memoriesOf_emptyOnColdStart() {
        // 冷启动（无快照无缓存）同样为空——不得回退到任何 mock 种子
        val store = SnapshotStore()
        assertTrue(store.memoriesOf("role-pm").isEmpty())
        assertTrue(store.memoriesOf("role-unknown").isEmpty())
    }

    @Test
    fun roleProposal_seedIsWhitelisted() {
        // 提案回显走 normalize 白名单回退：演示种子必须合法，否则弹窗回显默认值而非提案值
        val proposal = com.egosync.companion.ui.chat.roleProposal
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
