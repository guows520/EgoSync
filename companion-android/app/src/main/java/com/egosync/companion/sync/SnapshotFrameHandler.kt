package com.egosync.companion.sync

import com.egosync.companion.connection.CompanionLog
import com.egosync.companion.connection.Frame
import com.egosync.companion.connection.FrameConsumer

/**
 * 快照帧处理器（13.2 T3 消费端）：SNAPSHOT/STATE_DELTA 帧 → 分帧重组 →
 * 解析 → [onSnapshot]（接入 SnapshotStore.applySnapshot 全量替换）。
 *
 * - 每会话一个实例（RealConnectionClient.startSessionLoop 经工厂创建）——
 *   残缺分帧序列不跨会话；
 * - 重组/解析失败记日志并**重置重组器**（载荷级错误不杀会话，下一个全量
 *   序列自愈）；PING 保活帧与其他帧类型静默忽略；
 * - NFR-M7：只记事件类别，帧明文与快照内容不入日志。
 */
class SnapshotFrameHandler(
    private val onSnapshot: (snapshot: DesktopSnapshot, rawJson: ByteArray) -> Unit,
) : FrameConsumer {

    private var reassembler = ChunkReassembler()

    override fun onFrame(frame: Frame) {
        val data = when (frame) {
            is Frame.Snapshot -> frame.data
            is Frame.StateDelta -> frame.data
            else -> return // PING 保活/NOTICE 等：非快照通道帧
        }
        val rawJson = try {
            reassembler.offer(data)
        } catch (e: ChunkReassemblyException) {
            // 残缺/损坏序列：重置缓冲等待桌面重发完整序列（不杀会话）
            CompanionLog.warn("Companion/Snapshot", "分帧重组失败，已重置缓冲: ${e.message}")
            reassembler = ChunkReassembler()
            return
        } ?: return // 未收齐：继续等待后续分片

        val snapshot = try {
            // throwOnInvalidSequence=false：非法 UTF-8 替换为 U+FFFD 后必然 JSON
            // 解析失败——走下方显式丢弃路径，而非 CharacterCodingException 冒出
            // 本层杀死整条会话（13.2 评审 P3，与本类"载荷级错误不杀会话"契约一致）
            SnapshotParser.parse(rawJson.decodeToString(throwOnInvalidSequence = false))
        } catch (e: SnapshotFormatException) {
            // 收齐但解析失败：内容不可信，丢弃等待下一个全量序列
            CompanionLog.warn("Companion/Snapshot", "快照解析失败，已丢弃: ${e.message}")
            return
        }
        onSnapshot(snapshot, rawJson)
    }
}
