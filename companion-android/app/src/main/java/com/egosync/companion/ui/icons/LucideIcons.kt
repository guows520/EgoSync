package com.egosync.companion.ui.icons
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.unit.dp
/**
 * Lucide 图标集，path data 逐字移植自 lucide 官方 SVG（ISC 许可）。
 * viewport 24×24、strokeWidth=2、stroke-only；与桌面 roleIcons.ts 线框体系一致。
 */
object LucideIcons {
    private fun lucide(name: String, vararg paths: String): ImageVector {
        val builder = ImageVector.Builder(
            name = name,
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        )
        paths.forEach { p ->
            builder.addPath(
                pathData = PathParser().parsePathString(p).toNodes(),
                fill = null,
                stroke = SolidColor(Color.Black),
                strokeLineWidth = 2f,
            )
        }
        return builder.build()
    }
    val Briefcase: ImageVector by lazy { lucide("Briefcase", "M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16", "M4,6h16a2,2 0 0 1 2,2v10a2,2 0 0 1 -2,2h-16a2,2 0 0 1 -2,-2v-10a2,2 0 0 1 2,-2z") }
    val Code: ImageVector by lazy { lucide("Code", "m16 18 6-6-6-6", "m8 6-6 6 6 6") }
    val ChartBar: ImageVector by lazy { lucide("ChartBar", "M3 3v16a2 2 0 0 0 2 2h16", "M7 16h8", "M7 11h12", "M7 6h3") }
    val Palette: ImageVector by lazy { lucide("Palette", "M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z", "M13,6.5A0.5,0.5 0 1 0 14,6.5A0.5,0.5 0 1 0 13,6.5", "M17,10.5A0.5,0.5 0 1 0 18,10.5A0.5,0.5 0 1 0 17,10.5", "M6,12.5A0.5,0.5 0 1 0 7,12.5A0.5,0.5 0 1 0 6,12.5", "M8,7.5A0.5,0.5 0 1 0 9,7.5A0.5,0.5 0 1 0 8,7.5") }
    val PenTool: ImageVector by lazy { lucide("PenTool", "M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z", "m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18", "m2.3 2.3 7.286 7.286", "M9,11A2,2 0 1 0 13,11A2,2 0 1 0 9,11") }
    val BookOpen: ImageVector by lazy { lucide("BookOpen", "M12 5v16", "M20.001 19A2 2 0 0022 17V5a2 2 0 00-1.999-2L16 3.002A5 5 0 0012 5a5 5 0 00-4-2H4a2 2 0 00-2 2v12a2 2 0 001.999 2H8a5 5 0 014 2 5 5 0 014-2z") }
    val GraduationCap: ImageVector by lazy { lucide("GraduationCap", "M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z", "M22 10v6", "M6 12.5V16a6 3 0 0 0 12 0v-3.5") }
    val Dumbbell: ImageVector by lazy { lucide("Dumbbell", "M17.596 12.768a2 2 0 1 0 2.829-2.829l-1.768-1.767a2 2 0 0 0 2.828-2.829l-2.828-2.828a2 2 0 0 0-2.829 2.828l-1.767-1.768a2 2 0 1 0-2.829 2.829z", "m2.5 21.5 1.4-1.4", "m20.1 3.9 1.4-1.4", "M5.343 21.485a2 2 0 1 0 2.829-2.828l1.767 1.768a2 2 0 1 0 2.829-2.829l-6.364-6.364a2 2 0 1 0-2.829 2.829l1.768 1.767a2 2 0 0 0-2.828 2.829z", "m9.6 14.4 4.8-4.8") }
    val HeartPulse: ImageVector by lazy { lucide("HeartPulse", "M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5", "M3.22 13H9.5l.5-1 2 4.5 2-7 1.5 3.5h5.27") }
    val Leaf: ImageVector by lazy { lucide("Leaf", "M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 4.18 2 8 0 5.5-4.78 10-10 10Z", "M2 21c0-3 1.85-5.36 5.08-6C9.5 14.52 12 13 13 12") }
    val Home: ImageVector by lazy { lucide("Home", "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8", "M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z") }
    val Users: ImageVector by lazy { lucide("Users", "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2", "M16 3.128a4 4 0 0 1 0 7.744", "M22 21v-2a4 4 0 0 0-3-3.87", "M5,7A4,4 0 1 0 13,7A4,4 0 1 0 5,7") }
    val Baby: ImageVector by lazy { lucide("Baby", "M10 16c.5.3 1.2.5 2 .5s1.5-.2 2-.5", "M15 12h.01", "M19.38 6.813A9 9 0 0 1 20.8 10.2a2 2 0 0 1 0 3.6 9 9 0 0 1-17.6 0 2 2 0 0 1 0-3.6A9 9 0 0 1 12 3c2 0 3.5 1.1 3.5 2.5s-.9 2.5-2 2.5c-.8 0-1.5-.4-1.5-1", "M9 12h.01") }
    val Gamepad2: ImageVector by lazy { lucide("Gamepad2", "M6,11L10,11", "M8,9L8,13", "M15,12L15.01,12", "M18,10L18.01,10", "M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z") }
    val Music: ImageVector by lazy { lucide("Music", "M9 18V5l12-2v13", "M3,18A3,3 0 1 0 9,18A3,3 0 1 0 3,18", "M15,16A3,3 0 1 0 21,16A3,3 0 1 0 15,16") }
    val Camera: ImageVector by lazy { lucide("Camera", "M13.997 4a2 2 0 0 1 1.76 1.05l.486.9A2 2 0 0 0 18.003 7H20a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h1.997a2 2 0 0 0 1.759-1.048l.489-.904A2 2 0 0 1 10.004 4z", "M9,13A3,3 0 1 0 15,13A3,3 0 1 0 9,13") }
    val Plane: ImageVector by lazy { lucide("Plane", "M17.8 19.2 16 11l3.5-3.5C21 6 21.5 4 21 3c-1-.5-3 0-4.5 1.5L13 8 4.8 6.2c-.5-.1-.9.1-1.1.5l-.3.5c-.2.5-.1 1 .3 1.3L9 12l-2 3H4l-1 1 3 2 2 3 1-1v-3l3-2 3.5 5.3c.3.4.8.5 1.3.3l.5-.2c.4-.3.6-.7.5-1.2z") }
    val Utensils: ImageVector by lazy { lucide("Utensils", "M3 2v7c0 1.1.9 2 2 2h4a2 2 0 0 0 2-2V2", "M7 2v20", "M21 15V2a5 5 0 0 0-5 5v6c0 1.1.9 2 2 2h3Zm0 0v7") }
    val Coffee: ImageVector by lazy { lucide("Coffee", "M10 2v2", "M14 2v2", "M16 8a1 1 0 0 1 1 1v8a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1h14a4 4 0 1 1 0 8h-1", "M6 2v2") }
    val Target: ImageVector by lazy { lucide("Target", "M2,12A10,10 0 1 0 22,12A10,10 0 1 0 2,12", "M6,12A6,6 0 1 0 18,12A6,6 0 1 0 6,12", "M10,12A2,2 0 1 0 14,12A2,2 0 1 0 10,12") }
    val Sparkles: ImageVector by lazy { lucide("Sparkles", "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z", "M20 2v4", "M22 4h-4", "M2,20A2,2 0 1 0 6,20A2,2 0 1 0 2,20") }
    val Lightbulb: ImageVector by lazy { lucide("Lightbulb", "M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5", "M9 18h6", "M10 22h4") }
    val Compass: ImageVector by lazy { lucide("Compass", "M2,12A10,10 0 1 0 22,12A10,10 0 1 0 2,12", "m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z") }
    val Wallet: ImageVector by lazy { lucide("Wallet", "M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1", "M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4") }
    val ConciergeBell: ImageVector by lazy { lucide("ConciergeBell", "M3 20a1 1 0 0 1-1-1v-1a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v1a1 1 0 0 1-1 1Z", "M20 16a8 8 0 1 0-16 0", "M12 4v4", "M10 4h4") }
    val MessageSquare: ImageVector by lazy { lucide("MessageSquare", "M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z") }
    val ListTodo: ImageVector by lazy { lucide("ListTodo", "M13 5h8", "M13 12h8", "M13 19h8", "m3 17 2 2 4-4", "M4,4h4a1,1 0 0 1 1,1v4a1,1 0 0 1 -1,1h-4a1,1 0 0 1 -1,-1v-4a1,1 0 0 1 1,-1z") }
    val LayoutDashboard: ImageVector by lazy { lucide("LayoutDashboard", "M4,3h5a1,1 0 0 1 1,1v7a1,1 0 0 1 -1,1h-5a1,1 0 0 1 -1,-1v-7a1,1 0 0 1 1,-1z", "M15,3h5a1,1 0 0 1 1,1v3a1,1 0 0 1 -1,1h-5a1,1 0 0 1 -1,-1v-3a1,1 0 0 1 1,-1z", "M15,12h5a1,1 0 0 1 1,1v7a1,1 0 0 1 -1,1h-5a1,1 0 0 1 -1,-1v-7a1,1 0 0 1 1,-1z", "M4,16h5a1,1 0 0 1 1,1v3a1,1 0 0 1 -1,1h-5a1,1 0 0 1 -1,-1v-3a1,1 0 0 1 1,-1z") }
    val User: ImageVector by lazy { lucide("User", "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2", "M8,7A4,4 0 1 0 16,7A4,4 0 1 0 8,7") }
    val FileText: ImageVector by lazy { lucide("FileText", "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z", "M14 2v5a1 1 0 0 0 1 1h5", "M10 9H8", "M16 13H8", "M16 17H8") }
    val TrendingUp: ImageVector by lazy { lucide("TrendingUp", "M16 7h6v6", "m22 7-8.5 8.5-5-5L2 17") }
    val Bell: ImageVector by lazy { lucide("Bell", "M10.268 21a2 2 0 0 0 3.464 0", "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326") }
    val WifiOff: ImageVector by lazy { lucide("WifiOff", "M12 20h.01", "M8.5 16.429a5 5 0 0 1 7 0", "M5 12.859a10 10 0 0 1 5.17-2.69", "M19 12.859a10 10 0 0 0-2.007-1.523", "M2 8.82a15 15 0 0 1 4.177-2.643", "M22 8.82a15 15 0 0 0-11.288-3.764", "m2 2 20 20") }
    val Moon: ImageVector by lazy { lucide("Moon", "M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401") }
    val Sun: ImageVector by lazy { lucide("Sun", "M8,12A4,4 0 1 0 16,12A4,4 0 1 0 8,12", "M12 2v2", "M12 20v2", "m4.93 4.93 1.41 1.41", "m17.66 17.66 1.41 1.41", "M2 12h2", "M20 12h2", "m6.34 17.66-1.41 1.41", "m19.07 4.93-1.41 1.41") }
    val Monitor: ImageVector by lazy { lucide("Monitor", "M4,3h16a2,2 0 0 1 2,2v10a2,2 0 0 1 -2,2h-16a2,2 0 0 1 -2,-2v-10a2,2 0 0 1 2,-2z", "M8,21L16,21", "M12,17L12,21") }
    val ShieldCheck: ImageVector by lazy { lucide("ShieldCheck", "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z", "m9 12 2 2 4-4") }
    val HardDrive: ImageVector by lazy { lucide("HardDrive", "M10 16h.01", "M2.212 11.577a2 2 0 0 0-.212.896V18a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-5.527a2 2 0 0 0-.212-.896L18.55 5.11A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z", "M21.946 12.013H2.054", "M6 16h.01") }
    val Check: ImageVector by lazy { lucide("Check", "M20 6 9 17l-5-5") }
    val ArrowRight: ImageVector by lazy { lucide("ArrowRight", "M5 12h14", "m12 5 7 7-7 7") }
    val ChevronRight: ImageVector by lazy { lucide("ChevronRight", "m9 18 6-6-6-6") }
    val Play: ImageVector by lazy { lucide("Play", "m6 3 14 9-14 9z") }
    val Square: ImageVector by lazy { lucide("Square", "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z") }
    // ── 组 2 dashboard FR 图标（lucide 官方 path data 逐字移植；rect/circle 按既有约定转 path）──
    val CalendarDays: ImageVector by lazy {
        lucide(
            "CalendarDays",
            "M8 2v3",
            "M16 2v3",
            // lucide rect x=3 y=3 width=18 height=18 rx=2
            "M5,3h14a2,2 0 0 1 2,2v14a2,2 0 0 1 -2,2h-14a2,2 0 0 1 -2,-2v-14a2,2 0 0 1 2,-2z",
            "M3 9h18",
            "M8 13h.01",
            "M12 13h.01",
            "M16 13h.01",
            "M8 17h.01",
            "M12 17h.01",
            "M16 17h.01",
        )
    }
    val Brain: ImageVector by lazy {
        lucide(
            "Brain",
            "M12 18V5",
            "M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4",
            "M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5",
            "M17.997 5.125a4 4 0 0 1 2.526 5.77",
            "M18 18a4 4 0 0 0 2-7.464",
            "M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517",
            "M6 18a4 4 0 0 1-2-7.464",
            "M6.003 5.125a4 4 0 0 0-2.526 5.77",
        )
    }
    val Clock: ImageVector by lazy {
        lucide(
            "Clock",
            // lucide circle cx=12 cy=12 r=10（同 Target 圆弧写法）
            "M2,12A10,10 0 1 0 22,12A10,10 0 1 0 2,12",
            "M12 6v6l4 2",
        )
    }
    val ChevronDown: ImageVector by lazy { lucide("ChevronDown", "m6 9 6 6 6-6") }
    // ── 组 3 role/memory FR 图标（lucide 官方 path data 逐字移植；circle 按既有约定转 path）──
    val Trash2: ImageVector by lazy {
        lucide(
            "Trash2",
            "M3 6h18",
            "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6",
            "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
            // lucide circle cx=12 cy=11 r=1
            "M11,11A1,1 0 1 0 13,11A1,1 0 1 0 11,11",
            // lucide circle cx=12 cy=17 r=1
            "M11,17A1,1 0 1 0 13,17A1,1 0 1 0 11,17",
        )
    }
    val X: ImageVector by lazy { lucide("X", "M18 6 6 18", "m6 6 12 12") }
    // ── 组 4 tasks FR-23 图标（lucide 官方 path data 逐字移植）──
    val Filter: ImageVector by lazy { lucide("Filter", "M22 3H2l8 9.46V19l4 2v-8.54L22 3") }
    val Loader2: ImageVector by lazy { lucide("Loader2", "M21 12a9 9 0 1 1-6.219-8.56") }
}
