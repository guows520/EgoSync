package com.egosync.companion.notify

import com.egosync.companion.sync.NoticeItem
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * V1 应用内通知适配器：三级通知在 App 内的分组列表与未读管理。
 * 原型阶段由 SnapshotStore 的 mock 通知充当初始未读集。
 */
class InAppNotificationAdapter : NotificationDispatch {

    private val _notices = MutableStateFlow(SnapshotStore.notices)
    val notices: StateFlow<List<NoticeItem>> = _notices.asStateFlow()

    override fun dispatch(notice: NoticeItem) {
        _notices.value = listOf(notice) + _notices.value
    }

    override fun markRead(id: String) {
        _notices.value = _notices.value.map {
            if (it.id == id) it.copy(read = true) else it
        }
    }

    override fun markAllRead() {
        _notices.value = _notices.value.map { it.copy(read = true) }
    }

    override fun respond(id: String, confirmed: Boolean) {
        _notices.value = _notices.value.map {
            if (it.id == id) it.copy(actionState = confirmed, read = true) else it
        }
    }

    fun unreadCount(): Int = _notices.value.count { !it.read }
}
