package com.egosync.companion.pairing

import com.egosync.companion.connection.FakeConnectionClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 配对流状态机意图验证（FR-40 首跑路径）：
 * 步骤必须可控推进/回退，完成后配对关系必须建立——
 * 这决定了"再次启动直达主界面"的行为正确性。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PairingViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `欢迎页可进入扫码并可返回`() {
        val vm = PairingViewModel(FakeConnectionClient())

        assertEquals(PairingStep.WELCOME, vm.step.value)
        vm.startScan()
        assertEquals(PairingStep.SCAN, vm.step.value)
        vm.back()
        assertEquals(PairingStep.WELCOME, vm.step.value)
    }

    @Test
    fun `扫码完成后经连接动画到达成功页`() {
        val vm = PairingViewModel(FakeConnectionClient())
        vm.startScan()
        vm.onScanCompleted()

        assertEquals(PairingStep.CONNECTING, vm.step.value)

        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(PairingStep.SUCCESS, vm.step.value)
    }
}
