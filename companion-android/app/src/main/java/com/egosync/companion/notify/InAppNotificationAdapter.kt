package com.egosync.companion.notify

import android.content.SharedPreferences
import com.egosync.companion.sync.NoticeItem
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * 本地已读记录（13.2 评审决策 A）：已读通知 ID 持久化。
 *
 * WHY：快照 notifications 域只含未读，桌面并不知道手机已读（回写要等 13.3 指令
 * 通道）；STATE_DELTA 全量重发若不叠加本地已读态，读过的通知会反复复活。
 * 13.3 上线后由桌面回写取代（届时已读以桌面为准）。
 */
interface NoticeReadStateStore {
    fun readIds(): Set<String>
    fun markRead(id: String)
    fun markAllRead(ids: Collection<String>)
    fun clear()
}

/** SharedPreferences 实现（容器装配）：已读记录跨进程重启保留。 */
class PrefsNoticeReadStateStore(private val prefs: SharedPreferences) : NoticeReadStateStore {
    override fun readIds(): Set<String> = prefs.getStringSet(KEY_READ_IDS, emptySet()).orEmpty()
    override fun markRead(id: String) {
        prefs.edit().putStringSet(KEY_READ_IDS, readIds() + id).apply()
    }
    override fun markAllRead(ids: Collection<String>) {
        prefs.edit().putStringSet(KEY_READ_IDS, readIds() + ids).apply()
    }
    override fun clear() {
        prefs.edit().remove(KEY_READ_IDS).apply()
    }
    private companion object {
        const val KEY_READ_IDS = "notice_read_ids"
    }
}

/** 内存实现（单测默认缝，JVM 无 Android 依赖）。 */
class InMemoryNoticeReadStateStore : NoticeReadStateStore {
    private val ids = mutableSetOf<String>()
    override fun readIds(): Set<String> = ids.toSet()
    override fun markRead(id: String) {
        ids.add(id)
    }
    override fun markAllRead(ids: Collection<String>) {
        this.ids.addAll(ids)
    }
    override fun clear() {
        ids.clear()
    }
}

/**
 * V1 应用内通知适配器：三级通知在 App 内的分组列表与未读管理。
 * 13.2 起初始通知集来自桌面快照 notifications 域（STATE_DELTA 即时刷新）；
 * 本地 dispatch 的运行时通知（连接层推送）前插保留。
 */
class InAppNotificationAdapter(
    private val store: SnapshotStore,
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main),
    /** 本地已读记录（评审决策 A）：默认内存实现，生产装配注入 Prefs 实现。 */
    private val readState: NoticeReadStateStore = InMemoryNoticeReadStateStore(),
) : NotificationDispatch {

    private val readIds = readState.readIds().toMutableSet()

    private val _notices = MutableStateFlow(applyReadState(store.notices))
    val notices: StateFlow<List<NoticeItem>> = _notices.asStateFlow()

    init {
        // AC2：快照全量替换即时刷新通知列表；已读态经本地记录叠加不回退（决策 A）。
        // 未加载（unpair 清空）时列表一并清空——旧桌面数据不再呈现（评审 P8）
        scope.launch {
            store.state.collect { state ->
                _notices.value = if (state.loaded) applyReadState(store.notices) else emptyList()
            }
        }
    }

    /** 桌面未读域 + 本地已读叠加：快照重发不复活已读（评审决策 A）。 */
    private fun applyReadState(items: List<NoticeItem>): List<NoticeItem> =
        items.map { if (it.id in readIds) it.copy(read = true) else it }

    override fun dispatch(notice: NoticeItem) {
        _notices.value = listOf(notice) + _notices.value
    }

    override fun markRead(id: String) {
        readIds.add(id)
        readState.markRead(id)
        _notices.value = _notices.value.map {
            if (it.id == id) it.copy(read = true) else it
        }
    }

    override fun markAllRead() {
        val ids = _notices.value.map { it.id }
        readIds.addAll(ids)
        readState.markAllRead(ids)
        _notices.value = _notices.value.map { it.copy(read = true) }
    }

    override fun respond(id: String, confirmed: Boolean) {
        readIds.add(id)
        readState.markRead(id)
        _notices.value = _notices.value.map {
            if (it.id == id) it.copy(actionState = confirmed, read = true) else it
        }
    }

    /** 解除配对配套：清空本地已读记录与通知列表（评审 P8：数据不再可信，不留残留）。 */
    fun clear() {
        readIds.clear()
        readState.clear()
        _notices.value = emptyList()
    }

    fun unreadCount(): Int = _notices.value.count { !it.read }
}
