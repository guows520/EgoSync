package com.egosync.companion.sync

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 分帧重组器契约（镜像桌面 companion_snapshot.rs reassemble + test_companion.rs
 * phone_receive_snapshot）：按 seq 收齐 total 片 → base64 解码 → 拼接为完整明文。
 *
 * WHY：桌面按 48,000 字节切片发送快照，任一片损坏/错序若被静默容忍，手机要么
 * 永远收不齐（界面空白），要么拼出损坏载荷渲染半截数据——两者都比显式报错更糟。
 * 连续性断言在收齐时进行（镜像桌面 sort_by_key 后逐位校验），乱序到达本身合法。
 */
class ChunkReassemblerTest {

    /** 按桌面 frame_snapshot 手法构造 envelope JSON（camelCase）。 */
    private fun envelope(seq: Int, total: Int, chunk: ByteArray): String {
        val b64 = java.util.Base64.getEncoder().encodeToString(chunk)
        return """{"seq":$seq,"total":$total,"chunkBase64":"$b64"}"""
    }

    /** 将明文按给定尺寸切片并打乱顺序（重组必须容忍乱序到达，镜像桌面 sort_by_key）。
     *  seq=0 不作尾片：迟到 seq=0 与「新序列起点」不可区分（重开缓冲语义，12.4 P5），
     *  固定乱序尾片为非零序号，锁定「乱序合法」契约本身且消除随机性（稳定复现）。 */
    private fun shuffledEnvelopes(plain: ByteArray, chunkSize: Int): List<String> {
        val total = (plain.size + chunkSize - 1) / chunkSize
        val shuffled = plain.toList().chunked(chunkSize).mapIndexed { i, c ->
            envelope(i, total, c.toByteArray())
        }.shuffled().toMutableList()
        val seq0Idx = shuffled.indexOfFirst { it.contains(""""seq":0,""") }
        if (seq0Idx > 0) {
            // 与首位互换：seq=0 前置，其余保持乱序——尾片恒为非零序号
            val head = shuffled[seq0Idx]
            shuffled[seq0Idx] = shuffled.first()
            shuffled[0] = head
        }
        return shuffled
    }

    @Test
    fun `多片乱序到达收齐后重组出原始明文`() {
        // WHY：桌面整序列持锁入队，但接收侧仍须按 seq 排序重组而非按到达顺序
        // 拼接——任何乱序都会产出损坏载荷
        val plain = """{"schemaVersion":1,"roles":[1,2,3,4,5]}""".toByteArray()
        val reassembler = ChunkReassembler()

        val envelopes = shuffledEnvelopes(plain, chunkSize = 8)
        envelopes.dropLast(1).forEach { assertNull(reassembler.offer(it)) }
        assertArrayEquals(plain, reassembler.offer(envelopes.last()))
    }

    @Test
    fun `单片序列一次到达即完成`() {
        val plain = "single-chunk".toByteArray()
        val reassembler = ChunkReassembler()

        assertArrayEquals(plain, reassembler.offer(envelope(0, 1, plain)))
    }

    @Test
    fun `未收齐时返回 null 不产出半截数据`() {
        // WHY：total=3 只到 2 片就拼载荷会得到截断的损坏 JSON——必须等收齐，宁空勿残
        val reassembler = ChunkReassembler()
        assertNull(reassembler.offer(envelope(0, 3, "aaa".toByteArray())))
        assertNull(reassembler.offer(envelope(2, 3, "ccc".toByteArray()))) // 乱序合法，仍缺片
    }

    @Test
    fun `收齐时连续性断言拒绝跳号与重复片`() {
        // WHY：丢片（跳号）或重复投递（重连后半截重发）若被静默接受，收齐校验
        // 形同虚设——排序后逐位校验 0..total-1（镜像桌面 reassemble 断言）
        val reassembler = ChunkReassembler()
        reassembler.offer(envelope(0, 3, "a".toByteArray()))
        reassembler.offer(envelope(2, 3, "c".toByteArray()))

        // 第 3 片仍是 seq=2（重复）→ 计数达 total 但序号断档 [0,2,2] → 显式报错
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer(envelope(2, 3, "c".toByteArray()))
        }
    }

    @Test
    fun `base64 损坏显式报错`() {
        val reassembler = ChunkReassembler()
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer("""{"seq":0,"total":1,"chunkBase64":"!!!not-base64!!!"}""")
        }
    }

    @Test
    fun `envelope JSON 损坏显式报错`() {
        val reassembler = ChunkReassembler()
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer("not-json")
        }
    }

    @Test
    fun `seq 超界显式报错`() {
        // WHY：seq >= total 的 envelope 不属于任何合法序列——静默接收会让计数
        // 永远无法与 total 对齐（白屏）
        val reassembler = ChunkReassembler()
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer(envelope(3, 2, "x".toByteArray()))
        }
    }

    @Test
    fun `负数seq或total显式报错不产出空载荷`() {
        // WHY（13.2 评审 P9）：负值会绕过 seq>=total 与重复检查——收齐判定
        // receivedCount < 负数 恒假、拼接循环 0 until 负数 为空，静默拼出空
        // ByteArray 冒充完整快照，且误导性日志掩盖真实根因
        val reassembler = ChunkReassembler()
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer(envelope(-3, -2, "x".toByteArray()))
        }
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer(envelope(-1, 3, "x".toByteArray()))
        }
        assertThrows(ChunkReassemblyException::class.java) {
            reassembler.offer(envelope(0, -2, "x".toByteArray()))
        }
    }

    @Test
    fun `跨会话泄漏的孤儿片不毒化后续序列`() {
        // WHY：承载切换/重连后，旧会话的迟到孤儿片（seq>0 起始）可能先于新序列
        // 到达——它既不能拼装也不能让重组器卡死；seq=0 新序列必须能正常完成
        val reassembler = ChunkReassembler()
        assertNull(reassembler.offer(envelope(1, 2, "stray".toByteArray()))) // 孤儿片：滞留待新序列覆盖

        reassembler.offer(envelope(0, 2, "a".toByteArray()))
        assertEquals("ab", reassembler.offer(envelope(1, 2, "b".toByteArray()))?.decodeToString())
    }

    @Test
    fun `seq0 到达即重开缓冲容忍重连后的新序列`() {
        // WHY：承载切换/重连后桌面重发全新序列；旧序列残片若不被 seq=0 清掉，
        // 新旧混拼必产出损坏载荷（12.4 P5 同类教训）
        val reassembler = ChunkReassembler()
        reassembler.offer(envelope(0, 3, "old-0".toByteArray()))
        reassembler.offer(envelope(1, 3, "old-1".toByteArray()))
        // 旧序列第 3 片永不到达；新序列 seq=0 到达 → 旧缓冲整体作废
        reassembler.offer(envelope(0, 2, "new-0".toByteArray()))

        assertEquals("new-0new-1", reassembler.offer(envelope(1, 2, "new-1".toByteArray()))?.decodeToString())
    }

    @Test
    fun `重组完成后缓冲清空可承接下一序列`() {
        val reassembler = ChunkReassembler()
        reassembler.offer(envelope(0, 1, "first".toByteArray()))

        assertEquals(
            "second",
            reassembler.offer(envelope(0, 1, "second".toByteArray()))?.decodeToString(),
        )
    }

    @Test
    fun `空明文序列正常完成`() {
        // WHY：空载荷是合法边界输入——处理不当（除零/越界）会让整条会话循环崩溃
        val reassembler = ChunkReassembler()
        val result = reassembler.offer(envelope(0, 1, ByteArray(0)))
        assertTrue(result != null && result.isEmpty())
    }
}
