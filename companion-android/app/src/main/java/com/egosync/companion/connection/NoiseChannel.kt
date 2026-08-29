package com.egosync.companion.connection

import com.southernstorm.noise.protocol.CipherState
import com.southernstorm.noise.protocol.CipherStatePair
import com.southernstorm.noise.protocol.HandshakeState
import java.security.SecureRandom

/**
 * Noise XX 握手与传输期加解密封装——Kotlin 侧唯一触碰 noise-java 的位置
 * （架构硬边界 #1 加密边界）。API 用法逐字镜像
 * crates/companion-proto/interop/noise-java/GenVectors.java（实测用法，非 README 示例）。
 */
class NoiseChannel private constructor(private val handshake: HandshakeState) {

    /** split() 后的传输态：发送/接收两个方向的 ChaChaPoly 密码器。 */
    class Transport internal constructor(
        private val sender: CipherState,
        private val receiver: CipherState,
    ) {
        fun encrypt(plaintext: ByteArray): ByteArray {
            val out = ByteArray(plaintext.size + 16)
            val len = sender.encryptWithAd(null, plaintext, 0, out, 0, plaintext.size)
            return out.copyOf(len)
        }

        fun decrypt(ciphertext: ByteArray): ByteArray {
            val out = ByteArray(ciphertext.size)
            val len = receiver.decryptWithAd(null, ciphertext, 0, out, 0, ciphertext.size)
            return out.copyOf(len)
        }
    }

    companion object {
        const val SUITE = "Noise_XX_25519_ChaChaPoly_BLAKE2s"

        private const val MAX_MESSAGE_LEN = 65535

        /**
         * XX 发起方（手机侧 E2E 握手角色）。[staticPrivate] 为空时生成随机密钥。
         */
        fun initiator(staticPrivate: ByteArray? = null): NoiseChannel =
            create(HandshakeState.INITIATOR, staticPrivate)

        /** XX 响应方（中继鉴权握手中手机对 relay 的角色）。[staticPrivate] 为空时生成随机密钥。 */
        fun responder(staticPrivate: ByteArray? = null): NoiseChannel =
            create(HandshakeState.RESPONDER, staticPrivate)

        /** 仅测试：固定静态与临时密钥（黄金向量重放）；生产禁止固定临时密钥。 */
        fun initiatorForTest(staticPrivate: ByteArray, ephemeralPrivate: ByteArray): NoiseChannel {
            val hs = newHandshake(HandshakeState.INITIATOR, staticPrivate)
            hs.getFixedEphemeralKey().setPrivateKey(ephemeralPrivate, 0)
            hs.start()
            return NoiseChannel(hs)
        }

        /** 仅测试：固定静态与临时密钥的响应侧。 */
        fun responderForTest(staticPrivate: ByteArray, ephemeralPrivate: ByteArray): NoiseChannel {
            val hs = newHandshake(HandshakeState.RESPONDER, staticPrivate)
            hs.getFixedEphemeralKey().setPrivateKey(ephemeralPrivate, 0)
            hs.start()
            return NoiseChannel(hs)
        }

        private fun create(role: Int, staticPrivate: ByteArray?): NoiseChannel {
            val hs = newHandshake(role, staticPrivate ?: SecureRandom().deriveKey())
            hs.start()
            return NoiseChannel(hs)
        }

        private fun newHandshake(role: Int, staticPrivate: ByteArray): HandshakeState {
            val hs = HandshakeState(SUITE, role)
            hs.getLocalKeyPair().setPrivateKey(staticPrivate, 0)
            return hs
        }

        private fun SecureRandom.deriveKey(): ByteArray = ByteArray(32).also { nextBytes(it) }

        private fun extractPublicKey(dh: com.southernstorm.noise.protocol.DHState): ByteArray {
            val out = ByteArray(dh.publicKeyLength)
            dh.getPublicKey(out, 0)
            return out
        }

        /** 仅测试：由静态私钥派生公钥（信任锚测试的期望值，同一 noise-java 派生路径）。 */
        fun deriveStaticPublicKeyForTest(staticPrivate: ByteArray): ByteArray {
            val hs = HandshakeState(SUITE, HandshakeState.RESPONDER)
            hs.getLocalKeyPair().setPrivateKey(staticPrivate, 0)
            hs.start()
            return extractPublicKey(hs.localKeyPair)
        }
    }

    /** 发送一条握手消息（payload 可空），返回 wire 消息字节。 */
    fun writeHandshakeMessage(payload: ByteArray = ByteArray(0)): ByteArray {
        val message = ByteArray(MAX_MESSAGE_LEN)
        val len = handshake.writeMessage(message, 0, payload, 0, payload.size)
        return message.copyOf(len)
    }

    /** 读取一条握手消息，返回内嵌 payload 字节。 */
    fun readHandshakeMessage(message: ByteArray): ByteArray {
        val payload = ByteArray(MAX_MESSAGE_LEN)
        val len = handshake.readMessage(message, 0, message.size, payload, 0)
        return payload.copyOf(len)
    }

    /** 握手完成后取对端静态公钥（信任锚校验用：必须等于 QR 中桌面公钥，AC 防中间人）。 */
    fun remoteStaticPublicKey(): ByteArray = extractPublicKey(handshake.remotePublicKey)

    /** 握手完成后拆分出传输态（此后握手状态不可再用）。 */
    fun split(): Transport {
        val pair: CipherStatePair = handshake.split()
        return Transport(pair.sender, pair.receiver)
    }
}
