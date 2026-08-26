package com.egosync.companion.ui.icons

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定 getRoleIcon / normalizeIconId 兜底契约（I/O 矩阵）与 24 id 白名单对齐桌面。
 */
class RoleIconsTest {

    @Test
    fun knownId_returnsMatchingVector() {
        assertSame(LucideIcons.Home, RoleIcons.getRoleIcon("home"))
        assertSame(LucideIcons.Briefcase, RoleIcons.getRoleIcon("briefcase"))
        assertSame(LucideIcons.Target, RoleIcons.getRoleIcon("target"))
    }

    @Test
    fun nullId_fallsBackToTarget() {
        assertSame(LucideIcons.Target, RoleIcons.getRoleIcon(null))
    }

    @Test
    fun unknownId_fallsBackToTarget() {
        assertSame(LucideIcons.Target, RoleIcons.getRoleIcon("not-a-real-id"))
    }

    @Test
    fun blankId_fallsBackToTarget() {
        assertSame(LucideIcons.Target, RoleIcons.getRoleIcon(""))
        assertSame(LucideIcons.Target, RoleIcons.getRoleIcon("   "))
    }

    @Test
    fun normalizeId_knownPassesThrough() {
        assertEquals("home", RoleIcons.normalizeIconId("home"))
        assertEquals("target", RoleIcons.normalizeIconId("target"))
    }

    @Test
    fun normalizeId_invalidFallsBackToTarget() {
        assertEquals("target", RoleIcons.normalizeIconId(null))
        assertEquals("target", RoleIcons.normalizeIconId(""))
        assertEquals("target", RoleIcons.normalizeIconId("xx"))
    }

    // ── 组 3 FR-5：normalizeColorHex 契约（提案回显走白名单回退）────────

    @Test
    fun normalizeColorHex_knownPassesThrough() {
        assertEquals("#4F46E5", RoleIcons.normalizeColorHex("#4F46E5"))
        assertEquals("#0EA5E9", RoleIcons.normalizeColorHex("#0EA5E9"))
        assertEquals("#64748B", RoleIcons.normalizeColorHex("#64748B"))
    }

    @Test
    fun normalizeColorHex_invalidFallsBackToDefault() {
        assertEquals("#4F46E5", RoleIcons.normalizeColorHex(null))
        assertEquals("#4F46E5", RoleIcons.normalizeColorHex(""))
        assertEquals("#4F46E5", RoleIcons.normalizeColorHex("#123456"))
        assertEquals("#4F46E5", RoleIcons.normalizeColorHex("red"))
    }

    @Test
    fun normalizeColorHex_trimsAndUppercases() {
        // 桌面契约：trim + 大写后对白名单（后端可能回传小写/带空格的 hex）
        assertEquals("#0EA5E9", RoleIcons.normalizeColorHex("#0ea5e9"))
        assertEquals("#EC4899", RoleIcons.normalizeColorHex("  #EC4899  "))
    }

    @Test
    fun roleIcons_whitelistHas24AndMatchesDesktop() {
        // 与 egosync-app/src/lib/roleIcons.ts 的 24 id 逐字一致
        val expected = setOf(
            "briefcase", "code", "chart-bar", "palette", "pen-tool", "book-open",
            "graduation-cap", "dumbbell", "heart-pulse", "leaf", "home", "users",
            "baby", "gamepad-2", "music", "camera", "plane", "utensils", "coffee",
            "target", "sparkles", "lightbulb", "compass", "wallet",
        )
        assertEquals(24, RoleIcons.ROLE_ICONS.size)
        assertEquals(expected, RoleIcons.ROLE_ICONS.map { it.id }.toSet())
        assertEquals("target", RoleIcons.DEFAULT_ICON_ID)
    }

    @Test
    fun roleColors_whitelistHas8() {
        assertEquals(8, RoleIcons.ROLE_COLORS.size)
        assertTrue(RoleIcons.ROLE_COLORS.any { it.hex == "#4F46E5" })
        assertEquals("#4F46E5", RoleIcons.DEFAULT_COLOR_HEX)
    }
}
