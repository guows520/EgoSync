package com.egosync.companion.connection

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 承载状态机意图（AC4/AC5 + SPEC state-and-recovery-model §2/§3）：
 * - 「滞回防抖动」：直连丢失后 3s 探测窗内恢复 → 绝不切中继；
 * - 「探测 3s 仍不可达」→ 才发切中继意图；
 * - 「冷启动 Connecting 宽限」：初值 Connecting，20s 内不降级、耗尽才 Degraded；
 * - 「会话失去 → Reconnecting」宽限重启；「连接尝试失败不重置宽限」（防活锁）；
 * - 「中继态下直连重试失败」不扰动状态（切回直连靠直连握手成功）；
 * - 「Degraded 下重连失败保持 Degraded」不回弹闪烁。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ConnectionStateMachineTest {

    @Test
    fun direct_lost_with_relay_probes_three_seconds_then_switches() = runTest {
        // WHY: AC4 滞回——直连丢失必须先探测满 3s（注入 sleep 记录证明窗长），
        // 窗满且未恢复才允许请求中继承载。
        val sleeps = mutableListOf<Long>()
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { sleeps.add(it) })
        val intents = mutableListOf<ConnectionStateMachine.Intent>()
        machine.onIntent { intents.add(it) }

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()

        assertEquals(listOf(3_000L), sleeps)
        assertEquals(listOf<ConnectionStateMachine.Intent>(ConnectionStateMachine.Intent.StartRelay), intents)
        machine.onRelayEstablished()
        assertEquals(TransportStatus.Relay, machine.state.value)
    }

    @Test
    fun direct_recovery_within_probe_window_cancels_relay_switch() = runTest {
        // WHY: NSD 瞬断在 3s 内恢复时，中继切换必须被取消——否则每次 WiFi
        // 抖动都会把手机甩上中继白耗流量（AC4 prefer-direct 的本意）。
        val sleeps = mutableListOf<Long>()
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { sleeps.add(it) })
        val intents = mutableListOf<ConnectionStateMachine.Intent>()
        machine.onIntent { intents.add(it) }

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        machine.onDirectEstablished() // 瞬断后直连恢复（取消探测窗）
        advanceUntilIdle()

        assertTrue("探测窗内恢复不得发出切中继意图", intents.isEmpty())
        assertTrue("取消的探测窗不得执行 sleep", sleeps.isEmpty())
        assertEquals(TransportStatus.Direct, machine.state.value)
    }

    @Test
    fun direct_lost_without_relay_reconnecting_then_degraded_after_grace() = runTest {
        // WHY（SPEC state-model §2）：无中继时会话失去不立即降级——Reconnecting
        // + 20s 宽限让闪断自我愈合；宽限耗尽才如实 Degraded（旧语义直接 Offline）。
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, graceMs = 100)

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = false)
        assertEquals("会话失去先进 Reconnecting（宽限中）", TransportStatus.Reconnecting, machine.state.value)

        // advanceTimeBy 不执行恰落在目标时刻的任务：推进越过宽限端点
        advanceTimeBy(101)
        val state = machine.state.value
        assertTrue("宽限耗尽必须 Degraded", state is TransportStatus.Degraded)
        assertFalse((state as TransportStatus.Degraded).snapshotAvailable)
    }

    @Test
    fun direct_retry_failure_during_relay_does_not_disturb_state() = runTest {
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        machine.onRelayEstablished()
        assertEquals(TransportStatus.Relay, machine.state.value)

        // 中继态下直连重试又失败：状态必须仍为 Relay（切回靠直连握手成功）
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        assertEquals(TransportStatus.Relay, machine.state.value)
    }

    @Test
    fun relay_lost_means_both_bearers_down_reconnecting_then_degraded() = runTest {
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        machine.onRelayEstablished()

        machine.onRelayLost()
        assertEquals("中继会话结束先进 Reconnecting（宽限中）", TransportStatus.Reconnecting, machine.state.value)
        advanceUntilIdle() // sleep{Unit} → 宽限瞬时耗尽
        val state = machine.state.value
        assertTrue(state is TransportStatus.Degraded)
        assertFalse((state as TransportStatus.Degraded).snapshotAvailable)
        assertEquals(null, (state as TransportStatus.Degraded).dataAsOf)
    }

    @Test
    fun relay_establishment_failure_while_direct_probe_reaches_reconnecting() = runTest {
        // WHY（P2）：中继「从未建立」的失败此前走 onRelayLost——它只在 Relay 态
        // 生效，滞回窗内双承载皆断却永久驻留 Direct（AC5 击穿：UI 误报在线）。
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle() // 探测窗满，StartRelay 已发出；状态仍滞留 Direct
        machine.onRelayEstablishmentFailed() // 中继连接失败（从未建立）

        assertEquals("双承载皆不可达必须 Reconnecting（宽限开始）", TransportStatus.Reconnecting, machine.state.value)
        advanceUntilIdle()
        assertTrue("宽限耗尽如实 Degraded", machine.state.value is TransportStatus.Degraded)
    }

    @Test
    fun late_relay_success_after_direct_recovery_does_not_override_direct() = runTest {
        // WHY（P18）：探测窗满发出 StartRelay 的同刻直连恢复——迟到的中继建立
        // 不得覆盖 Direct，否则状态机自替换健康会话并瞬时误报。
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        machine.onDirectEstablished() // 直连恰在此刻恢复
        machine.onRelayEstablished() // 迟到的中继成功到达

        assertEquals("直连在线时中继不得夺位", TransportStatus.Direct, machine.state.value)
    }

    // ── T-S3：冷启动 Connecting 宽限（SPEC state-model §3）────────────

    @Test
    fun cold_start_connecting_within_grace_not_degraded() = runTest {
        // WHY：冷启动第一帧 Connecting（取代旧「即 Offline + 全屏遮罩」）——
        // 20s 宽限是桌面启动/网络就绪的现实窗口，宽限内不得降级惊扰。
        val machine = ConnectionStateMachine(scope = this, graceMs = 100)

        assertEquals("初值即 Connecting", TransportStatus.Connecting, machine.state.value)
        advanceTimeBy(99)
        assertEquals("宽限内不得 Degraded", TransportStatus.Connecting, machine.state.value)
        // advanceTimeBy 不执行恰落在目标时刻的任务：推进越过宽限端点
        advanceTimeBy(2)
        assertTrue("宽限耗尽必须 Degraded", machine.state.value is TransportStatus.Degraded)
    }

    @Test
    fun grace_cancelled_once_session_established() = runTest {
        // WHY：会话建立即取消宽限——已在线后不应有残留计时器把状态翻成 Degraded。
        val machine = ConnectionStateMachine(scope = this, graceMs = 100)
        advanceTimeBy(50)
        machine.onDirectEstablished()
        advanceTimeBy(500)
        assertEquals(TransportStatus.Direct, machine.state.value)

        // 中继建立同样取消宽限
        machine.onDirectLost(relayAvailable = false)
        machine.onRelayEstablished()
        advanceTimeBy(500)
        assertEquals(TransportStatus.Relay, machine.state.value)
    }

    @Test
    fun attempt_failure_does_not_reset_grace() = runTest {
        // WHY（防活锁）：退避重连每秒失败一次——若每次失败都重置宽限，
        // Degraded 永远不会到达，用户看不到任何降级提示。
        val machine = ConnectionStateMachine(scope = this, graceMs = 100)
        advanceTimeBy(50)
        machine.onRelayEstablishmentFailed() // 连接尝试失败（Connecting 保持，宽限不重置）
        // advanceTimeBy 不执行恰落在目标时刻的任务：推进越过宽限端点
        advanceTimeBy(51)
        assertTrue("宽限须自冷启动起算满 100ms 即 Degraded", machine.state.value is TransportStatus.Degraded)
    }

    @Test
    fun degraded_stays_on_retry_failure_and_recovers_on_establish() = runTest {
        // WHY（SPEC state-model §2）：Degraded ≠ 停止重连，但重连失败也不回弹
        // Connecting/Reconnecting（横幅闪烁）；恢复的唯一出口是会话建立。
        val machine = ConnectionStateMachine(scope = this, graceMs = 100)
        // advanceTimeBy 不执行恰落在目标时刻的任务：推进越过宽限端点
        advanceTimeBy(101)
        assertTrue(machine.state.value is TransportStatus.Degraded)

        machine.onRelayEstablishmentFailed()
        assertTrue("降级下重试失败保持 Degraded", machine.state.value is TransportStatus.Degraded)
        machine.onDirectLost(relayAvailable = false)
        assertTrue(machine.state.value is TransportStatus.Degraded)

        machine.onDirectEstablished()
        assertEquals("后台退避重连成功即回 Direct", TransportStatus.Direct, machine.state.value)
    }
}
