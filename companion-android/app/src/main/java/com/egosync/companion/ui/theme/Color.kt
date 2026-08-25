package com.egosync.companion.ui.theme

import androidx.compose.ui.graphics.Color

// ── 深色主题（默认）— 源自桌面端 UX 规范色彩 token ──────────────────────
val DarkBackground = Color(0xFF0F1117)      // 深墨
val DarkSurface = Color(0xFF1A1B2E)         // 表面
val DarkSurfaceElevated = Color(0xFF252638) // 浮起表面
val DarkTextPrimary = Color(0xFFE8E8ED)
val DarkTextSecondary = Color(0xFF9CA3AF)
val DarkOutline = Color(0xFF3A3B4E)

// ── 浅色主题 ───────────────────────────────────────────────────────────
val LightBackground = Color(0xFFF8F9FA)
val LightSurface = Color(0xFFFFFFFF)
val LightSurfaceElevated = Color(0xFFFAFAFA)
val LightTextPrimary = Color(0xFF1A1A2E)
val LightTextSecondary = Color(0xFF6B7280)
val LightOutline = Color(0xFFE5E7EB)

// ── 品牌与功能色（双主题共用）──────────────────────────────────────────
val BrandIndigo = Color(0xFF6366F1)         // 管家中性
val BrandIndigoLight = Color(0xFFA5B4FC)
val BrandAmber = Color(0xFFD97706)          // 家庭暖色
val BrandAmberLight = Color(0xFFF5B95C)
val BrandGreen = Color(0xFF10B981)          // 成功 / 直连
val BrandBlue = Color(0xFF3B82F6)           // 信息 / 中继
val BrandWarn = Color(0xFFF59E0B)           // 警告 / Q3 / 中能量
val BrandError = Color(0xFFEF4444)          // 错误 / 离线 / Q1
val BrandViolet = Color(0xFF7C3AED)         // 学习
val BrandEmerald = Color(0xFF059669)        // 健康
val QuadrantGray = Color(0xFF9CA3AF)        // Q4 / 低能量（暗淡灰，避免负罪感）

// ── 能量色谱（≥70 翠绿 / 40~69 琥珀 / <40 暗淡灰）───────────────────────
val EnergyHigh = BrandGreen
val EnergyMid = BrandWarn
val EnergyLow = QuadrantGray

fun energyColor(energy: Int): Color = when {
    energy >= 70 -> EnergyHigh
    energy >= 40 -> EnergyMid
    else -> EnergyLow
}

/**
 * 角色域 → 强调色映射位。
 * 预留"工作=冷色 accent / 家庭=暖色 accent"的色彩角色映射：
 * 后续接入真实角色数据时按 domain 取色，实现角色切换的微妙色温变化。
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
    RoleDomain.BUTLER -> RoleAccent(BrandIndigo, BrandIndigo.copy(alpha = 0.12f))
    RoleDomain.WORK -> RoleAccent(BrandIndigoLight, BrandIndigo.copy(alpha = 0.10f))   // 冷色
    RoleDomain.FAMILY -> RoleAccent(BrandAmberLight, BrandAmber.copy(alpha = 0.12f))  // 暖色
    RoleDomain.LEARN -> RoleAccent(BrandViolet, BrandViolet.copy(alpha = 0.12f))
    RoleDomain.HEALTH -> RoleAccent(BrandEmerald, BrandEmerald.copy(alpha = 0.12f))
}

// ── 四象限颜色（Q1 红 / Q2 蓝·受保护 / Q3 琥珀 / Q4 灰）──────────────────
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
