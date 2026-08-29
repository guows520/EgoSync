package com.egosync.companion.connection

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test
import java.util.HexFormat

/**
 * AC2 意图锚点：「跨语言互通不是纸面承诺」。
 *
 * 以 12.1 冻结的黄金向量（Noise_XX_25519_ChaChaPoly_BLAKE2s，noise-java ↔ snow
 * 行协议真实握手产物，crates/companion-proto/tests/fixtures/noise_java_vectors.json）
 * 用固定密钥重放 XX 握手与传输期加解密，断言**逐字节**一致——
 * 任何一侧实现漂移（密钥派生、nonce 顺序、消息封装）立即在此暴露。
 */
class NoiseInteropTest {

    private val hex = HexFormat.of()

    private val vectors: org.json.JSONObject by lazy {
        val stream = javaClass.classLoader!!.getResourceAsStream("noise_java_vectors.json")
        org.json.JSONObject(stream!!.readBytes().decodeToString())
    }

    /** 发起侧：固定静态 + 固定临时密钥重放 XX 握手，3 条消息与全部传输密文逐字节等于冻结向量。 */
    @Test
    fun initiator_replays_golden_vectors_byte_for_byte() {
        // WHY: 手机侧永远是 E2E 握手的 initiator（直连与中继同构）——重放路径
        // 与真实运行路径一致，向量失配即无法与桌面 snow 互通。
        val initiator = vectors.getJSONObject("initiator")
        val messages = vectors.getJSONArray("handshakeMessages")
        val channel = NoiseChannel.initiatorForTest(
            staticPrivate = hex.parseHex(initiator.getString("staticPrivate")),
            ephemeralPrivate = hex.parseHex(initiator.getString("ephemeralPrivate")),
        )

        val m1 = channel.writeHandshakeMessage()
        assertEquals("m1 与冻结向量不一致", messages.getString(0), hex.formatHex(m1))

        channel.readHandshakeMessage(hex.parseHex(messages.getString(1)))

        val m3 = channel.writeHandshakeMessage()
        assertEquals("m3 与冻结向量不一致", messages.getString(2), hex.formatHex(m3))

        val transport = channel.split()
        val entries = vectors.getJSONArray("transport")
        for (i in 0 until entries.length()) {
            val entry = entries.getJSONObject(i)
            val plaintext = hex.parseHex(entry.getString("plaintext"))
            val ciphertext = hex.parseHex(entry.getString("ciphertext"))
            if (entry.getString("direction") == "i2r") {
                assertEquals("i2r 密文 #$i 与冻结向量不一致", entry.getString("ciphertext"), hex.formatHex(transport.encrypt(plaintext)))
            } else {
                assertArrayEquals("r2i 密文 #$i 解密与冻结向量不一致", plaintext, transport.decrypt(ciphertext))
            }
        }
    }

    /** 响应侧：中继鉴权握手中手机作 responder（relay 为 initiator），同样受向量约束。 */
    @Test
    fun responder_replays_golden_vectors_byte_for_byte() {
        val responder = vectors.getJSONObject("responder")
        val messages = vectors.getJSONArray("handshakeMessages")
        val channel = NoiseChannel.responderForTest(
            staticPrivate = hex.parseHex(responder.getString("staticPrivate")),
            ephemeralPrivate = hex.parseHex(responder.getString("ephemeralPrivate")),
        )

        channel.readHandshakeMessage(hex.parseHex(messages.getString(0)))

        val m2 = channel.writeHandshakeMessage()
        assertEquals("m2 与冻结向量不一致", messages.getString(1), hex.formatHex(m2))

        channel.readHandshakeMessage(hex.parseHex(messages.getString(2)))

        val transport = channel.split()
        // 方向语义以 initiator 为视角：i2r 由 responder 的 receiver 解密，
        // r2i 由 responder 的 sender 加密——nonce 顺序与向量生成时一致（先 i2r 块后 r2i 块）。
        val entries = vectors.getJSONArray("transport")
        for (i in 0 until entries.length()) {
            val entry = entries.getJSONObject(i)
            val plaintext = hex.parseHex(entry.getString("plaintext"))
            val ciphertext = hex.parseHex(entry.getString("ciphertext"))
            if (entry.getString("direction") == "i2r") {
                assertArrayEquals("i2r 密文 #$i 解密与冻结向量不一致", plaintext, transport.decrypt(ciphertext))
            } else {
                assertEquals("r2i 密文 #$i 与冻结向量不一致", entry.getString("ciphertext"), hex.formatHex(transport.encrypt(plaintext)))
            }
        }
    }

    /** 信任锚提取（Task 4 防中间人）：initiator 必须能取到 responder 静态公钥本体。 */
    @Test
    fun initiator_extracts_responder_static_public_key() {
        // WHY: 「信任锚不可绕过」的底层原语——手机完成握手后取到的远端静态
        // 公钥必须与 QR 中桌面公钥逐字节可比对；取不到或取错即信任锚失效。
        val init = vectors.getJSONObject("initiator")
        val resp = vectors.getJSONObject("responder")
        val messages = vectors.getJSONArray("handshakeMessages")
        val channel = NoiseChannel.initiatorForTest(
            staticPrivate = hex.parseHex(init.getString("staticPrivate")),
            ephemeralPrivate = hex.parseHex(init.getString("ephemeralPrivate")),
        )
        channel.writeHandshakeMessage()
        channel.readHandshakeMessage(hex.parseHex(messages.getString(1)))
        channel.writeHandshakeMessage()

        val actual = channel.remoteStaticPublicKey()
        // 期望值：由 fixture 响应侧静态私钥派生的公钥（同一 noise-java 派生路径）
        val expected = NoiseChannel.deriveStaticPublicKeyForTest(hex.parseHex(resp.getString("staticPrivate")))
        assertEquals("远端静态公钥提取应与响应侧私钥派生一致", HexFormat.of().formatHex(expected), HexFormat.of().formatHex(actual))
    }
}
