package com.egosync.companion.ui

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * 锁定冷启动两态路由契约（2026-09-10 裁决：手机端引导流程移除，FR-21 移动侧退役）：
 * 已配对用户必须直达主界面——桌面是唯一事实源，初始设置在桌面完成；
 * 历史设备上遗留的 onboarded 标记（缺失或为假）都不得再把用户拦进引导屏。
 */
class AppNavHostTest {

    @Test
    fun coldStartDestination_unpairedLandsOnPairing() {
        // 未配对是配对流的唯一入口，回归不变项
        assertEquals(Routes.PAIRING, coldStartDestination(paired = false))
    }

    @Test
    fun coldStartDestination_pairedLandsOnDashboard() {
        // 已配对直达主界面：判定只看 paired，不存在第三个分支
        assertEquals(Routes.DASHBOARD, coldStartDestination(paired = true))
    }
}
