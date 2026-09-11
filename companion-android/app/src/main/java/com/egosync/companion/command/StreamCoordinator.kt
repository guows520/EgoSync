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
 * 暂存（只留最新），流结束/失败/超时立即补应用（spec-fix-companion-chat-stream-ux：
 * flush 前做新鲜度校验——暂存已含本轮完成回复才应用，陈旧即丢弃，防 done 时刻
 * 回放流中段快照掐掉已定气泡；done 写信号触发的新快照约 2s 后直通收敛）。
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
        /**
         * 本轮回复锚点 id（M1'）：ack 的 assistantMessageId 最早可得，任意 Some
         * messageId 帧兜底捕获——本轮所有 Some id 同指一落库行（final_message_id
         * ==占位 id）。消费方：flush 新鲜度守卫 + ChatViewModel 渲染 id（快照落地
         * key 稳定、trace 不孤儿化）。
         */
        val roundAssistantId: String?,
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

    /** chat.send ack 后置活跃标志（首 token 前的思考态/工具态也计入活跃）。
     *  M1'：ack 的 assistantMessageId 作本轮锚点传入（快照落地 key 稳定、
     *  flush 守卫按锚点行校验新鲜度）。 */
    fun streamStarting(conversationId: String, assistantMessageId: String? = null) {
        if (_state.value?.takeIf { !it.done && it.conversationId == conversationId } != null) return
        // 整改：跨会话替换活跃流前先 flush 暂存（与 onStreamToken 替换路径
        // 一致）——旧流的暂存快照不得记在新流账上延迟应用。
        if (_state.value?.takeIf { !it.done && it.conversationId != conversationId } != null) {
            flushStash()
        }
        _state.value = DesktopStream(
            conversationId = conversationId,
            roundAssistantId = assistantMessageId,
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
            // done 残留替换前先 flush（与下方跨会话分支同一不变量）：旧回合
            // 暂存不得记入新回合延迟回放。
            flushStash()
            _state.value = foldNew(event, event.conversationId, cur)
            return
        }
        // 不同会话的流到达（防御：替换——桌面串行不会并发，但跨会话切换残留需重置）
        if (event.conversationId != cur.conversationId) {
            flushStash()
            _state.value = foldNew(event, event.conversationId, null)
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

    /** 会话彻底不可用：先按本回合上下文 flush 再清态（守卫需读回合锚点/会话；
     *  VM 据 null 态感知断连）。陈旧暂存被守卫丢弃——连接死亡时保持本地已定
     *  文本优于回放陈旧快照，重连后新快照收敛。 */
    fun reset() {
        flushStash()
        _state.value = null
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
        } ?: return
        // M1' 新鲜度守卫：陈旧即丢弃（done 必触发桌面写信号重建，含完成行的
        // 新快照约 2s 后直通应用；断连不达则保持本地已定文本，重连后收敛）
        if (isStaleForRound(toApply.first)) return
        toApply.let { (snapshot, raw) -> applySnapshot(snapshot, raw) }
    }

    /**
     * 暂存快照新鲜度校验（M1'）：已含本轮完成回复才允许回放。
     * WHY：流中段 STATE_DELTA（占位行 isComplete=false、content 空）在 done
     * 时刻 flush 会触发 onSnapshotReplaced 掐掉已定文本——气泡消失约 2s 后
     * 随写信号新快照回归（闪烁）。陈旧暂存丢弃是唯一不回放空气泡的解。
     * - 有锚点：锚点消息须 isComplete 且 content 非空；
     * - 无锚点（旧桌面/兜底路径）：回退会话末条 assistant 同校验；
     * - 无回合上下文 / 快照无该会话：视为陈旧。
     */
    private fun isStaleForRound(snapshot: DesktopSnapshot): Boolean {
        val round = _state.value ?: return true
        val conversation = snapshot.conversations.find { it.id == round.conversationId }
            ?: return true
        val anchorId = round.roundAssistantId
        val row = if (anchorId != null) {
            conversation.messages.find { it.id == anchorId }
        } else {
            conversation.messages.lastOrNull { it.role == "assistant" }
        }
        return row == null || !row.isComplete || row.content.isBlank()
    }

    private fun foldNew(event: StreamEvent, conversationId: String, prevRound: DesktopStream?): DesktopStream {
        val trace = event.processEvent?.let { listOf(it.toTraceBlock()) } ?: emptyList()
        val isText = isAnswerText(event)
        return DesktopStream(
            conversationId = conversationId,
            // M1' 锚点：事件自身 Some messageId（首帧即段的 id，渲染不翻转）；
            // 同会话旧回合的锚点保留（停止后迟到 token 重启流——ack 锚点不得
            // 被 null 事件覆写丢失）；跨会话替换绝不沿用旧回合锚点。
            roundAssistantId = event.messageId
                ?: prevRound?.takeIf { it.conversationId == conversationId }?.roundAssistantId,
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
        if (event.messageId != null && active != null && active.messageId != event.messageId) {
            // null→Some 切换（M2' 委派分段恢复）：仅 answering 正文帧允许——唤醒帧
            //（answering+空 token+非 done，agent_engine.rs:3034/3047）标志第二段开始；
            // process/tool/done 帧也带 Some 占位 id，仅按 messageId 放宽会把普通流
            // 的工具期/收口帧误切两气泡（Design Notes 分段窄化）。
            val splitOnNullActive = active.messageId == null && isAnswerText(event) && !event.done
            if (active.messageId != null || splitOnNullActive) {
                segments = segments.mapIndexed { i, s -> if (i == segments.lastIndex) s.copy(sealed = true) else s }
                segments = segments + StreamSegment(event.messageId, "", sealed = false)
            }
        }
        // 文本累加到活跃段
        if (isAnswerText(event) && event.token.isNotEmpty()) {
            val idx = segments.indexOfLast { !it.sealed }
            if (idx >= 0) {
                segments = segments.toMutableList().also {
                    it[idx] = it[idx].copy(text = it[idx].text + event.token)
                }
            }
        }
        var thinking = cur.thinking
        if (event.thinking) thinking = true
        // 思考态关闭与正文累加共用同一判定（isAnswerText）——两处判据漂移是
        // 「空光标后整段回填」的历史根因形态，不允许再次分叉
        if (event.phase == "tool" || event.phase == "process" || (isAnswerText(event) && event.token.isNotEmpty())) {
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
            // M1'：done 帧兜底捕获锚点（spec I/O 矩阵：无锚路径仅 done 落库受益）。
            // 流中段不捕获——process/tool/唤醒帧的 Some id 会让 null 段渲染 id
            // 从合成兜底中途翻转为锚点（LazyColumn key 翻转闪烁）；ack 锚点
            // 与段自身 id 均不受此限。
            roundAssistantId = (if (event.done) event.messageId else null) ?: cur.roundAssistantId,
        )
    }

    /**
     * 桌面正文 token 判定（跨端契约，SPEC-companion-connection-chat-ux / streaming-protocol.md）：
     * `thinking=false && phase ∈ {null, "answering"}`。
     * - "answering"：生产总线 delta 路径（桌面 agent_engine.rs emit_stream_token 硬编码）；
     * - null：历史 SSE Text/Error 收口路径（桌面 SseEvent 映射，serde skip 后无 phase 字段）。
     * 值域锚点：桌面 models/chat.rs `stream_phase_domain_is_locked` 测试 + 本仓
     * `app/src/test/resources/streaming/` 黄金契约 fixture。tool/process/thinking/done
     * 及未知 phase 值永不为正文（未知值保守忽略，不改变 thinking 态）。
     */
    private fun isAnswerText(event: StreamEvent): Boolean =
        !event.thinking && (event.phase == null || event.phase == "answering")

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
