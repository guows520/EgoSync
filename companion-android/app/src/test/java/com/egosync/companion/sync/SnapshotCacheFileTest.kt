package com.egosync.companion.sync

import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * 快照缓存契约单测（注入假密文实现，绕开 Android Keystore——P7 注入缝手法，
 * 同 SecretsProviderTest）。锁定：
 * - 往返一致（密文落盘 → 读回明文一致）；
 * - 未知版本号显式失败（不静默，留迁移决策给上层）；
 * - 密文/结构损坏自愈删除（快照可由下一 SNAPSHOT 重建，P14 同款语义）；
 * - clear 后无残留。
 */
class SnapshotCacheFileTest {

    private lateinit var dir: File

    @Before
    fun setUp() {
        dir = createTempDirectory("snap-cache-test")
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
    }

    private fun cache(cipher: SnapshotCipher = AesCipher()) =
        SnapshotCacheFile(dir, cipher)

    @Test
    fun `往返一致_落盘密文读回明文相等`() {
        // WHY：缓存核心契约——解密读回必须与写入明文逐字节一致，否则离线启动
        // 把损坏快照喂给解析器，UI 白屏且无任何提示
        val c = cache()
        val plain = """{"schemaVersion":1}""".toByteArray()
        c.save(plain)
        assertArrayEquals(plain, c.load())
    }

    @Test
    fun `无缓存返回null不抛错`() {
        // 冷启动常态：无缓存文件，返回 null 让上层走「无快照」分支
        assertNull(cache().load())
        assertFalse(cache().exists())
    }

    @Test
    fun `未知版本号显式失败_不静默`() {
        // WHY：未来 schemaVersion 升级后旧端读新缓存——静默当作损坏删掉会掩盖
        // 版本漂移这一真实问题；显式抛出把迁移决策留给上层（不偷偷删用户数据）
        val c = cache()
        // 直接写一个版本号=999 的容器（不走 save 的版本头写入路径）
        File(dir, "snapshot_cache.bin").writeBytes(
            byteArrayOf(0, 0, 3, 0xE7.toByte()) + ByteArray(28) // version=999 + 伪密文
        )
        assertThrows(SnapshotCacheException::class.java) { c.load() }
    }

    @Test
    fun `密文损坏自愈删除_返回null`() {
        // WHY：密钥失效/磁盘损坏导致密文不可解——快照不是「敏感单一源」，
        // 自愈删掉让下一 SNAPSHOT 重建即可（P14 PairingSecrets 同款语义）；
        // 若抛错冒泡会让降级态崩溃，离线连不上桌面时白屏
        val c = cache()
        c.save("hello".toByteArray())
        // 篡改密文区（保留版本头，破坏后续字节）
        val file = File(dir, "snapshot_cache.bin")
        val bytes = file.readBytes()
        bytes[bytes.lastIndex] = (bytes[bytes.lastIndex].toInt() xor 0xFF).toByte()
        file.writeBytes(bytes)
        assertNull(c.load())
        assertFalse(file.exists()) // 自愈已删除
    }

    @Test
    fun `文件过短自愈删除_返回null`() {
        // WHY：写到一半崩溃留下 < 版本头长度的残文件——读不得，删之自愈
        val c = cache()
        File(dir, "snapshot_cache.bin").writeBytes(byteArrayOf(0, 0))
        assertNull(c.load())
        assertFalse(File(dir, "snapshot_cache.bin").exists())
    }

    @Test
    fun `clear后无残留`() {
        val c = cache()
        c.save("data".toByteArray())
        assertTrue(c.exists())
        c.clear()
        assertFalse(c.exists())
        assertNull(c.load())
    }

    @Test
    fun `save为原子写_不残留tmp文件`() {
        // WHY：崩溃半截写会让下次 load 读到损坏；tmp+rename 后要么完整要么旧版，
        // 且不留 .tmp 污染目录
        cache().save("data".toByteArray())
        assertFalse(File(dir, "snapshot_cache.bin.tmp").exists())
    }

    /** JVM 可跑的 AES-GCM 假密文（真 Keystore 路径归 androidTest）。 */
    private class AesCipher : SnapshotCipher {
        private val key: SecretKey =
            KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

        override fun encrypt(plaintext: ByteArray): ByteArray {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            return cipher.iv + cipher.doFinal(plaintext)
        }

        override fun decrypt(blob: ByteArray): ByteArray {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, blob, 0, 12))
            return cipher.doFinal(blob, 12, blob.size - 12)
        }
    }
}

private fun createTempDirectory(prefix: String): File {
    val f = File.createTempFile(prefix, null)
    f.delete()
    f.mkdirs()
    return f
}
