package com.egosync.companion.ui.theme

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定信息密度双模式契约（I/O 矩阵）。
 * WHY：「对话流像聊天、仪表盘像控制台」是设计意图——轻量模式的呼吸空间必须 ≥
 * 密集模式，否则双模式气质倒挂，「同一个产品」的密度语言失真；
 * 数值有意重调时本测试必须被迫一起评审。
 */
class DensitySpecTest {

    @Test
    fun conversational_values_matchSpecMatrix() {
        val d = densitySpec(InfoDensity.CONVERSATIONAL)
        assertEquals(10.dp, d.itemSpacing)
        assertEquals(16.dp, d.unitPaddingX)
        assertEquals(12.dp, d.unitPaddingY)
    }

    @Test
    fun console_values_matchSpecMatrix() {
        val d = densitySpec(InfoDensity.CONSOLE)
        assertEquals(10.dp, d.itemSpacing)
        assertEquals(12.dp, d.unitPaddingX)
        assertEquals(10.dp, d.unitPaddingY)
    }

    @Test
    fun conversational_units_neverTighterThanConsole() {
        val light = densitySpec(InfoDensity.CONVERSATIONAL)
        val dense = densitySpec(InfoDensity.CONSOLE)
        assertTrue(light.itemSpacing >= dense.itemSpacing)
        assertTrue(light.unitPaddingX >= dense.unitPaddingX)
        assertTrue(light.unitPaddingY >= dense.unitPaddingY)
    }
}
