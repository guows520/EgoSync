package com.egosync.companion.connection

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * 配对密钥管理的最小依赖面：连接编排层只依赖此接口（测试注入假实现即可
 * JVM 全速验证编排逻辑，P7），Keystore 真路径归 androidTest（Task 8）。
 *
 * T-S4 拆分（SPEC state-model §5.1）：取回与生成显式分流——配对路径不得
 * 静默重建身份（新身份必须显式走 createForPairing），已配对重连缺文件即为
 * 失配事实（上报恢复，不悄悄顶替）。
 */
interface SecretsProvider {
    /** 已配对设备取回既有私钥：包裹文件缺失抛 [MissingSecretsException]；密文不可解抛 [SecretsInvalidatedException]。 */
    fun loadExistingStaticPrivateKey(): ByteArray

    /** 首次配对/换绑时生成新私钥并落盘——仅未配对或 wipe 已完成后调用（生成即是重扫意图本身）。 */
    fun createForPairingStaticPrivateKey(): ByteArray

    /** 包裹私钥文件存在性（冷启动同步检测用，不触发解密）。 */
    fun hasStaticPrivateKey(): Boolean

    fun wipe()
}

/**
 * 已配对但包裹私钥文件缺失（重装/系统清理残留）——上层应清配对态回重扫
 * 路径并发布 CredentialMissing 恢复，不得静默生成新身份（T-S4，AC6 语义）。
 */
class MissingSecretsException : Exception()

/**
 * 包裹密文不可解（Keystore 密钥被系统失效 / 密文损坏）——上层收到本异常应
 * 清除配对态回到重扫路径（自愈），绝不无限重试（P14）。
 */
class SecretsInvalidatedException : Exception()

/**
 * Noise X25519 静态私钥的 Keystore 包裹管理（AC2、裁决 3/4）。
 *
 * Android Keystore 不支持 X25519 原生托管且不可导出密钥字节，而 noise-java
 * 需要私钥字节参与握手——采用 Keystore AES-GCM 密钥包裹：AES 密钥永不出
 * Keystore，X25519 私钥密文落应用私有文件，明文仅存内存。导出/日志路径上
 * 均无私钥材料（NFR-M7）。
 */
class PairingSecrets(private val context: Context) : SecretsProvider {

    /** 取回既有私钥：文件缺失/密文不可解均类型化上抛（语义见接口注释）。 */
    override fun loadExistingStaticPrivateKey(): ByteArray {
        val wrapped = wrappedFile()
        if (!wrapped.exists()) throw MissingSecretsException()
        return try {
            unwrap(wrapped.readBytes())
        } catch (_: Exception) {
            // 密文/Keystore 已失效：删除残留密文，类型化上抛驱动上层自愈——
            // 静默重试只会永久卡死连接层（P14）
            wrapped.delete()
            throw SecretsInvalidatedException()
        }
    }

    /** 生成新私钥并落盘（显式重建身份：重扫二维码/换绑路径专属）。 */
    override fun createForPairingStaticPrivateKey(): ByteArray {
        val privateKey = ByteArray(KEY_LEN).also { SecureRandom().nextBytes(it) }
        atomicWrite(wrappedFile(), wrap(privateKey))
        return privateKey
    }

    override fun hasStaticPrivateKey(): Boolean = wrappedFile().exists()

    /**
     * 清除私钥材料：删密文文件 + 删 Keystore AES 密钥。Keystore 密钥销毁后
     * 旧私钥即使文件残留也不可解密（AC6「清除密钥」的最强形态）。
     */
    override fun wipe() {
        wrappedFile().delete()
        val keystore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        keystore.deleteEntry(KEY_ALIAS)
    }

    /** 原子落盘：先写临时文件再原子重命名——崩溃不留半截密文（P14）。 */
    private fun atomicWrite(target: File, bytes: ByteArray) {
        val tmp = File(context.filesDir, "$WRAPPED_FILE.tmp")
        tmp.writeBytes(bytes)
        if (!tmp.renameTo(target)) {
            tmp.delete()
            throw java.io.IOException("配对密钥落盘失败")
        }
    }

    private fun wrappedFile(): File = File(context.filesDir, WRAPPED_FILE)

    private fun obtainKey(): SecretKey {
        val keystore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keystore.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    /** 包裹布局：IV（12 字节）‖ AES-GCM 密文（含 128 位认证标签）。 */
    private fun wrap(plaintext: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, obtainKey())
        val ciphertext = cipher.doFinal(plaintext)
        return cipher.iv + ciphertext
    }

    private fun unwrap(blob: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        val iv = blob.copyOfRange(0, GCM_IV_LEN)
        val ciphertext = blob.copyOfRange(GCM_IV_LEN, blob.size)
        cipher.init(Cipher.DECRYPT_MODE, obtainKey(), GCMParameterSpec(GCM_TAG_BITS, iv))
        return cipher.doFinal(ciphertext)
    }

    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "egosync_companion_noise_wrap"
        private const val WRAPPED_FILE = "noise_static_wrapped.bin"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_LEN = 12
        private const val GCM_TAG_BITS = 128
        private const val KEY_LEN = 32
    }
}
