package com.egosync.companion.connection

import android.util.Log

/**
 * 伴侣端统一日志口（NFR-M7）：
 * - tag 一律 `Companion/<组件>`；
 * - 永不记录帧明文、公私钥材料、QR 内容——只记事件类别/状态变更/对端标识（relay_id）。
 */
internal object CompanionLog {

    fun info(component: String, message: String) {
        Log.i("Companion/$component", message)
    }

    fun warn(component: String, message: String) {
        Log.w("Companion/$component", message)
    }
}
