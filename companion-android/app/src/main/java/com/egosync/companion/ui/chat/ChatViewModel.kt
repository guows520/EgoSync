package com.egosync.companion.ui.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.ActionCardSuggestion
import com.egosync.companion.sync.ActionCardState
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.DecompositionState
import com.egosync.companion.sync.ExecutionTraceBlock
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.RoleProposal
import com.egosync.companion.sync.RoleProposalState
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TaskDecompositionProposal
import java.util.UUID
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ChatUiState(
    val messages: List<ChatMessage> = SnapshotStore.initialChat,
    /** 管家思考中（用户消息后、回复开始前）。 */
    val thinking: Boolean = false,
    val actionCards: List<ActionCardSuggestion> = emptyList(),
    /** FR-20：当前对话角色视图；null=管家 */
    val activeRoleId: String? = null,
    /** FR-20：切换器角色列表（镜像桌面 Sidebar 角色栏） */
    val roles: List<RoleCard> = SnapshotStore.roles,
    /** FR-2：对话流内嵌拆分提案卡；null=无提案 */
    val decomposition: TaskDecompositionProposal? = null,
    /** FR-5：角色涌现提案卡（含处理态）；null=未浮现。属管家对话流（角色视图不承载） */
    val roleProposal: RoleProposal? = null,
    /** FR-29：按消息挂执行溯源块（Think/Narration/Action） */
    val traceByMessageId: Map<String, List<ExecutionTraceBlock>> = emptyMap(),
    /** FR-33：流式期间工具执行状态行（工具名）；null=无 */
    val streamingToolTitle: String? = null,
    /** FR-33：流式打字机进行中（停止按钮可用态） */
    val responding: Boolean = false,
) {
    companion object {
        fun sample() = ChatUiState(
            messages = SnapshotStore.initialChat + ChatMessage(
                id = "m-u1",
                fromButler = false,
                text = "今天下午都有什么安排？",
            ),
            actionCards = listOf(
                ActionCardSuggestion(
                    id = "ac-1",
                    title = "把「整理季度 OKR 草稿」排进明天上午",
                    detail = "明早 9:30–11:00 无日程，是本周最完整的一块时间。",
                    fromRole = "产品经理",
                ),
            ),
        )
    }
}

/**
 * 管家对话 mock 状态机：发送 → 思考中 → 工具执行状态行 → 流式打字机回复（镜像 llm:stream 语义）。
 * 组 1 chat：FR-1 两段委派、FR-2 拆分提案、FR-20 角色切换、FR-29 溯源、FR-30 低置信、FR-33 工具可视+停止。
 * 初始消息与回复轮换取自容器快照（只读）；接入真实连接层后：发送改为 COMMAND 帧，回复改为 STREAM_TOKEN 帧驱动。
 */
class ChatViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        ChatUiState(messages = container.snapshotStore.initialChat)
    )
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private var replyIndex = 0
    private var nextId = 100

    /** 按视图隔离的已完成回复轮次（评审修正：全局计数会让角色视图绕行吞掉第 2 轮触发）；null 键=管家 */
    private val completedReplyRounds = mutableMapOf<String?, Int>()

    /** 委派第一段（交接声明）消息 id：第二段开始前被停止则回滚，防悬空孤气泡 */
    private var pendingHandoffMsgId: String? = null

    /** 按角色视图保存会话（镜像桌面每角色独立 conversation）；null 键=管家 */
    private val history = mutableMapOf<String?, List<ChatMessage>>()

    /** 当前流式协程句柄：停止按钮（FR-33 Square）取消之 */
    private var streamJob: Job? = null
    private var streamingMsgId: String? = null

    init {
        history[null] = container.snapshotStore.initialChat
    }

    // ── FR-20 角色切换 ─────────────────────────────────────────────────

    fun selectRole(roleId: String?) {
        if (roleId == _uiState.value.activeRoleId) return
        // 切换前终止进行中的流式回复（镜像桌面切视图即重置流式态）
        stopStreaming()
        streamJob?.cancel()
        streamJob = null
        _uiState.update {
            it.copy(
                activeRoleId = roleId,
                messages = history.getOrPut(roleId) {
                    if (roleId == null) container.snapshotStore.initialChat
                    else container.snapshotStore.roleChatSeeds[roleId] ?: emptyList()
                },
                thinking = false,
                responding = false,
                streamingToolTitle = null,
                // 提案卡/建议卡属管家对话流；角色视图不承载（镜像桌面按 conversation 隔离）
                actionCards = if (roleId == null) it.actionCards else emptyList(),
                decomposition = if (roleId == null) it.decomposition else null,
                roleProposal = if (roleId == null) it.roleProposal else null,
            )
        }
    }

    // ── 发送与流式回复 ────────────────────────────────────────────────

    fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        // 并发守卫：上一条回复未完成（思考中/流式中）时忽略新发送，防打字机交错
        if (_uiState.value.thinking || _uiState.value.responding) return

        appendMessage(ChatMessage(nextId++.toString(), false, trimmed))

        // FR-1：管家视图命中委派关键词 → 两段委派路由
        val delegationRoleId = if (_uiState.value.activeRoleId == null) {
            container.snapshotStore.delegationKeywords.entries
                .firstOrNull { trimmed.contains(it.key) }?.value
        } else null

        streamJob = viewModelScope.launch {
            if (delegationRoleId != null) {
                runDelegation(delegationRoleId)
            } else {
                runNormalReply()
            }
        }
    }

    /** FR-1 两段委派（镜像 ChatStream.tsx streamBubbles 分桶语义）：
     *  气泡一「稍等，我让 X 看一下」（管家）→ 气泡二「来自 X 的反馈…」（角色头像+名）。 */
    private suspend fun runDelegation(roleId: String) {
        val roleName = container.snapshotStore.roles.find { it.id == roleId }?.name ?: "角色"
        _uiState.update { it.copy(thinking = true, responding = true) }
        delay(600)
        _uiState.update { it.copy(thinking = false) }

        // 第一段：管家委派声明（即时落定，不走打字机）；记录 id 供中途停止时回滚
        val handoffId = nextId++.toString()
        pendingHandoffMsgId = handoffId
        appendMessage(ChatMessage(handoffId, true, container.snapshotStore.delegationFirstSegment(roleName)))
        delay(500)
        currentCoroutineContext().ensureActive()
        pendingHandoffMsgId = null

        // 第二段：目标角色反馈（流式打字机）
        val full = container.snapshotStore.delegationReplies[roleId]
            ?: container.snapshotStore.butlerReplies[replyIndex++ % container.snapshotStore.butlerReplies.size]
        streamTypewriter(full, senderRoleId = roleId)
        _uiState.update { it.copy(responding = false) }
    }

    /** 常规单段回复：思考 → 工具执行状态行（FR-33）→ 打字机；第 2 轮（按视图计）附拆分提案+溯源。 */
    private suspend fun runNormalReply() {
        val viewKey = _uiState.value.activeRoleId
        _uiState.update { it.copy(thinking = true, responding = true) }
        delay(700)
        _uiState.update { it.copy(thinking = false) }

        // FR-33：工具执行过程可视（工具名+运行中状态轮换）
        for (stage in container.snapshotStore.toolExecutionStages) {
            _uiState.update { it.copy(streamingToolTitle = stage) }
            delay(350)
        }
        _uiState.update { it.copy(streamingToolTitle = null) }

        val replies = container.snapshotStore.butlerReplies
        // FR-30：管家视图第 4 轮回复走低置信（mock confidence<0.7）
        val lowConfidence = viewKey == null && replyIndex % replies.size == 3
        val full = if (lowConfidence) container.snapshotStore.lowConfidenceReply
        else replies[replyIndex % replies.size]
        replyIndex++

        val msgId = streamTypewriter(full, lowConfidence = lowConfidence)
        _uiState.update { it.copy(responding = false) }
        if (msgId == null) return // 已被停止中断：不计轮次、不触发卡片

        // 本轮完整落定才计入该视图轮次（被停止打断的轮次不消耗触发点）
        val round = (completedReplyRounds[viewKey] ?: 0) + 1
        completedReplyRounds[viewKey] = round

        // FR-2/FR-29：第 2 轮回复后浮现拆分提案卡，并把执行溯源挂到本条助手消息（仅管家对话流）
        if (round == 2 && viewKey == null) {
            if (_uiState.value.decomposition == null) {
                _uiState.update { it.copy(decomposition = container.snapshotStore.decompositionProposal) }
            }
            _uiState.update {
                it.copy(traceByMessageId = it.traceByMessageId + (msgId to container.snapshotStore.executionTrace))
            }
        }

        // 第 2 轮对话后浮现一张待确认建议卡（FR-11/12 ActionCard，既有行为保留；仅管家对话流）
        if (round == 2 && viewKey == null && _uiState.value.actionCards.isEmpty()) {
            _uiState.update {
                it.copy(
                    actionCards = it.actionCards + ActionCardSuggestion(
                        id = UUID.randomUUID().toString(),
                        title = "把「整理季度 OKR 草稿」排进明天上午",
                        detail = "明早 9:30–11:00 无日程，是本周最完整的一块时间。确认后我来安排。",
                        fromRole = "产品经理",
                    )
                )
            }
        }

        // FR-5：第 2 轮回复后浮现角色涌现提案卡（仅管家对话流，一次性守卫）
        if (round == 2 && viewKey == null && _uiState.value.roleProposal == null) {
            _uiState.update { it.copy(roleProposal = container.snapshotStore.roleProposal) }
        }
    }

    /** 打字机流式输出，返回消息 id（供溯源挂靠）；被停止中断时返回 null。
     *  每次写入前 ensureActive：取消后不再产生任何状态写入（评审修正）。 */
    private suspend fun streamTypewriter(
        full: String,
        senderRoleId: String? = null,
        lowConfidence: Boolean = false,
    ): String? {
        currentCoroutineContext().ensureActive()
        val msgId = nextId++.toString()
        streamingMsgId = msgId
        appendMessage(
            ChatMessage(msgId, true, "", streaming = true, senderRoleId = senderRoleId, lowConfidence = lowConfidence)
        )
        val builder = StringBuilder()
        for (ch in full) {
            builder.append(ch)
            val acc = builder.toString()
            currentCoroutineContext().ensureActive()
            updateMessage(msgId) { it.copy(text = acc, streaming = true) }
            delay(24)
        }
        updateMessage(msgId) { it.copy(streaming = false) }
        streamingMsgId = null
        persistCurrentMessages() // 终态回写 history，防止切视图恢复出空气泡（评审修正）
        return msgId
    }

    // ── FR-33 停止 ────────────────────────────────────────────────────

    /** 停止流式回复（镜像桌面 ChatInput 停止，Square 图标）：中断打字机，已浮现内容落定；
     *  若停在委派两段之间，一并回滚第一段交接声明，不留悬空孤气泡。 */
    fun stopStreaming() {
        streamJob?.cancel()
        streamJob = null
        val msgId = streamingMsgId
        streamingMsgId = null
        _uiState.update {
            var messages = it.messages.map { m ->
                if (m.id == msgId || m.streaming) m.copy(streaming = false) else m
            }
            pendingHandoffMsgId?.let { handoffId ->
                messages = messages.filterNot { m -> m.id == handoffId }
                pendingHandoffMsgId = null
            }
            it.copy(
                thinking = false,
                responding = false,
                streamingToolTitle = null,
                messages = messages,
            )
        }
        persistCurrentMessages()
    }

    // ── FR-2 拆分提案响应 ─────────────────────────────────────────────

    fun respondDecomposition(proposalId: String, accepted: Boolean) {
        // 并发守卫：流式/思考中不受理，防回执插进未完成的流式气泡之前（评审修正）
        if (_uiState.value.thinking || _uiState.value.responding) return
        // 双击守卫：仅 PENDING 提案可响应
        val proposal = _uiState.value.decomposition ?: return
        if (proposal.id != proposalId || proposal.state != DecompositionState.PENDING) return

        _uiState.update {
            it.copy(
                decomposition = proposal.copy(
                    state = if (accepted) DecompositionState.ACCEPTED else DecompositionState.KEPT_SINGLE
                ),
                messages = it.messages + ChatMessage(
                    id = nextId++.toString(),
                    fromButler = true,
                    // 条目数动态拼装，避免与卡片渲染的 items.size 矛盾（评审修正）
                    text = if (accepted) "好的，已按拆分创建 ${proposal.items.size} 个任务，排进对应象限了。"
                    else "明白，那就保持一件事，不拆分。",
                ),
            )
        }
        persistCurrentMessages()
    }

    fun respondActionCard(cardId: String, confirmed: Boolean) {
        // 并发守卫：流式/思考中不受理（评审修正）
        if (_uiState.value.thinking || _uiState.value.responding) return
        // 双击守卫：仅 PENDING 卡片可响应，防重复回复
        val target = _uiState.value.actionCards.find { it.id == cardId } ?: return
        if (target.state != ActionCardState.PENDING) return

        _uiState.update { state ->
            state.copy(
                actionCards = state.actionCards.map {
                    if (it.id == cardId) {
                        it.copy(state = if (confirmed) ActionCardState.CONFIRMED else ActionCardState.REJECTED)
                    } else it
                },
                messages = state.messages + ChatMessage(
                    id = nextId++.toString(),
                    fromButler = true,
                    text = if (confirmed) {
                        "好的，已确认。我这就转给产品经理去办，完成后第一时间告诉您。"
                    } else {
                        "明白，那这条建议就先放下了。需要时随时叫我。"
                    },
                ),
            )
        }
        persistCurrentMessages()
    }

    // ── FR-5 角色涌现提案 ──────────────────────────────────────────────

    /** FR-5 创建角色：提案卡以编辑后的值置已创建态 + 管家回执（mock：仅内存态，不落角色列表）。 */
    fun confirmRoleProposal(name: String, icon: String, color: String, goal: String) {
        // 并发守卫：流式/思考中不受理（与其他卡响应一致）
        if (_uiState.value.thinking || _uiState.value.responding) return
        val proposal = _uiState.value.roleProposal ?: return
        if (proposal.state != RoleProposalState.PENDING) return // 双击守卫

        _uiState.update {
            it.copy(
                roleProposal = proposal.copy(
                    name = name, icon = icon, color = color, goal = goal,
                    state = RoleProposalState.CREATED,
                ),
                messages = it.messages + ChatMessage(
                    id = nextId++.toString(),
                    fromButler = true,
                    text = "好的，已创建角色「$name」。以后这类事可以交给它跟进。",
                ),
            )
        }
        persistCurrentMessages()
    }

    /** FR-5 不需要（弹窗「不需要」/关闭或卡片按钮）：提案卡置已跳过态 + 管家回执。 */
    fun skipRoleProposal() {
        if (_uiState.value.thinking || _uiState.value.responding) return
        val proposal = _uiState.value.roleProposal ?: return
        if (proposal.state != RoleProposalState.PENDING) return

        _uiState.update {
            it.copy(
                roleProposal = proposal.copy(state = RoleProposalState.SKIPPED),
                messages = it.messages + ChatMessage(
                    id = nextId++.toString(),
                    fromButler = true,
                    text = "明白，这个角色先不创建。需要时随时跟我说。",
                ),
            )
        }
        persistCurrentMessages()
    }

    // ── 内部工具 ──────────────────────────────────────────────────────

    private fun appendMessage(message: ChatMessage) {
        _uiState.update { it.copy(messages = it.messages + message) }
        persistCurrentMessages()
    }

    private fun updateMessage(id: String, transform: (ChatMessage) -> ChatMessage) {
        _uiState.update { state ->
            state.copy(messages = state.messages.map { if (it.id == id) transform(it) else it })
        }
    }

    private fun persistCurrentMessages() {
        history[_uiState.value.activeRoleId] = _uiState.value.messages
    }
}