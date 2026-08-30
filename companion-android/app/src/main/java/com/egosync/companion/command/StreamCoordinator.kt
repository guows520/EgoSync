package com.egosync.companion.command

import com.egosync.companion.connection.CompanionLog
import com.egosync.companion.sync.DesktopSnapshot
import com.egosync.companion.sync.ExecutionTraceBlock
import com.egosync.companion.sync.ToolStatus
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * 桌面 llm:stream 聚合（Story 13.3 T5）：按当前查看会话追踪活跃流，驱动
 * ChatViewModel 渲染；同时作为**快照延后门**——流式活跃期间 SNAPSHOT/STATE_DELTA
 * 暂存（只留最新），流结束/失败/超时立即补应用。
 *
 * 裁决（Dev Notes §4）：`chat_send_message` 写入触发 2s debounce 重建，流式中段
 * STATE_DELTA 必然到达；ChatViewModel 既有 `onSnapshotReplaced → stopStreaming()`
 * 守卫会掐掉正在流式的气泡——全局暂存是最小正确解；任务/仪表盘秒级延迟由
 * 乐观更新掩盖。
 */
class StreamCoordinator(
    private val applySnapshot: (DesktopSnapshot, ByteArray?) -> Unit,
) {
    /** 当前活跃流（一次一个——桌面 busy 守卫串行化）；null=无活跃流。 */
    private val _state = MutableStateFlow<DesktopStream?>(null)
    val state: StateFlow<DesktopStream?> = _state.asStateFlow()

    /** 当前查看会话（ChatViewModel 经 onViewedConversation 更新）。 */
    @Volatile
    private var viewedConversationId: String? = null

    /** 流式活跃期间暂存的最晚快照（只留最新）。跨线程访问——所有读写经 [stashLock]。 */
    private var stashed: Pair<DesktopSnapshot, ByteArray?>? = null

    /** stashed 与 viewed 切换互斥（帧分发线程 vs VM 线程）。 */
    private val stashLock = Any()

    /** 一次流式会话的聚合状态。 */
    data class DesktopStream(
        val conversationId: String,
        /** 已完成段 + 活跃段（活跃段为末项）。 */
        val segments: List<StreamSegment>,
        val thinking: Boolean,
        val toolTitle: String?,
        val traceBlocks: List<ExecutionTraceBlock>,
        val done: Boolean,
    ) {
        val activeSegment: StreamSegment?
            get() = segments.lastOrNull { !it.sealed }
    }

    /** 单段（桌面 messageId 为 key；多段委派/续写）。 */
    data class StreamSegment(
        val messageId: String?,
        val text: String,
        /** 已收口（下一段开始时上一段 sealed）。 */
        val sealed: Boolean,
    )

    fun onViewedConversation(conversationId: String?) {
        val changed = viewedConversationId != conversationId
        viewedConversationId = conversationId
        if (changed) {
            // 整改：切走查看会话即 flush 暂存——否则后续快照因流不属新查看
            // 会话而直通应用，旧暂存滞留到流结束才回放覆盖更新的已应用
            // 快照（UI 状态回退）。flush 后快照门对流会话失效，后续直通。
            flushStash()
        }
    }

    /** chat.send ack 后置活跃标志（首 token 前的思考态/工具态也计入活跃）。 */
    fun streamStarting(conversationId: String) {
        if (_state.value?.takeIf { !it.done && it.conversationId == conversationId } != null) return
        // 整改：跨会话替换活跃流前先 flush 暂存（与 onStreamToken 替换路径
        // 一致）——旧流的暂存快照不得记在新流账上延迟应用。
        if (_state.value?.takeIf { !it.done && it.conversationId != conversationId } != null) {
            flushStash()
        }
        _state.value = DesktopStream(
            conversationId = conversationId,
            segments = emptyList(),
            thinking = true,
            toolTitle = null,
            traceBlocks = emptyList(),
            done = false,
        )
    }

    /** 帧循环收到 STREAM_TOKEN：按事件折叠状态机。 */
    fun onStreamToken(data: String) {
        val event = StreamEvent.parse(data) ?: run {
            CompanionLog.warn(TAG, "STREAM_TOKEN 解析失败，跳过")
            return
        }
        val cur = _state.value
        if (cur == null || cur.done) {
            // 桌面发起的流（用户未发指令）——新建活跃流（仅渲染查看中的会话）。
            // 整改：归属键一律用事件自身 conversationId——沿用 cur?.conversationId
            // 会把新流挂到已 done 的旧会话上（快照门错开、渲染错挂会话）。
            _state.value = foldNew(event, event.conversationId)
            return
        }
        // 不同会话的流到达（防御：替换——桌面串行不会并发，但跨会话切换残留需重置）
        if (event.conversationId != cur.conversationId) {
            flushStash()
            _state.value = foldNew(event, event.conversationId)
            return
        }
        _state.value = foldInto(cur, event)
    }

    /** 流主动结束（停止/失败/看门狗）：收口并 flush 暂存快照。 */
    fun streamEnded(conversationId: String) {
        val cur = _state.value
        if (cur != null && cur.conversationId == conversationId && !cur.done) {
            _state.value = cur.copy(done = true, thinking = false, toolTitle = null)
        }
        // 整改：仅当无活跃流时 flush——迟到/无关会话的 streamEnded 不得击穿
        // 快照门（否则暂存快照被立即应用，onSnapshotReplaced 掐掉另一条正
        // 在流式的气泡，恰是该门存在的全部意义）。
        val after = _state.value
        if (after == null || after.done) {
            flushStash()
        }
    }

    /** 会话彻底不可用：清空流态 + flush（VM 据 null 态感知断连）。 */
    fun reset() {
        _state.value = null
        flushStash()
    }

    /** 当前查看会话是否有活跃流（快照门判断）。 */
    private fun activeForViewed(): Boolean {
        val cur = _state.value ?: return false
        return !cur.done && cur.conversationId == viewedConversationId
    }

    /** 快照送达：活跃流期间暂存（只留最新），否则立即应用。 */
    fun deliverSnapshot(snapshot: DesktopSnapshot, rawJson: ByteArray?) {
        val stashedNow = synchronized(stashLock) {
            if (activeForViewed()) {
                stashed = snapshot to rawJson
                true
            } else {
                false
            }
        }
        // 锁外应用（applySnapshot 可能重——不得持锁回调）
        if (!stashedNow) {
            applySnapshot(snapshot, rawJson)
        }
    }

    private fun flushStash() {
        val toApply = synchronized(stashLock) {
            val s = stashed
            stashed = null
            s
        }
        toApply?.let { (snapshot, raw) -> applySnapshot(snapshot, raw) }
    }

    private fun foldNew(event: StreamEvent, conversationId: String): DesktopStream {
        val trace = event.processEvent?.let { listOf(it.toTraceBlock()) } ?: emptyList()
        val isText = !event.thinking && event.phase == null
        return DesktopStream(
            conversationId = conversationId,
            segments = listOf(
                StreamSegment(messageId = event.messageId, text = if (isText) event.token else "", sealed = false),
            ),
            thinking = event.thinking,
            toolTitle = toolTitleOf(event),
            traceBlocks = trace,
            done = event.done,
        )
    }

    private fun foldInto(cur: DesktopStream, event: StreamEvent): DesktopStream {
        // processEvent → 累计溯源（不进文本段）
        val trace = if (event.processEvent != null) {
            cur.traceBlocks + event.processEvent.toTraceBlock()
        } else {
            cur.traceBlocks
        }
        // 段切换（messageId 变化 → 上一段 sealed，开新段）；
        // streamStarting 预置的空段列表在此补建首段（ack 后首 token 落段）
        var segments = cur.segments
        if (segments.none { !it.sealed }) {
            segments = segments + StreamSegment(messageId = event.messageId, text = "", sealed = false)
        }
        val active = segments.lastOrNull()
        if (event.messageId != null && active != null && active.messageId != null && active.messageId != event.messageId) {
            segments = segments.mapIndexed { i, s -> if (i == segments.lastIndex) s.copy(sealed = true) else s }
            segments = segments + StreamSegment(event.messageId, "", sealed = false)
        }
        // 文本累加到活跃段
        if (!event.thinking && event.phase == null && event.token.isNotEmpty()) {
            val idx = segments.indexOfLast { !it.sealed }
            if (idx >= 0) {
                segments = segments.toMutableList().also {
                    it[idx] = it[idx].copy(text = it[idx].text + event.token)
                }
            }
        }
        var thinking = cur.thinking
        if (event.thinking) thinking = true
        if (event.phase == "tool" || event.phase == "process" || (!event.thinking && event.token.isNotEmpty())) {
            thinking = false
        }
        val toolTitle = if (event.phase == "tool") (event.statusText ?: event.toolName) else cur.toolTitle
        val done = event.done || cur.done
        return cur.copy(
            segments = segments,
            thinking = thinking && !done,
            toolTitle = if (done) null else toolTitle,
            traceBlocks = trace,
            done = done,
        )
    }

    private fun toolTitleOf(event: StreamEvent): String? =
        if (event.phase == "tool") (event.statusText ?: event.toolName) else null

    private companion object {
        const val TAG = "Companion/Stream"
    }
}

/** 溯源块 id 计数器（整改：identityHashCode 可碰撞且重解析不稳定，不能作 keyed diff 的列表 key）。 */
private val traceIdCounter = java.util.concurrent.atomic.AtomicLong(0)

/** 桌面 MessageProcessEvent → ExecutionTraceBlock（FR-29 三型镜像，简化映射）。 */
internal fun StreamEvent.ProcessEvent.toTraceBlock(): ExecutionTraceBlock {
    val id = "evt-${traceIdCounter.incrementAndGet()}"
    return when (eventType) {
        "thinking" -> ExecutionTraceBlock.Thinking(
            id = id,
            content = summary.ifEmpty { "思考中" },
            elapsedSeconds = null,
            isActive = status == "running",
        )
        "narration" -> ExecutionTraceBlock.Narration(id, summary)
        else -> {
            val actionType = when {
                toolName?.contains("read", ignoreCase = true) == true -> "read"
                toolName?.contains("edit", ignoreCase = true) == true -> "edit"
                toolName?.contains("write", ignoreCase = true) == true -> "write"
                toolName?.contains("bash", ignoreCase = true) == true ||
                    toolName?.contains("shell", ignoreCase = true) == true -> "shell"
                toolName?.contains("skill", ignoreCase = true) == true -> "skill"
                toolName?.contains("explore", ignoreCase = true) == true -> "explore"
                else -> "tool"
            }
            val status = when (status) {
                "running" -> ToolStatus.RUNNING
                "failed", "error" -> ToolStatus.FAILED
                else -> ToolStatus.COMPLETED
            }
            ExecutionTraceBlock.Action(
                id = id,
                title = summary.ifEmpty { toolName ?: actionType },
                status = status,
                actionType = actionType,
            )
        }
    }
}
