package com.egosync.companion.sync

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import com.egosync.companion.connection.CompanionLog
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * 快照密文编解码（注入缝：JVM 单测注入假实现即可全速验证缓存文件逻辑，P7 手法；
 * Keystore 真路径归 androidTest——同 [com.egosync.companion.connection.SecretsProvider]）。
 */
interface SnapshotCipher {
    /** 加密并返回 IV‖GCM 密文。 */
    fun encrypt(plaintext: ByteArray): ByteArray

    /** 解密 IV‖GCM 密文；密钥失效/密文损坏抛异常。 */
    fun decrypt(blob: ByteArray): ByteArray
}

/** 缓存文件不可用（未知版本等显式失败）——不静默、不重试。 */
class SnapshotCacheException(message: String) : Exception(message)

/**
 * 快照缓存：单一版本化文件（`snapshot_cache.bin`），
 * 布局 = 4 字节大端版本号 + IV（12B）‖AES-GCM 密文。
 *
 * - AES 密钥由 Android Keystore 派生（照抄 `connection/KeyStore.kt` PairingSecrets
 *   先例，alias 另立）；明文快照 JSON 永不落盘。
 * - 原子写（tmp + rename）：崩溃不留半截文件。
 * - 未知版本**显式失败**（不静默）——留给未来迁移路径决策；
 *   密文/结构损坏**自愈**（删除重建空态），快照可由下一个 SNAPSHOT 重建。
 */
class SnapshotCacheFile(
    private val dir: File,
    private val cipher: SnapshotCipher,
) {

    fun exists(): Boolean = cacheFile().exists()

    /** 异步落盘入口（[SnapshotStore] 调用）：原子写版本头 + 密文。 */
    fun save(json: ByteArray) {
        val blob = byteArrayOf(
            (CACHE_VERSION ushr 24).toByte(),
            (CACHE_VERSION ushr 16).toByte(),
            (CACHE_VERSION ushr 8).toByte(),
            CACHE_VERSION.toByte(),
        ) + cipher.encrypt(json)
        atomicWrite(cacheFile(), blob)
    }

    /**
     * 读取缓存明文。
     * @return 无缓存或损坏自愈删除后返回 null；
     * @throws SnapshotCacheException 版本头未知（显式失败，调用方决定处置）。
     */
    fun load(): ByteArray? {
        val file = cacheFile()
        if (!file.exists()) return null
        val blob = file.readBytes()
        if (blob.size < VERSION_HEADER_LEN) {
            return selfHeal(file, "缓存文件过短（${blob.size} 字节）")
        }
        val version = ((blob[0].toInt() and 0xFF) shl 24) or
            ((blob[1].toInt() and 0xFF) shl 16) or
            ((blob[2].toInt() and 0xFF) shl 8) or
            (blob[3].toInt() and 0xFF)
        if (version != CACHE_VERSION) {
            throw SnapshotCacheException("快照缓存版本未知：$version（本端支持 $CACHE_VERSION）")
        }
        return try {
            cipher.decrypt(blob.copyOfRange(VERSION_HEADER_LEN, blob.size))
        } catch (e: Exception) {
            // 密钥失效/密文损坏：删残留自愈（快照由下一个 SNAPSHOT 重建，P14 同款语义）
            selfHeal(file, "缓存密文不可解（${e.javaClass.simpleName}）")
        }
    }

    /** 清空缓存（unpair 等场景）。删除失败显式记日志——数据已不可信却残留是安全语义缺口。 */
    fun clear() {
        val file = cacheFile()
        if (!file.delete() && file.exists()) {
            CompanionLog.warn("Companion/Snapshot", "快照缓存删除失败: ${file.path}")
        }
    }

    private fun selfHeal(file: File, reason: String): ByteArray? {
        file.delete()
        CompanionLog.warn("Companion/Snapshot", "快照缓存自愈删除：$reason")
        return null
    }

    /**
     * 原子落盘：先写临时文件再原子重命名（照抄 PairingSecrets.atomicWrite）。
     * 写失败/改名失败均清理 tmp 残留（13.2 评审 P5：磁盘满时 .tmp 永久残留）。
     * 非线程安全：并发调用方须自行串行化（SnapshotStore.diskMutex 单飞）。
     */
    private fun atomicWrite(target: File, bytes: ByteArray) {
        val tmp = File(dir, "$CACHE_FILE.tmp")
        try {
            tmp.writeBytes(bytes)
            if (!tmp.renameTo(target)) {
                throw java.io.IOException("快照缓存落盘失败")
            }
        } finally {
            tmp.delete() // 改名成功后已不存在，删除为无害空操作
        }
    }

    private fun cacheFile(): File = File(dir, CACHE_FILE)

    private companion object {
        /** 缓存结构版本（独立于快照 schemaVersion：此处是加密容器布局版本）。 */
        const val CACHE_VERSION = 1
        const val CACHE_FILE = "snapshot_cache.bin"
        const val VERSION_HEADER_LEN = 4
    }
}

/**
 * Keystore 派生 AES-GCM 密钥的快照密文实现（照抄 PairingSecrets：AES 密钥
 * 永不出 Keystore，密文落应用私有文件；alias 另立，与配对密钥互不影响）。
 */
class KeystoreSnapshotCipher(private val context: Context) : SnapshotCipher {

    override fun encrypt(plaintext: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, obtainKey())
        return cipher.iv + cipher.doFinal(plaintext)
    }

    override fun decrypt(blob: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        val iv = blob.copyOfRange(0, GCM_IV_LEN)
        val ciphertext = blob.copyOfRange(GCM_IV_LEN, blob.size)
        cipher.init(Cipher.DECRYPT_MODE, obtainKey(), GCMParameterSpec(GCM_TAG_BITS, iv))
        return cipher.doFinal(ciphertext)
    }

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

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val KEY_ALIAS = "egosync_companion_snapshot_wrap"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val GCM_IV_LEN = 12
        const val GCM_TAG_BITS = 128
    }
}
