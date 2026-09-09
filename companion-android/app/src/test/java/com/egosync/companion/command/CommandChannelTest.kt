package com.egosync.companion.command

import com.egosync.companion.connection.Frame
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 指令通道契约测试（Story 13.3 T9，AC:1/7）。
 *
 * WHY：pending 表跨会话存活 + 出站 pump 分离是会话断开时「在途指令明确失败」
 * 而非悬挂的前提；超时与未知回执路径决定手机端永不无限等待。
 * T-S2：sessionActive 升级为可观察 StateFlow——commandReady 是写操作与
 * flush 守门的唯一判据，bind/unbind/shutdown 的同步发布语义在此锁定。
 */
class CommandChannelTest {

    // ── T-S2：sessionActive / commandReady 事实源 ─────────────────────

    @Test
    fun `sessionActive随bind与unbind同步发布`() = runTest {
        // WHY：布尔 getter 的竞窗读数会让 flush 守门/UI enabled 误判可发送——
        // bind 即刻 true、unbind 即刻 false（先于 pending 失败与状态机处理）。
        val channel = CommandChannel(ackTimeoutMs = 60_000)
        assertFalse(channel.sessionActive.value)

        val binding = channel.bind()
        assertTrue(channel.sessionActive.value)

        channel.unbind(binding)
        assertFalse(channel.sessionActive.value)
    }

    @Test
    fun `新绑定接管后旧unbind不得拉低sessionActive`() = runTest {
        // WHY：会话切换（新会话 bind 先于旧 loop 的 unbind 到达）时旧 unbind
        // 必须是 no-op——否则新会话明明在线却被误报不可发送。
        val channel = CommandChannel(ackTimeoutMs = 60_000)
        val oldBinding = channel.bind()
        val newBinding = channel.bind()
        assertTrue(channel.sessionActive.value)

        channel.unbind(oldBinding) // 旧绑定已非当前绑定
        assertTrue("旧 unbind 不得误关新会话的 sessionActive", channel.sessionActive.value)

        channel.unbind(newBinding)
        assertFalse(channel.sessionActive.value)
    }

    @Test
    fun `shutdown同步发布sessionActive为false`() = runTest {
        val channel = CommandChannel(ackTimeoutMs = 60_000)
        channel.bind()
        channel.shutdown()
        assertFalse(channel.sessionActive.value)
    }

    @Test
    fun `发送出站帧并由回执完成`() = runTest {
        val channel = CommandChannel(ackTimeoutMs = 5_000)
        val binding = channel.bind()

        val job = launch {
            val result = channel.execute("memory.list", """{"roleId":"r-1"}""")
            assertEquals(3, result.optInt("count"))
        }

        // 出站帧到达 pump 源，data 为 envelope JSON（含 action 与 commandId）
        val frame = binding.outbound.receive() as Frame.Command
        val envelope = JSONObject(frame.data)
        assertEquals("memory.list", envelope.optString("action"))
        val commandId = envelope.optString("commandId")
        assertTrue(commandId.isNotEmpty())

        // 回执按 commandId 命中 pending
        channel.onCommandResult(
            """{"schemaVersion":1,"commandId":"$commandId","ok":true,"result":{"count":3}}""",
        )
        job.join()
    }

    @Test
    fun `超时无回执抛超时异常`() = runTest {
        val channel = CommandChannel(ackTimeoutMs = 100)
        channel.bind()

        val e = runCatching {
            channel.execute("memory.list", "{}")
        }.exceptionOrNull() as CommandException
        // 看门狗：dispatch 为 spawn 短路径，超时即链路死亡——必须显式失败
        assertEquals("ConnectionError", e.variant)
        assertTrue(e.message!!.contains("超时"))
    }

    @Test
    fun `会话断开在途指令全部失败`() = runTest {
        val channel = CommandChannel(ackTimeoutMs = 60_000)
        val binding = channel.bind()

        val first = async { runCatching { channel.execute("chat.send", "{}") } }
        val second = async { runCatching { channel.execute("task.toggle", "{}") } }
        // 排空出站（pump 语义）确保指令已注册 pending
        binding.outbound.receive()
        binding.outbound.receive()
        runCurrent()

        channel.unbind(binding)
        assertTrue(first.await().isFailure)
        assertTrue(second.await().isFailure)
        val e = second.await().exceptionOrNull() as CommandException
        assertTrue(e.message!!.contains("连接已断开"))
    }

    @Test
    fun `未绑定会话直接失败不悬挂`() = runTest {
        val channel = CommandChannel()
        val e = runCatching { channel.execute("chat.send", "{}") }
            .exceptionOrNull() as CommandException
        assertEquals("桌面引擎不可达", e.message)
    }

    @Test
    fun `注册pending后绑定失效立即失败不送孤儿帧`() = runTest {
        // WHY: execute 快照 binding 与注册 pending 之间存在 TOCTOU 窗口——
        // unbind 若发生在二者之间，failAllPending 清不到该 pending，帧被
        // 送进已死绑定只能等超时。send 前复查绑定即可完整关闭该窗口。
        val channel = CommandChannel(ackTimeoutMs = 60_000)
        val binding = channel.bind()
        // 模拟交错：execute 读到 binding 后、send 前，会话恰好断开
        val job = async { runCatching { channel.execute("chat.send", "{}") } }
        // 让 execute 走到 send 前（虚拟时间立即），随后立刻 unbind
        runCurrent()
        channel.unbind(binding)
        val result = job.await()
        assertTrue("绑定失效后不得等超时", result.isFailure)
        val e = result.exceptionOrNull() as CommandException
        assertEquals("ConnectionError", e.variant)
    }

    @Test
    fun `未知commandId回执不误完成已注册pending`() = runTest {
        // WHY: 原零断言版本只能证明「没抛异常」，无法证明「ghost 回执不误完成
        // 真实 pending」。注册一条真 pending，送 ghost 回执，断言该 pending
        // 仍挂起，随后送正确回执才能完成。
        val channel = CommandChannel(ackTimeoutMs = 5_000)
        val binding = channel.bind()
        val job = launch { channel.execute("memory.list", "{}") }
        val commandId = JSONObject((binding.outbound.receive() as Frame.Command).data)
            .optString("commandId")

        // 迟到/重放 ghost 回执：不误完成真实 pending
        channel.onCommandResult(
            """{"schemaVersion":1,"commandId":"ghost","ok":true,"result":{}}""",
        )
        channel.onCommandResult("坏 json")
        runCurrent()
        assertTrue("ghost 回执不得误完成真实 pending", job.isActive)

        // 正确回执到达才完成
        channel.onCommandResult(
            """{"schemaVersion":1,"commandId":"$commandId","ok":true,"result":{}}""",
        )
        job.join()
    }

    @Test
    fun `错误回执抛携带变体的异常`() = runTest {
        val channel = CommandChannel(ackTimeoutMs = 5_000)
        val binding = channel.bind()
        val job = launch {
            val e = runCatching { channel.execute("task.toggle", "{}") }
                .exceptionOrNull() as CommandException
            assertEquals("NotFound", e.variant)
        }
        val commandId = JSONObject((binding.outbound.receive() as Frame.Command).data)
            .optString("commandId")
        channel.onCommandResult(
            """{"schemaVersion":1,"commandId":"$commandId","ok":false,"error":{"NotFound":"任务不存在"}}""",
        )
        job.join()
    }
}
