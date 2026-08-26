package com.egosync.companion.ui.theme

import android.provider.Settings
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext

// ═══════════════════════════════════════════════════════════════════════
// 动效白名单常量 — 动效克制原则：仅角色卡呼吸 + 功能性反馈，无装饰动效。
// 母本：egosync-app/src/index.css（--duration-breath / --duration-color /
// prefers-reduced-motion 全局降级）。详见 README「与桌面端的设计语言对照表」。
// ═══════════════════════════════════════════════════════════════════════

/** 呼吸全周期 — 母本 --duration-breath: 3s（opacity 0.6↔1.0 ease-in-out 无限循环）。 */
const val BreathDurationMillis = 3000

/** 色温过渡 — 母本 --duration-color: 300ms（进入角色时 --role-accent 切换过渡）。 */
const val ColorTransitionMillis = 300

/**
 * reduced-motion（web prefers-reduced-motion: reduce 的平台等价物）：
 * 系统无障碍「移除动画」把三项动画时长缩放置 0，取 ANIMATOR_DURATION_SCALE==0 判定。
 * 为 true 时白名单动效全部静态化（各消费点处理），读取一次并缓存。
 */
@Composable
fun rememberReducedMotion(): Boolean {
    val context = LocalContext.current
    return remember {
        Settings.Global.getFloat(
            context.contentResolver,
            Settings.Global.ANIMATOR_DURATION_SCALE,
            1f,
        ) == 0f
    }
}
