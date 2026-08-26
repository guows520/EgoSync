package com.egosync.companion.ui.icons

import androidx.compose.ui.graphics.vector.ImageVector

/**
 * 镜像桌面 `egosync-app/src/lib/roleIcons.ts`：24 角色 icon id 白名单 + 8 品牌色，
 * 与后端 `agent_engine.rs` SUPPORTED_ICONS / SUPPORTED_COLORS 三方一致。
 * 未知/空 id 回退 Target（镜像桌面 normalizeIconId）。
 */
data class RoleIconOption(val id: String, val label: String)
data class RoleColorOption(val hex: String, val label: String)

object RoleIcons {
    const val DEFAULT_ICON_ID = "target"
    const val DEFAULT_COLOR_HEX = "#4F46E5"

    val ROLE_ICONS: List<RoleIconOption> = listOf(
        RoleIconOption("briefcase", "工作"),
        RoleIconOption("code", "编程"),
        RoleIconOption("chart-bar", "数据"),
        RoleIconOption("palette", "设计"),
        RoleIconOption("pen-tool", "写作"),
        RoleIconOption("book-open", "阅读"),
        RoleIconOption("graduation-cap", "学习"),
        RoleIconOption("dumbbell", "健身"),
        RoleIconOption("heart-pulse", "健康"),
        RoleIconOption("leaf", "自然"),
        RoleIconOption("home", "家庭"),
        RoleIconOption("users", "朋友"),
        RoleIconOption("baby", "育儿"),
        RoleIconOption("gamepad-2", "游戏"),
        RoleIconOption("music", "音乐"),
        RoleIconOption("camera", "摄影"),
        RoleIconOption("plane", "旅行"),
        RoleIconOption("utensils", "美食"),
        RoleIconOption("coffee", "休闲"),
        RoleIconOption("target", "目标"),
        RoleIconOption("sparkles", "灵感"),
        RoleIconOption("lightbulb", "想法"),
        RoleIconOption("compass", "探索"),
        RoleIconOption("wallet", "财务"),
    )

    val ROLE_COLORS: List<RoleColorOption> = listOf(
        RoleColorOption("#4F46E5", "靛蓝"),
        RoleColorOption("#0EA5E9", "天蓝"),
        RoleColorOption("#10B981", "翠绿"),
        RoleColorOption("#F59E0B", "琥珀"),
        RoleColorOption("#EF4444", "玫红"),
        RoleColorOption("#8B5CF6", "紫罗兰"),
        RoleColorOption("#EC4899", "粉"),
        RoleColorOption("#64748B", "石板灰"),
    )

    private val byId: Map<String, ImageVector> = mapOf(
        "briefcase" to LucideIcons.Briefcase,
        "code" to LucideIcons.Code,
        "chart-bar" to LucideIcons.ChartBar,
        "palette" to LucideIcons.Palette,
        "pen-tool" to LucideIcons.PenTool,
        "book-open" to LucideIcons.BookOpen,
        "graduation-cap" to LucideIcons.GraduationCap,
        "dumbbell" to LucideIcons.Dumbbell,
        "heart-pulse" to LucideIcons.HeartPulse,
        "leaf" to LucideIcons.Leaf,
        "home" to LucideIcons.Home,
        "users" to LucideIcons.Users,
        "baby" to LucideIcons.Baby,
        "gamepad-2" to LucideIcons.Gamepad2,
        "music" to LucideIcons.Music,
        "camera" to LucideIcons.Camera,
        "plane" to LucideIcons.Plane,
        "utensils" to LucideIcons.Utensils,
        "coffee" to LucideIcons.Coffee,
        "target" to LucideIcons.Target,
        "sparkles" to LucideIcons.Sparkles,
        "lightbulb" to LucideIcons.Lightbulb,
        "compass" to LucideIcons.Compass,
        "wallet" to LucideIcons.Wallet,
    )

    /** 通过 id 查图标；null/空/未知回退 Target，不抛错。镜像 roleIcons.ts getRoleIconComponent。 */
    fun getRoleIcon(iconId: String?): ImageVector =
        if (iconId.isNullOrBlank()) LucideIcons.Target else byId[iconId] ?: LucideIcons.Target

    /** 校验 id 合法；非法/空返回默认 target。镜像 roleIcons.ts normalizeIconId。 */
    fun normalizeIconId(iconId: String?): String {
        if (iconId.isNullOrBlank()) return DEFAULT_ICON_ID
        return if (byId.containsKey(iconId)) iconId else DEFAULT_ICON_ID
    }

    /** 校验色值合法；非法/空返回默认 hex。镜像 roleIcons.ts normalizeColorHex（trim+大写后对白名单）。 */
    fun normalizeColorHex(color: String?): String {
        if (color.isNullOrBlank()) return DEFAULT_COLOR_HEX
        val upper = color.trim().uppercase()
        return if (ROLE_COLORS.any { it.hex == upper }) upper else DEFAULT_COLOR_HEX
    }
}
