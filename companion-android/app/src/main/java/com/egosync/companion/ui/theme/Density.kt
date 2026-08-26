package com.egosync.companion.ui.theme

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * 信息密度双模式 — PRD §4.14「对话流轻量（像聊天），仪表盘信息密集（像控制台）
 * ——两种模式自然切换」。各屏在入口声明所属模式取用，不做运行时用户切换。
 */
enum class InfoDensity {
    /** 对话流·轻量：气泡宽松、流距适中。 */
    CONVERSATIONAL,

    /** 控制台·密集：单元紧凑、信息格密排。 */
    CONSOLE,
}

/**
 * 密度 token —— 取值锚定桌面母本组件参数：
 * - CONVERSATIONAL ← ChatStream.tsx space-y-3(12px) 流距 / ChatBubble.tsx px-5 py-3
 *   气泡的移动近似档 = 移动端现状值；
 * - CONSOLE ← DashboardTab.tsx grid gap-2.5(10px) / 表单类 px-3 py-2.5 的移动近似档
 *   （FR-38 指标网格等后续屏幕直接复用）。
 */
data class DensitySpec(
    /** 流内条目垂直间距。 */
    val itemSpacing: Dp,
    /** 信息单元（气泡/单元格）水平内边距。 */
    val unitPaddingX: Dp,
    /** 信息单元垂直内边距。 */
    val unitPaddingY: Dp,
)

fun densitySpec(density: InfoDensity): DensitySpec = when (density) {
    InfoDensity.CONVERSATIONAL -> DensitySpec(itemSpacing = 10.dp, unitPaddingX = 16.dp, unitPaddingY = 12.dp)
    InfoDensity.CONSOLE -> DensitySpec(itemSpacing = 10.dp, unitPaddingX = 12.dp, unitPaddingY = 10.dp)
}
