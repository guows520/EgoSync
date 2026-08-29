package com.egosync.companion.connection

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 三态状态机意图（AC4/AC5）：
 * - 「滞回防抖动」：直连丢失后 3s 探测窗内恢复 → 绝不切中继；
 * - 「探测 3s 仍不可达」→ 才发切中继意图；
 * - 「中继态下直连重试失败」不扰动状态（切回直连靠直连握手成功）；
 * - 「双承载均失败」→ Offline（false, null），不误报有缓存。
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
        assertEquals(ConnectionState.Relay, machine.state.value)
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
        assertEquals(ConnectionState.Direct, machine.state.value)
    }

    @Test
    fun direct_lost_without_relay_goes_offline_immediately() = runTest {
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })
        val intents = mutableListOf<ConnectionStateMachine.Intent>()
        machine.onIntent { intents.add(it) }

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = false)
        advanceUntilIdle()

        val state = machine.state.value
        assertTrue(state is ConnectionState.Offline)
        assertFalse((state as ConnectionState.Offline).snapshotAvailable)
        assertTrue("无中继可用时不应发出切中继意图", intents.isEmpty())
    }

    @Test
    fun direct_retry_failure_during_relay_does_not_disturb_state() = runTest {
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })
        val intents = mutableListOf<ConnectionStateMachine.Intent>()
        machine.onIntent { intents.add(it) }

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        machine.onRelayEstablished()
        assertEquals(ConnectionState.Relay, machine.state.value)

        // 中继态下直连重试又失败：状态必须仍为 Relay（切回靠直连握手成功）
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        assertEquals(ConnectionState.Relay, machine.state.value)
    }

    @Test
    fun relay_lost_means_both_bearers_down_offline_no_snapshot() = runTest {
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle()
        machine.onRelayEstablished()

        machine.onRelayLost()
        val state = machine.state.value
        assertTrue(state is ConnectionState.Offline)
        assertFalse((state as ConnectionState.Offline).snapshotAvailable)
        assertEquals(null, (state as ConnectionState.Offline).dataAsOf)
    }

    @Test
    fun relay_establishment_failure_while_direct_probe_reaches_offline() = runTest {
        // WHY（P2）：中继「从未建立」的失败此前走 onRelayLost——它只在 Relay 态
        // 生效，Direct 滞回窗内双承载皆断却永久驻留 Direct（AC5 击穿：UI 误报在线）。
        val machine = ConnectionStateMachine(scope = this, hysteresisMs = 3_000, sleep = { Unit })

        machine.onDirectEstablished()
        machine.onDirectLost(relayAvailable = true)
        advanceUntilIdle() // 探测窗满，StartRelay 已发出；状态仍滞留 Direct
        machine.onRelayEstablishmentFailed() // 中继连接失败（从未建立）

        val state = machine.state.value
        assertTrue("双承载皆不可达必须 Offline", state is ConnectionState.Offline)
        assertFalse((state as ConnectionState.Offline).snapshotAvailable)
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

        assertEquals("直连在线时中继不得夺位", ConnectionState.Direct, machine.state.value)
    }
}
