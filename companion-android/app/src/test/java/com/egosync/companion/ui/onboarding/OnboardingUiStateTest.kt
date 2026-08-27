package com.egosync.companion.ui.onboarding

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 锁定 FR-21 引导步进契约（桌面基线 OnboardingView.tsx L205/L213-217/L351/L362）：
 * 步进必须第 5 步封顶——越界步数会让 placeholder 数组越界崩溃；
 * 第 5 步必须当场标记完成——否则中途杀进程后用户被重复引导；
 * 跳过链接第 2 步起才可见——第 1 步连一句话都没说就提供跳过，空态引导失去意义。
 */
class OnboardingUiStateTest {

    @Test
    fun nextStep_incrementsAndCapsAtMaxStep() {
        // 桌面 Math.min(step + 1, 5)：第 5 步封顶，不再增长
        assertEquals(2, OnboardingUiState.nextStep(1))
        assertEquals(5, OnboardingUiState.nextStep(4))
        assertEquals(5, OnboardingUiState.nextStep(5))
    }

    @Test
    fun marksCompletion_trueOnlyWhenReachingMaxStep() {
        // 桌面 L215：nextStep >= 5 才标记完成；前几步发送不应提前终结引导
        assertFalse(OnboardingUiState.marksCompletion(OnboardingUiState.nextStep(1)))
        assertFalse(OnboardingUiState.marksCompletion(OnboardingUiState.nextStep(3)))
        assertTrue(OnboardingUiState.marksCompletion(OnboardingUiState.nextStep(4)))
        assertTrue(OnboardingUiState.marksCompletion(OnboardingUiState.nextStep(5)))
    }

    @Test
    fun skipVisible_fromStepTwoOnwards() {
        // 桌面 L362：step >= 2 才渲染「跳过角色引导」
        assertFalse(OnboardingUiState.skipVisible(1))
        assertTrue(OnboardingUiState.skipVisible(2))
        assertTrue(OnboardingUiState.skipVisible(5))
    }

    @Test
    fun placeholderFor_mirrorsDesktopIndexing_capOutOfBounds() {
        // 桌面 L351 placeholders[min(step, len-1)]：索引即 step（首条桌面本身不展示），
        // 越界步数封顶到末条「好的，创建吧！」，不抛错
        assertEquals("比如：我是一个产品经理，最近在忙新产品上线...", OnboardingUiState.placeholderFor(1))
        assertEquals("好的，创建吧！", OnboardingUiState.placeholderFor(5))
        assertEquals("好的，创建吧！", OnboardingUiState.placeholderFor(99))
    }

    @Test
    fun proposalRound_isSecondCompletedReply() {
        // 桌面语义：访谈中段（第 2 轮回复后）浮现角色提议，太早无上下文、太晚流失用户
        assertEquals(2, OnboardingUiState.PROPOSAL_ROUND)
    }
}
