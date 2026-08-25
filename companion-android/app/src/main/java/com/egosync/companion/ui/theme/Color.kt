package com.egosync.companion.ui.theme

import androidx.compose.ui.graphics.Color

// ═══════════════════════════════════════════════════════════════════════
// 色彩 Token — 母本：egosync-app/src/index.css（CSS 变量）+ 桌面组件实际用色
// 映射关系详见 README「与桌面端的设计语言对照表」
// ═══════════════════════════════════════════════════════════════════════

// ── 深色主题（默认）— --bg-base / --bg-surface / --bg-elevated / --text-* / --border-default ──
val DarkBackground = Color(0xFF0F1117)      // --bg-base 深墨
val DarkSurface = Color(0xFF1A1B2E)         // --bg-surface
val DarkSurfaceElevated = Color(0xFF252638) // --bg-elevated
val DarkTextPrimary = Color(0xFFE8E8ED)     // --text-primary
val DarkTextSecondary = Color(0xFF9CA3AF)   // --text-secondary
val DarkTextMuted = Color(0xFF6B7280)       // --text-muted（深色模式更暗一档）
val DarkOutline = Color(0xFF374151)         // --border-default

// ── 浅色主题 ───────────────────────────────────────────────────────────
val LightBackground = Color(0xFFF8F9FA)     // --bg-base
val LightSurface = Color(0xFFFFFFFF)        // --bg-surface
val LightSurfaceElevated = Color(0xFFFAFAFA) // --bg-elevated
val LightTextPrimary = Color(0xFF1A1A2E)    // --text-primary
val LightTextSecondary = Color(0xFF6B7280)  // --text-secondary
val LightTextMuted = Color(0xFF9CA3AF)      // --text-muted
val LightOutline = Color(0xFFE5E7EB)        // --border-default

// ── 品牌与功能色（双主题共用）──────────────────────────────────────────
// 管家中性 = 桌面 BUTLER_ACCENT #6366F1；工作冷色 = 桌面色板靛蓝 #4F46E5
val BrandIndigo = Color(0xFF6366F1)
val BrandIndigoDeep = Color(0xFF4F46E5)
val BrandIndigoLight = Color(0xFFA5B4FC)
// 家庭暖色 = 桌面色板琥珀 #F59E0B（8 色板，非 UX 文档的 #D97706）
val BrandAmber = Color(0xFFF59E0B)
// 学习 = 桌面色板紫罗兰 #8B5CF6
val BrandViolet = Color(0xFF8B5CF6)
// 健康 = 桌面色板翠绿 #10B981
val BrandEmerald = Color(0xFF10B981)

// 功能色 — index.css --color-success / info / warning / error
val BrandGreen = Color(0xFF10B981)          // --color-success（直连/成功）
val BrandBlue = Color(0xFF3B82F6)           // --color-info（中继/信息/tap 级）
val BrandWarn = Color(0xFFF59E0B)           // --color-warning（Q3/中能量）
val BrandError = Color(0xFFEF4444)          // --color-error（离线/Q1/knock 级）
val QuadrantGray = Color(0xFF9CA3AF)        // Q4

// ── 能量色谱（桌面组件实际分界：≥70 翠绿 / 40~69 琥珀 / <40 红）───────
// 依据：DashboardTab.tsx emerald-500/amber-500/red-500 与 RoleSidebarIcon
// #10B981/#F59E0B/#EF4444 两处一致；CSS --energy-low 灰 token 从未被组件引用
val EnergyHigh = BrandGreen
val EnergyMid = BrandWarn
val EnergyLow = BrandError

fun energyColor(energy: Int): Color = when {
    energy >= 70 -> EnergyHigh
    energy >= 40 -> EnergyMid
    else -> EnergyLow
}

/**
 * 角色域 → 强调色映射位（"色温"系统）。
 * 桌面母本：每角色从 8 色板自选 color（roleIcons.ts），进入角色时
 * --role-accent 切换 + 300ms 过渡 + 6% alpha 背景 tint。
 * 移动端按域预映射：工作=冷色（靛蓝）/ 家庭=暖色（琥珀）。
 * 接真实数据后改为角色自带 color 字段直接取色。
 */
enum class RoleDomain(val label: String) {
    BUTLER("管家"),
    WORK("工作"),
    FAMILY("家庭"),
    LEARN("学习"),
    HEALTH("健康"),
}

data class RoleAccent(val accent: Color, val tint: Color)

fun RoleDomain.accent(): RoleAccent = when (this) {
    RoleDomain.BUTLER -> RoleAccent(BrandIndigo, BrandIndigo.copy(alpha = 0.06f))
    RoleDomain.WORK -> RoleAccent(BrandIndigoDeep, BrandIndigoDeep.copy(alpha = 0.06f)) // 冷色
    RoleDomain.FAMILY -> RoleAccent(BrandAmber, BrandAmber.copy(alpha = 0.06f))         // 暖色
    RoleDomain.LEARN -> RoleAccent(BrandViolet, BrandViolet.copy(alpha = 0.06f))
    RoleDomain.HEALTH -> RoleAccent(BrandEmerald, BrandEmerald.copy(alpha = 0.06f))
}

// ── 四象限（移动端信息架构要求分色；桌面任务卡不按象限着色）────────────
enum class Quadrant(val code: String, val title: String, val subtitle: String) {
    Q1("Q1", "重要且紧急", "危机 · 临期"),
    Q2("Q2", "重要不紧急", "规划 · 成长 · 受保护区"),
    Q3("Q3", "紧急不重要", "打扰 · 杂务"),
    Q4("Q4", "不重要不紧急", "消遣 · 琐事"),
}

fun Quadrant.color(): Color = when (this) {
    Quadrant.Q1 -> BrandError
    Quadrant.Q2 -> BrandBlue
    Quadrant.Q3 -> BrandWarn
    Quadrant.Q4 -> QuadrantGray
}
