package com.egosync.companion.connection

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * FakeConnectionClient 意图验证：
 * 六档 debug 预设必须准确映射到承载态 + 降级信息 + commandReady，
 * 驱动顶部状态条与操作禁用态的实时切换是状态模拟入口的契约（T-S3）。
 */
class FakeConnectionClientTest {

    @Test
    fun `direct 预设映射为局域网直连`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.DIRECT)

        assertEquals(TransportStatus.Direct, client.state.value)
        assertTrue(client.commandReady.value)
    }

    @Test
    fun `relay 预设映射为中继转发`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.RELAY)

        assertEquals(TransportStatus.Relay, client.state.value)
        assertTrue(client.commandReady.value)
    }

    @Test
    fun `connecting 与 reconnecting 预设映射为宽限态`() {
        // WHY（T-S3）：宽限两档驱动「紧凑状态条 + 写操作禁用」组合——
        // 展示非在线且 commandReady=false，与真实编排的宽限语义一致。
        val client = FakeConnectionClient()

        client.setDebugMode(DebugConnectionMode.CONNECTING)
        assertEquals(TransportStatus.Connecting, client.state.value)
        assertFalse(client.commandReady.value)

        client.setDebugMode(DebugConnectionMode.RECONNECTING)
        assertEquals(TransportStatus.Reconnecting, client.state.value)
        assertFalse(client.commandReady.value)
    }

    @Test
    fun `offline 预设映射为无缓存降级`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.OFFLINE)

        val state = client.state.value
        assertTrue(state is TransportStatus.Degraded)
        state as TransportStatus.Degraded
        assertFalse(state.snapshotAvailable)
        assertNull(state.dataAsOf)
        assertFalse(client.commandReady.value)
    }

    @Test
    fun `degraded 预设映射为只读缓存降级态`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.DEGRADED)

        val state = client.state.value
        assertTrue(state is TransportStatus.Degraded)
        state as TransportStatus.Degraded
        assertTrue(state.snapshotAvailable)
        assertEquals("今天 08:15", state.dataAsOf)
        assertFalse(client.commandReady.value)
    }

    @Test
    fun `配对完成与解除配对往返`() {
        val client = FakeConnectionClient(initialPaired = false)

        client.completePairing()
        assertTrue(client.paired.value)

        client.unpair()
        assertFalse(client.paired.value)
    }

    @Test
    fun `commandReady随debug档位与配对往返同步`() {
        // WHY（T-S2）：commandReady 是写操作唯一判据——fake 的档位/配对语义
        // 必须同步翻转就绪面，否则 Debug 预览与 VM 测试注入的禁用态失真。
        val client = FakeConnectionClient()
        assertTrue(client.commandReady.value)

        client.setDebugMode(DebugConnectionMode.DIRECT)
        assertTrue(client.commandReady.value)
        client.setDebugMode(DebugConnectionMode.RELAY)
        assertTrue(client.commandReady.value)
        client.setDebugMode(DebugConnectionMode.OFFLINE)
        assertFalse(client.commandReady.value)
        client.setDebugMode(DebugConnectionMode.DEGRADED)
        assertFalse(client.commandReady.value)

        client.completePairing()
        assertTrue(client.commandReady.value)
    }
}
