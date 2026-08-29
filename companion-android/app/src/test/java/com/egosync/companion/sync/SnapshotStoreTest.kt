package com.egosync.companion.sync

import java.io.File
import kotlin.coroutines.EmptyCoroutineContext
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 帧驱动快照存储的时序契约单测（13.2 评审 P1/P4/P13 修复的行为锁定）：
 * - 陈旧缓存不得覆盖先到的在线快照（启动加载协程迟到竞态）；
 * - clear 封存后迟到帧/迟到缓存不得复活已清数据，resume（新会话）解封；
 * - offlineInfo 的 dataAsOf 走用户可读格式化（不再直出原始 ISO）。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class SnapshotStoreTest {

    private lateinit var dir: File

    @Before
    fun setUp() {
        dir = createTempDirectory("snap-store-test")
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
    }

    /** 异或假密文（JVM 可逆，绕开 Android Keystore——P7 注入缝手法）。 */
    private class XorCipher : SnapshotCipher {
        override fun encrypt(plaintext: ByteArray) =
            plaintext.map { (it.toInt() xor 0x5A).toByte() }.toByteArray()

        override fun decrypt(blob: ByteArray) =
            blob.map { (it.toInt() xor 0x5A).toByte() }.toByteArray()
    }

    private fun cacheFile(): SnapshotCacheFile = SnapshotCacheFile(dir, XorCipher())

    /** 最小合法快照 JSON（generatedAt 可区分新旧）。 */
    private fun snapshotJson(generatedAt: String): String = """
        {"schemaVersion":1,"generatedAt":"$generatedAt","dataCutoffAt":null,
         "truncated":false,"truncatedDomains":[],
         "roles":[],"tasks":[],
         "dashboard":{"statuses":[],"metrics":{"taskCount":1,"memoryCount":2,
                       "conversationCount":3,"pendingTaskCount":0,
                       "generatedAt":"$generatedAt"}},
         "conversations":[],"briefings":[],"weeklyReviews":[],"notifications":[]}
    """.trimIndent()

    private fun store() = SnapshotStore()

    @Test
    fun `在线快照先到时陈旧缓存不得覆盖`() = runTest {
        // WHY（评审 P1）：启动读缓存是异步协程——若连接极快、首个 SNAPSHOT 先于
        // 缓存 IO 完成，无条件 applySnapshot 会用陈旧缓存覆盖在线新快照，UI 静默
        // 回退旧数据且无任何信号。缓存只是「首个在线快照之前」的降级呈现。
        val cache = cacheFile()
        cache.save(snapshotJson("2026-08-01T08:00:00Z").toByteArray()) // 陈旧缓存

        val store = SnapshotStore(
            cache = cache,
            scope = this,
            ioDispatcher = EmptyCoroutineContext, // 加载协程进测试调度器，时序确定
        )
        // 缓存协程尚未执行：在线快照先到
        store.applySnapshot(SnapshotParser.parse(snapshotJson("2026-08-29T12:00:00Z")))
        advanceUntilIdle() // 此刻迟到的缓存加载才运行

        assertEquals("2026-08-29T12:00:00Z", store.state.value.snapshot?.generatedAt)
    }

    @Test
    fun `无在线快照时缓存正常加载为离线呈现`() = runTest {
        // WHY（AC3）：缓存加载是离线只读呈现的正路——守卫只拦「缓存迟到」，
        // 不得误伤冷启动离线场景
        val cache = cacheFile()
        cache.save(snapshotJson("2026-08-01T08:00:00Z").toByteArray())

        val store = SnapshotStore(
            cache = cache,
            scope = this,
            ioDispatcher = EmptyCoroutineContext,
        )
        advanceUntilIdle()

        assertEquals("2026-08-01T08:00:00Z", store.state.value.snapshot?.generatedAt)
    }

    @Test
    fun `clear封存后迟到快照被丢弃_resume解封恢复接收`() {
        // WHY（评审 P4）：unpair 后会话终止与在飞帧之间存在窗口——若迟到帧把
        // 已清空的 store 重新填上，「解除配对后数据不再可信」就是空话
        val s = store()
        s.applySnapshot(SnapshotParser.parse(snapshotJson("2026-08-29T12:00:00Z")))
        assertTrue(s.state.value.loaded)

        s.clear()
        assertFalse(s.state.value.loaded)

        // 迟到帧：封存期内拒绝
        s.applySnapshot(SnapshotParser.parse(snapshotJson("2026-08-29T13:00:00Z")))
        assertFalse(s.state.value.loaded)

        // 重新配对 → 新会话建立 → resume 解封
        s.resume()
        s.applySnapshot(SnapshotParser.parse(snapshotJson("2026-08-29T13:00:00Z")))
        assertEquals("2026-08-29T13:00:00Z", s.state.value.snapshot?.generatedAt)
    }

    @Test
    fun `offlineInfo格式化数据截止时间不直出原始ISO`() {
        // WHY（评审 P13/AC4）：降级横幅把 "2026-08-01" 原样塞给用户是可用性缺陷
        val s = store()
        s.applySnapshot(
            SnapshotParser.parse(
                snapshotJson("2026-08-29T12:00:00Z")
                    .replace("\"dataCutoffAt\":null", "\"dataCutoffAt\":\"2026-08-01\""),
            ),
        )
        // 纯日期截止时间（无时分分支）与时区无关，可精确断言
        assertEquals("8月1日", s.offlineInfo().dataAsOf)
        assertTrue(s.offlineInfo().snapshotAvailable)
    }

    @Test
    fun `无快照时offlineInfo为不可用`() {
        val s = store()
        assertFalse(s.offlineInfo().snapshotAvailable)
        assertNull(s.offlineInfo().dataAsOf)
    }
}

private fun createTempDirectory(prefix: String): File {
    val f = File.createTempFile(prefix, null)
    f.delete()
    f.mkdirs()
    return f
}
