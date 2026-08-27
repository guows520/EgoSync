package com.egosync.companion.connection

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * FakeConnectionClient 意图验证：
 * 四档 debug 预设必须准确映射到连接三态 + 降级信息，
 * 驱动全局降级遮罩与操作禁用态的实时切换是状态模拟入口的契约。
 */
class FakeConnectionClientTest {

    @Test
    fun `direct 预设映射为局域网直连`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.DIRECT)

        assertEquals(ConnectionState.Direct, client.state.value)
        assertTrue(client.state.value.engineAvailable)
    }

    @Test
    fun `relay 预设映射为中继转发`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.RELAY)

        assertEquals(ConnectionState.Relay, client.state.value)
        assertTrue(client.state.value.engineAvailable)
    }

    @Test
    fun `offline 预设映射为无缓存离线`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.OFFLINE)

        val state = client.state.value
        assertTrue(state is ConnectionState.Offline)
        state as ConnectionState.Offline
        assertFalse(state.snapshotAvailable)
        assertNull(state.dataAsOf)
        assertFalse(state.engineAvailable)
    }

    @Test
    fun `degraded 预设映射为只读缓存降级态`() {
        val client = FakeConnectionClient()
        client.setDebugMode(DebugConnectionMode.DEGRADED)

        val state = client.state.value
        assertTrue(state is ConnectionState.Offline)
        state as ConnectionState.Offline
        assertTrue(state.snapshotAvailable)
        assertEquals("今天 08:15", state.dataAsOf)
        assertFalse(state.engineAvailable)
    }

    @Test
    fun `配对完成与解除配对往返`() {
        val client = FakeConnectionClient(initialPaired = false)

        client.completePairing()
        assertTrue(client.paired.value)

        client.unpair()
        assertFalse(client.paired.value)
    }
}
