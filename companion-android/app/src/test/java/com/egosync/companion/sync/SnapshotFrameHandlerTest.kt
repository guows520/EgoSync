package com.egosync.companion.sync

import com.egosync.companion.connection.Frame
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Base64

/**
 * 快照帧处理器契约单测（13.2 T3 消费端）：
 * - SNAPSHOT/STATE_DELTA 同一处理路径（13.1 裁决 2）；
 * - 多片分帧收齐后回调一次（onSnapshot 携原始 JSON 供缓存落盘）；
 * - PING/NOTICE 等非快照帧静默忽略；
 * - 重组失败重置缓冲不杀后续序列；解析失败丢弃不回调。
 */
class SnapshotFrameHandlerTest {

    private class Collector {
        val snapshots = mutableListOf<Pair<DesktopSnapshot, ByteArray>>()
    }

    private fun handler(collector: Collector) = SnapshotFrameHandler { snap, raw ->
        collector.snapshots.add(snap to raw)
    }

    private fun envelope(seq: Int, total: Int, chunk: ByteArray): String {
        val b64 = Base64.getEncoder().encodeToString(chunk)
        return """{"seq":$seq,"total":$total,"chunkBase64":"$b64"}"""
    }

    private val snapshotJson = """
        {"schemaVersion":1,"generatedAt":"2026-08-29T12:00:00Z","dataCutoffAt":null,
         "truncated":false,"truncatedDomains":[],
         "roles":[],"tasks":[],
         "dashboard":{"statuses":[],"metrics":{"taskCount":1,"memoryCount":2,
                       "conversationCount":3,"pendingTaskCount":0,
                       "generatedAt":"2026-08-29T12:00:00Z"}},
         "conversations":[],"briefings":[],"weeklyReviews":[],"notifications":[]}
    """.trimIndent()

    /** 按固定片宽切片（顺序投递即可，重组乱序语义已由 ChunkReassemblerTest 锁定）。 */
    private fun envelopesOf(json: String, chunkSize: Int): List<String> {
        val bytes = json.toByteArray()
        val total = (bytes.size + chunkSize - 1) / chunkSize
        return bytes.toList().chunked(chunkSize).mapIndexed { i, c ->
            envelope(i, total, c.toByteArray())
        }
    }

    @Test
    fun `SNAPSHOT多片收齐回调一次且携带原始JSON`() {
        // WHY：SNAPSHOT 建连即全量——收不齐就拼半截会白屏；回调必须同时给
        // 解析产物与原始 JSON（后者供加密缓存落盘，AC3 离线呈现）
        val collector = Collector()
        val h = handler(collector)
        envelopesOf(snapshotJson, chunkSize = 64).forEach { h.onFrame(Frame.Snapshot(it)) }

        assertEquals(1, collector.snapshots.size)
        assertEquals(snapshotJson, collector.snapshots.first().second.decodeToString())
        assertEquals(1, collector.snapshots.first().first.schemaVersion)
    }

    @Test
    fun `STATE_DELTA与SNAPSHOT同一处理路径`() {
        // WHY：13.1 裁决 2——STATE_DELTA 载荷同为全量快照，若两路处理（如只走 diff）
        // 会让写信号触发的更新被丢，UI 停在陈旧数据
        val collector = Collector()
        val h = handler(collector)
        envelopesOf(snapshotJson, chunkSize = 128).forEach { h.onFrame(Frame.StateDelta(it)) }
        assertEquals(1, collector.snapshots.size)
    }

    @Test
    fun `PING与NOTICE帧静默忽略不回调`() {
        // WHY：会话循环里保活/通知帧与快照帧共用通道——误入重组器会污染序列
        val collector = Collector()
        val h = handler(collector)
        h.onFrame(Frame.Ping)
        h.onFrame(Frame.Notice("""{"content":"提醒"}"""))
        assertTrue(collector.snapshots.isEmpty())
    }

    @Test
    fun `未收齐不回调`() {
        val collector = Collector()
        val h = handler(collector)
        val envelopes = envelopesOf(snapshotJson, chunkSize = 64)
        envelopes.dropLast(1).forEach { h.onFrame(Frame.Snapshot(it)) }
        assertTrue(collector.snapshots.isEmpty())
    }

    @Test
    fun `重组失败重置缓冲_后续完整序列仍可成功`() {
        // WHY：载荷级错误（total 漂移等）若不重置重组器，内部状态卡死会让
        // 本会话后续所有快照全部丢失（杀会话是传输层的职责，这里只重置）。
        // 真正触发 total 漂移进 catch 分支：先喂非 0 起始片锁定 expectedTotal=2
        //（seq=0 会无条件重开缓冲、吞掉 total 变化），再喂 total 不同的片 → 异常
        val collector = Collector()
        val h = handler(collector)
        h.onFrame(Frame.Snapshot(envelope(1, 2, "aa".toByteArray())))
        h.onFrame(Frame.Snapshot(envelope(2, 3, "bb".toByteArray()))) // total 漂移 → ChunkReassemblyException → 捕获重置
        assertTrue(collector.snapshots.isEmpty())

        // 重组器已重置：新完整序列正常送达
        envelopesOf(snapshotJson, chunkSize = 64).forEach { h.onFrame(Frame.Snapshot(it)) }
        assertEquals(1, collector.snapshots.size)
    }

    @Test
    fun `解析失败丢弃不回调_后续序列自愈`() {
        // WHY：收齐但内容不是合法快照（schemaVersion 未知/半截 JSON）——内容不可信，
        // 静默丢弃等桌面重发；回调半成品会让 StateMerger 存进垃圾
        val collector = Collector()
        val h = handler(collector)
        val badJson = """{"schemaVersion":99}""" // 未知版本 → SnapshotFormatException
        envelopesOf(badJson, chunkSize = 64).forEach { h.onFrame(Frame.Snapshot(it)) }
        assertTrue(collector.snapshots.isEmpty())

        envelopesOf(snapshotJson, chunkSize = 64).forEach { h.onFrame(Frame.Snapshot(it)) }
        assertEquals(1, collector.snapshots.size)
    }
}
