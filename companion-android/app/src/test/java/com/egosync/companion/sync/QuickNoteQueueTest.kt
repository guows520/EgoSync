package com.egosync.companion.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 速记队列（FR-43）意图验证：
 * 降级态的速记不能丢——恢复连接后必须完整提交并清队。
 */
class QuickNoteQueueTest {

    @Test
    fun `提交速记进入队列并计数`() {
        val queue = QuickNoteQueue()
        queue.submit("给母亲回电话")
        queue.submit("买钢琴教材")

        assertEquals(2, queue.pendingCount())
        assertEquals(2, queue.items.value.size)
    }

    @Test
    fun `空白速记不入队`() {
        val queue = QuickNoteQueue()
        queue.submit("   ")

        assertEquals(0, queue.pendingCount())
    }

    @Test
    fun `每条速记带唯一幂等ID`() {
        val queue = QuickNoteQueue()
        queue.submit("a")
        queue.submit("b")

        val ids = queue.items.value.map { it.id }
        assertEquals(ids.size, ids.toSet().size)
    }

    @Test
    fun `恢复连接后清队且幂等`() {
        val queue = QuickNoteQueue()
        queue.submit("待同步速记")
        queue.flush()

        assertEquals(0, queue.pendingCount())
        // 二次 flush 无副作用（断线抖动场景）
        queue.flush()
        assertTrue(queue.items.value.isEmpty())
    }
}
