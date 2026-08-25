package com.egosync.companion.sync

import java.util.UUID
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 速记队列（FR-43）：降级态下的本地 FIFO。
 * 每条带幂等 ID；恢复连接后自动提交管家处理，确认后删队，无丢失。
 *
 * 接入真实连接层后：flush() 改为逐条以 COMMAND(幂等 ID) 发往桌面、
 * 收到确认后删队；本原型以内存队列模拟该行为。
 */
class QuickNoteQueue {

    data class QuickNote(
        val id: String,
        val text: String,
        val submitted: Boolean = false,
    )

    private val _items = MutableStateFlow<List<QuickNote>>(emptyList())
    val items: StateFlow<List<QuickNote>> = _items.asStateFlow()

    /** 待同步条数（含已提交未清空的过渡态条目）。 */
    val count: Int get() = _items.value.size

    fun pendingCount(): Int = _items.value.count { !it.submitted }

    /** 降级态提交一条速记（幂等 ID = UUID v4）。 */
    fun submit(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        _items.value = _items.value + QuickNote(id = UUID.randomUUID().toString(), text = trimmed)
    }

    /**
     * 恢复连接后自动提交：标记全部已提交并清空队列。
     * 幂等：队列为空时调用无副作用。
     */
    fun flush() {
        _items.value = emptyList()
    }
}
