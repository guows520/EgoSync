package com.egosync.companion.ui

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.Dp
import com.egosync.companion.connection.CompanionLog

/**
 * T-S8-A IME 布局诊断（SPEC ime-diagnosis.md §A.1）：一次 composer 焦点切换
 * （键盘收起→弹出）前后各记录一轮的几何快照。
 *
 * WHY 先诊断后修复：症状 Confirmed（输入条与键盘间数百像素空区），但各层
 * Insets 的精确像素贡献未由运行数据确认——仓库既往只修过嵌套 Scaffold 的
 * system-bars 顶部双重 inset，IME 无仪器记录；禁止跳过数据直接宣称
 * 「adjustResize + imePadding() 双重计算」为根因（AGENTS 规则十三）。
 *
 * 仅 debug 构建接线（调用点以 BuildConfig.DEBUG 门控，release 侧常量折叠
 * + R8 死代码消除，零引用零开销）；仅记录几何数值，绝不记录用户输入内容。
 * 日志 tag 独立（ImeDiag）便于 adb 过滤：`adb logcat -s ImeDiag`。
 */
object ImeDiagnostics {
    /** 根容器（MainActivity 外层 Scaffold）bounds。 */
    var rootBounds: Rect? = null

    /** 外层 Scaffold content Box bounds（MainActivity）。 */
    var outerContentBounds: Rect? = null

    /** 内层 Scaffold content Box bounds（AppNavHost MainShell）。 */
    var innerContentBounds: Rect? = null

    /** 内层 bottomBar NavigationBar bounds。 */
    var navigationBarBounds: Rect? = null

    /** composer（ChatScreen 输入条 Surface）bounds——唯一底部输入面（速记条已退役）。 */
    var composerBounds: Rect? = null

    /** 外层 Scaffold content padding 的 bottom 分量。 */
    var outerBottomPadding: Dp? = null

    /** 内层 Scaffold content padding 的 bottom 分量。 */
    var innerBottomPadding: Dp? = null

    /** 输出一轮快照。调用时机：焦点切换（「前」）与 ime inset 值变化（动画完成后的「后」）。 */
    fun snapshot(trigger: String, imeBottomPx: Int, navBarBottomPx: Int) {
        CompanionLog.info(
            "ImeDiag",
            "IME_DIAG trigger=$trigger root=${fmt(rootBounds)} outer=${fmt(outerContentBounds)} " +
                "inner=${fmt(innerContentBounds)} navbar=${fmt(navigationBarBounds)} " +
                "composer=${fmt(composerBounds)} outerPadB=${outerBottomPadding} " +
                "innerPadB=${innerBottomPadding} imeBottom=${imeBottomPx}px navInsetB=${navBarBottomPx}px",
        )
    }

    private fun fmt(rect: Rect?): String =
        rect?.let { "[%.0f,%.0f,%.0f,%.0f]".format(it.left, it.top, it.right, it.bottom) } ?: "null"
}
