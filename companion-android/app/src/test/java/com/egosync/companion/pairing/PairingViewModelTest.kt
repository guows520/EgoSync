package com.egosync.companion.pairing

import com.egosync.companion.connection.FakeConnectionClient
import com.egosync.companion.connection.PairingConnector
import com.egosync.companion.connection.PairingProgress
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/** 真实配对路径测试替身：进度可编程推进，记录 pairWithQr 输入。 */
private class FakePairingConnector : PairingConnector {
    override val pairingProgress = MutableStateFlow<PairingProgress>(PairingProgress.Idle)
    var pairWithQrInput: String? = null
    override fun pairWithQr(qrJson: String) {
        pairWithQrInput = qrJson
    }
}

/** 与桌面 generate_qr_payload 契约一致的合法 QR JSON（camelCase 四字段，64-hex 公钥）。 */
private val VALID_QR_JSON =
    """{"relayAddr":null,"desktopStaticPubkey":"${"ab".repeat(32)}","relayId":"1122334455667788","pairingNonce":"n-1"}"""

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

    // ── 真实路径（Story 12.4 Task 7）：PairingConnector 进度驱动 UI ──

    @Test
    fun `残缺二维码被拒且不发起配对`() {
        // WHY：残缺 QR 若被放行，用户会停留在「连接中」直到超时——
        // 解析闸门是唯一防线，必须整体拒绝并给出原因。
        val pairing = FakePairingConnector()
        val vm = PairingViewModel(FakeConnectionClient(), pairing)
        vm.startScan()

        vm.onQrScanned("""{"relayId":"1122334455667788"}""")

        assertEquals(PairingStep.SCAN, vm.step.value)
        assertNull("残缺 QR 不得触发真实配对", pairing.pairWithQrInput)
        assertNotNull("必须给出失败原因", vm.pairingError.value)
    }

    @Test
    fun `真实进度驱动连接三阶段并到达成功页`() {
        // WHY：进度→阶段映射错位（如停在「发现设备」却已握手成功）会让
        // 用户误判卡死而重启配对——阶段必须如实跟随握手推进。
        val pairing = FakePairingConnector()
        val vm = PairingViewModel(FakeConnectionClient(), pairing)
        vm.startScan()

        vm.onQrScanned(VALID_QR_JSON)
        assertEquals("合法 QR 必须转交真实配对器", VALID_QR_JSON, pairing.pairWithQrInput)
        assertEquals(PairingStep.CONNECTING, vm.step.value)

        pairing.pairingProgress.value = PairingProgress.ExchangingKeys
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, vm.connectStage.value)

        pairing.pairingProgress.value = PairingProgress.VerifyingIdentity
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(2, vm.connectStage.value)

        pairing.pairingProgress.value = PairingProgress.Success
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(PairingStep.SUCCESS, vm.step.value)
        assertNull(vm.pairingError.value)
    }

    @Test
    fun `换绑待桌面确认时停留连接页并呈现等待文案`() {
        // WHY：换绑 pending 是正常中间态——若呈现为失败，用户会在桌面
        // 确认前反复重扫，nonce 单次有效导致确认后也连不上。
        val pairing = FakePairingConnector()
        val vm = PairingViewModel(FakeConnectionClient(), pairing)
        vm.startScan()
        vm.onQrScanned(VALID_QR_JSON)

        pairing.pairingProgress.value = PairingProgress.WaitDesktopConfirm
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("必须停留连接页等待", PairingStep.CONNECTING, vm.step.value)
        assertTrue("必须呈现等待桌面确认", vm.waitDesktopConfirm.value)
    }

    @Test
    fun `配对失败回扫码页并展示原因`() {
        // WHY：失败只退页不说明原因，用户无从定位（二维码过期？网络不通？）。
        val pairing = FakePairingConnector()
        val vm = PairingViewModel(FakeConnectionClient(), pairing)
        vm.startScan()
        vm.onQrScanned(VALID_QR_JSON)

        pairing.pairingProgress.value = PairingProgress.Failed("未发现桌面设备")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(PairingStep.SCAN, vm.step.value)
        assertEquals("未发现桌面设备", vm.pairingError.value)
    }
}
