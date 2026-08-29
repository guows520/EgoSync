package com.egosync.companion.notify

import com.egosync.companion.sync.SnapshotParser
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 应用内通知适配器契约单测（13.2 评审 P8/P17 + 决策 A 行为锁定）：
 * - 本地已读记录在 STATE_DELTA 全量重发后仍然生效（已读不复活）；
 * - 快照清空（unpair）后通知列表一并清空，不残留旧桌面数据。
 */
@OptIn(ExperimentalCoroutinesApi::class)
class InAppNotificationAdapterTest {

    private fun storeWithNotice(): SnapshotStore = SnapshotStore().apply {
        applySnapshot(SnapshotParser.parse(notificationSnapshotJson("""[{"id":"n-1","roleId":"role-pm","level":"tap","content":"提醒","createdAt":"2026-08-29T10:30:00Z","roleName":"产品经理","roleIcon":"target","roleColor":"#6366F1"}]""")))
    }

    private fun notificationSnapshotJson(notificationsJson: String): String = """
        {"schemaVersion":1,"generatedAt":"2026-08-29T12:00:00Z","dataCutoffAt":null,
         "truncated":false,"truncatedDomains":[],
         "roles":[],"tasks":[],
         "dashboard":{"statuses":[],"metrics":{"taskCount":1,"memoryCount":2,
                       "conversationCount":3,"pendingTaskCount":0,
                       "generatedAt":"2026-08-29T12:00:00Z"}},
         "conversations":[],"briefings":[],"weeklyReviews":[],
         "notifications":$notificationsJson}
    """.trimIndent()

    /** Unconfined 作用域：StateFlow 发射同步驱动 adapter 内部 collect，时序确定。 */
    private fun adapter(store: SnapshotStore, readState: NoticeReadStateStore) =
        InAppNotificationAdapter(
            store = store,
            scope = CoroutineScope(Dispatchers.Unconfined),
            readState = readState,
        )

    @Test
    fun `STATE_DELTA全量重发不复活已读`() = runBlocking {
        // WHY（决策 A）：桌面 notifications 域只含未读、并不知道手机已读——全量
        // 重发若直接覆盖，用户读过的通知会反复弹回，直到 13.3 指令通道上线
        val store = storeWithNotice()
        val readState = InMemoryNoticeReadStateStore()
        val a = adapter(store, readState)

        a.markRead("n-1")
        assertTrue(a.notices.value.single().read)

        // 桌面 STATE_DELTA 重发同一未读通知（read 恒 false）→ 本地已读叠加生效
        store.applySnapshot(SnapshotParser.parse(notificationSnapshotJson("""[{"id":"n-1","roleId":"role-pm","level":"tap","content":"提醒","createdAt":"2026-08-29T10:30:00Z","roleName":"产品经理","roleIcon":"target","roleColor":"#6366F1"}]""")))

        assertTrue(a.notices.value.single().read)
        assertTrue("已读 ID 必须写穿到持久层（跨进程重启保留）", "n-1" in readState.readIds())
    }

    @Test
    fun `快照清空后通知列表一并清空不残留`() = runBlocking {
        // WHY（评审 P8）：unpair 后旧桌面数据不可信——collect 只在 loaded 分支
        // 刷新会让已读/未读列表残留至下一次配对
        val store = storeWithNotice()
        val a = adapter(store, InMemoryNoticeReadStateStore())
        assertEquals(1, a.notices.value.size)

        store.clear() // loaded=false → 列表清空

        assertTrue(a.notices.value.isEmpty())
    }

    @Test
    fun `clear同时清除本地已读记录`() {
        // WHY：解配后数据全部清除——已读记录若残留，重新配对后旧 ID 恰好碰撞
        // 会把新桌面的未读通知错误标记为已读
        val store = storeWithNotice()
        val readState = InMemoryNoticeReadStateStore()
        val a = adapter(store, readState)
        a.markRead("n-1")

        a.clear()

        assertTrue(readState.readIds().isEmpty())
    }
}
