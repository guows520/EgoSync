package com.egosync.companion.sync

import org.json.JSONException
import org.json.JSONObject
import java.util.Base64

/**
 * 分帧重组器（镜像桌面 `companion_snapshot.rs` `reassemble` + `test_companion.rs`
 * `phone_receive_snapshot` 的接收侧）。
 *
 * 帧承载（塞在 `Frame.Snapshot/StateDelta.data` 字符串内）为 envelope JSON：
 * `{"seq":0,"total":3,"chunkBase64":"..."}`。接收方收齐 total 片 → base64 解码 →
 * 拼接为完整明文（快照 JSON）。
 *
 * 语义（13.1 裁决 2 + 评审 deferred 收口）：
 * - 乱序到达合法（镜像桌面 `sort_by_key`）；连续性断言在**收齐时**逐位校验 0..total-1。
 * - `seq=0` 到达即重开缓冲——承载切换/重连后桌面重发全新序列，旧残片不混入新序列
 *   （12.4 P5 + 13.1 P3 整改先例）。
 * - 任一步失败（envelope 损坏/base64 损坏/total 漂移/seq 越界/重复片）显式抛
 *   [ChunkReassemblyException]——上层捕获并重置，等待桌面重发完整序列。
 */
class ChunkReassembler {

    /** 当前序列期望的片数；null = 尚未收到任何片。 */
    private var expectedTotal: Int? = null
    /** seq → 片明文字节（base64 解码后）。 */
    private val buffer = LinkedHashMap<Int, ByteArray>()
    /** 已到达片数（含重复——重复片急切报错，此计数仅为状态自洽）。 */
    private var receivedCount = 0

    /**
     * 喂入一个 envelope 的 data 字符串。
     * @return 收齐时返回完整明文字节并清空缓冲；未收齐返回 null。
     * @throws ChunkReassemblyException envelope/数据损坏或序列非法。
     */
    fun offer(data: String): ByteArray? {
        val envelope = parseEnvelope(data)

        // 负数 seq/total（如 seq=-3,total=-2）会绕过 seq>=total 与重复检查，
        // 收齐判定 1 < 负数 恒假、拼接循环 0 until 负数 为空——静默产出空载荷
        // 冒充完整快照（13.2 评审 P9）：显式报错交上层重置
        if (envelope.seq < 0 || envelope.total < 1) {
            throw ChunkReassemblyException("分帧序列非法：seq=${envelope.seq}, total=${envelope.total}")
        }

        if (envelope.seq == 0) {
            // 新序列起点：旧缓冲整体作废（容忍重连后的新序列与 total 变化）
            buffer.clear()
            receivedCount = 0
            expectedTotal = envelope.total
        }
        val total = expectedTotal ?: envelope.total.also { expectedTotal = it }

        if (envelope.total != total) {
            throw ChunkReassemblyException("分帧 total 漂移：期望 $total，收到 ${envelope.total}")
        }
        if (envelope.seq >= total) {
            throw ChunkReassemblyException("分帧 seq=${envelope.seq} 越界（total=$total）")
        }
        if (buffer.containsKey(envelope.seq)) {
            throw ChunkReassemblyException("分帧 seq=${envelope.seq} 重复到达（丢片后半截重发）")
        }

        buffer[envelope.seq] = decodeBase64(envelope.chunkBase64)
        receivedCount++

        if (receivedCount < total) return null

        // 收齐：按 seq 逐位校验并拼接（连续性断言，镜像桌面 reassemble）
        val chunks = ArrayList<ByteArray>(total)
        for (expected in 0 until total) {
            val chunk = buffer.remove(expected)
                ?: throw ChunkReassemblyException("分帧连续性断档：期望 seq=$expected，实际缺失")
            chunks.add(chunk)
        }
        // 状态复位，承接下一序列
        buffer.clear()
        receivedCount = 0
        expectedTotal = null

        var size = 0
        chunks.forEach { size += it.size }
        return ByteArray(size).also { out ->
            var offset = 0
            chunks.forEach { chunk ->
                chunk.copyInto(out, offset)
                offset += chunk.size
            }
        }
    }

    private data class Envelope(val seq: Int, val total: Int, val chunkBase64: String)

    private fun parseEnvelope(data: String): Envelope = try {
        val json = JSONObject(data)
        Envelope(
            seq = json.getInt("seq"),
            total = json.getInt("total"),
            chunkBase64 = json.getString("chunkBase64"),
        )
    } catch (e: JSONException) {
        throw ChunkReassemblyException("分帧 envelope 解析失败: ${e.message}", e)
    }

    private fun decodeBase64(b64: String): ByteArray = try {
        Base64.getDecoder().decode(b64)
    } catch (e: IllegalArgumentException) {
        throw ChunkReassemblyException("分帧 base64 解码失败: ${e.message}", e)
    }
}

/** 分帧重组失败——上层捕获并重置，等待桌面重发完整序列。 */
class ChunkReassemblyException(message: String, cause: Throwable? = null) : Exception(message, cause)
