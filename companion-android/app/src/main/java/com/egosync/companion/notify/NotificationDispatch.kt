package com.egosync.companion.notify

import com.egosync.companion.sync.NoticeItem

/**
 * 通知分发抽象（FR-42 降级记录）：
 * V1 仅应用内通知；FCM 中继代理 / UnifiedPush 作为后续可插适配器。
 *
 * 系统推送接入时新增对应 Adapter 实现本接口即可，UI 层零改动。
 */
interface NotificationDispatch {
    /** 新通知进入分发（whisper/tap/knock 三级）。 */
    fun dispatch(notice: NoticeItem)

    /** 单条标记已读。 */
    fun markRead(id: String)

    /** 全部标记已读。 */
    fun markAllRead()

    /** 敲门级通知的快捷决策（等效 App 内 ActionCard 的确认/拒绝）。 */
    fun respond(id: String, confirmed: Boolean)
}
