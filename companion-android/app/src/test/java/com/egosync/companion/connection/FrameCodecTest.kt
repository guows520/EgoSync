package com.egosync.companion.connection

import org.json.JSONObject
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import java.util.HexFormat

/**
 * 帧编解码与桌面 frames.rs 逐字镜像的契约验证。
 *
 * 意图锚点：「中继零知识」——Kotlin 编码的帧 JSON 必须与 12.1 冻结的 8 条明文
 * 黄金串**逐字节**一致（serde 产物），任何键序/转义漂移都会让中继之外的第三方
 * （以及桌面 serde 侧）解析失败。
 */
class FrameCodecTest {

    private val hex = HexFormat.of()

    private val vectors: JSONObject by lazy {
        val stream = javaClass.classLoader!!.getResourceAsStream("noise_java_vectors.json")
        JSONObject(stream!!.readBytes().decodeToString())
    }

    /** fixture 中 8 条 i2r 明文黄金串（与 GenVectors.java PLAINTEXTS 同源）。 */
    private val goldenPlaintexts: List<Pair<Frame, ByteArray>> by lazy {
        val entries = vectors.getJSONArray("transport")
        val frames = listOf(
            Frame.Hello(protocolVersion = 1),
            Frame.Snapshot(data = "snapshot-全量快照"),
            Frame.StateDelta(data = "delta-增量-1"),
            Frame.Command(data = "cmd-1"),
            Frame.CommandResult(data = "result-ok"),
            Frame.StreamToken(data = "token-1"),
            Frame.Notice(data = "notice-1"),
            Frame.Ping,
        )
        (0 until 8).map { i ->
            frames[i] to hex.parseHex(entries.getJSONObject(i).getString("plaintext"))
        }
    }

    /** 进程内 XX 握手（随机密钥），返回（发起方、响应方）传输通道。 */
    private fun transportPair(): Pair<NoiseChannel.Transport, NoiseChannel.Transport> {
        val initiator = NoiseChannel.initiator()
        val responder = NoiseChannel.responder()
        responder.readHandshakeMessage(initiator.writeHandshakeMessage())
        initiator.readHandshakeMessage(responder.writeHandshakeMessage())
        responder.readHandshakeMessage(initiator.writeHandshakeMessage())
        return initiator.split() to responder.split()
    }

    @Test
    fun encoded_frames_match_golden_plaintexts_byte_for_byte() {
        // WHY: 帧 JSON 是三端协议事实源的 wire 形态——键序漂移（org.json 无序键）
        // 或转义差异在功能测试中不可见，但对字节级协议契约是破坏性的。
        val (ti, tr) = transportPair()
        for ((frame, golden) in goldenPlaintexts) {
            val wire = FrameCodec.encode(frame, ti)
            val plaintext = tr.decrypt(wire.copyOfRange(4, wire.size))
            assertArrayEquals("帧 ${frame.javaClass.simpleName} 编码与黄金串不一致", golden, plaintext)
        }
    }

    @Test
    fun decode_roundtrip_all_eight_frame_types() {
        // WHY: 编解码往返无损是三端数据一致性的地基——任何一帧往返失真
        // 意味着协议数据在传输中被静默损坏。
        val (ti, tr) = transportPair()
        for ((frame, _) in goldenPlaintexts) {
            val wire = FrameCodec.encode(frame, ti)
            assertEquals("帧 ${frame.javaClass.simpleName} 往返不一致", frame, FrameCodec.decode(wire, tr))
        }
    }

    @Test
    fun hello_missing_protocol_version_is_rejected() {
        // WHY: protocolVersion 是版本协商安全性的唯一依据——缺失该字段的
        // HELLO 被接受等于允许未协商版本的连接混入，破坏 schema 冻结契约。
        val (ti, tr) = transportPair()
        val ciphertext = ti.encrypt("""{"type":"hello"}""".toByteArray())
        val wire = byteArrayOf(0, 0, 0, ciphertext.size.toByte()) + ciphertext
        assertThrows(FrameCodec.FrameCodecException::class.java) { FrameCodec.decode(wire, tr) }
    }

    @Test
    fun hello_wrong_protocol_version_is_rejected() {
        // WHY: 「不匹配即拒绝」——放行任意版本号会让未协商版本静默混入。
        val (ti, tr) = transportPair()
        val ciphertext = ti.encrypt("""{"type":"hello","protocolVersion":999}""".toByteArray())
        val wire = byteArrayOf(0, 0, 0, ciphertext.size.toByte()) + ciphertext
        assertThrows(FrameCodec.FrameCodecException::class.java) { FrameCodec.decode(wire, tr) }
    }

    @Test
    fun length_prefix_mismatch_is_rejected() {
        // WHY: 长度前缀是流式分帧的唯一依据——前缀与实际字节数不符不报错，
        // 后续所有帧会静默错位。
        val (ti, tr) = transportPair()
        val wire = FrameCodec.encode(Frame.Ping, ti)
        val corrupted = wire.copyOf().also { it[3] = (it[3].toInt() xor 0x01).toByte() }
        assertThrows(FrameCodec.FrameCodecException::class.java) { FrameCodec.decode(corrupted, tr) }
    }

    @Test
    fun oversized_length_prefix_is_rejected_before_decryption() {
        // WHY: 单帧密文上限镜像 MAX_CIPHERTEXT_LEN——超限声明必须在解密前
        // 拒绝，防止恶意长度前缀触发 oversized 缓冲分配。
        val (ti, tr) = transportPair()
        val ciphertext = ti.encrypt("""{"type":"ping"}""".toByteArray())
        val oversized = FrameCodec.MAX_CIPHERTEXT_LEN + 1
        val wire = byteArrayOf(
            (oversized ushr 24).toByte(), (oversized ushr 16).toByte(), (oversized ushr 8).toByte(), oversized.toByte(),
        ) + ciphertext
        assertThrows(FrameCodec.FrameCodecException::class.java) { FrameCodec.decode(wire, tr) }
    }
}
