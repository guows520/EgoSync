package com.egosync.companion.sync

import java.io.File
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 离线待发箱契约单测（T-S10，SPEC state-and-recovery-model §4.4 裁决 B）。
 * 注入假密文实现绕开 Android Keystore（P7 注入缝手法，同 SnapshotCacheFileTest）。锁定：
 * - FIFO 入队/删条目同步落盘，新实例（进程重建语义）从盘上原样恢复；
 * - 结构/密文损坏自愈为空队（不静默炸进程：坏文件永久卡死启动比丢几条待发更糟）；
 * - 明文永不落盘（与快照缓存同级 Keystore 加密保护）。
 */
class ChatOutboxTest {

    private lateinit var dir: File

    @Before
    fun setUp() {
        dir = createTempDirectory("chat-outbox-test")
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
    }

    private fun entry(commandId: String) = OutboxEntry(
        commandId = commandId,
        roleId = null,
        conversationId = null,
        content = "离线内容-$commandId",
        localMessageId = "u-$commandId",
    )

    @Test
    fun `入队FIFO删条目并跨实例恢复`() {
        // WHY（SPEC 裁决 B）：进程被杀/冷启动不丢队列——入队即落盘，
        // 新实例（进程重建语义）必须原样还原（含本地占位会话条目 conversationId=null）。
        val persistence = ChatOutboxFile(dir, AesCipher())
        val outbox = ChatOutbox(persistence)

        outbox.enqueue(entry("c-1"))
        outbox.enqueue(entry("c-2"))
        assertEquals(listOf("c-1", "c-2"), outbox.entries.value.map { it.commandId })

        val restored = ChatOutbox(persistence)
        assertEquals(outbox.entries.value, restored.entries.value)

        // 删条目同步落盘：成功发送后进程再死，条目不得复活
        restored.remove("c-1")
        assertEquals(listOf("c-2"), ChatOutbox(persistence).entries.value.map { it.commandId })
    }

    @Test
    fun `文件损坏自愈为空队不抛错`() {
        // WHY：密文/结构损坏时炸启动 = 永久卡死（用户无法进入应用）；自愈丢
        // 几条待发（用户可重发）是更小的失败面——与快照缓存 P14 自愈同款权衡。
        val persistence = ChatOutboxFile(dir, AesCipher())
        ChatOutbox(persistence).enqueue(entry("c-1"))

        File(dir, "chat_outbox.bin").writeBytes(byteArrayOf(9, 9, 9, 9, 1, 2, 3))
        assertTrue(ChatOutbox(persistence).entries.value.isEmpty())

        // 半截文件（长度不足版本头）同样自愈
        ChatOutbox(persistence).enqueue(entry("c-2"))
        File(dir, "chat_outbox.bin").writeBytes(byteArrayOf(1, 2))
        assertTrue(ChatOutbox(persistence).entries.value.isEmpty())
    }

    @Test
    fun `明文内容与commandId不落盘`() {
        // WHY：filesDir 落盘内容必须全程密文——明文对话残留在私有目录是
        // 安全语义缺口（与快照缓存同级保护，SPEC 裁决 B）。
        val persistence = ChatOutboxFile(dir, AesCipher())
        ChatOutbox(persistence).enqueue(
            OutboxEntry("cmd-秘密", roleId = null, conversationId = null, content = "离线内容", localMessageId = "u-9"),
        )

        val raw = String(File(dir, "chat_outbox.bin").readBytes(), Charsets.UTF_8)
        assertFalse("内容不得明文出现", raw.contains("离线内容"))
        assertFalse("commandId 不得明文出现", raw.contains("cmd-秘密"))
    }

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
