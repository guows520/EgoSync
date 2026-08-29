package com.egosync.companion.ui.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.sync.ActionCardSuggestion
import com.egosync.companion.sync.ActionCardState
import com.egosync.companion.sync.ChatConversation
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
    val messages: List<ChatMessage> = emptyList(),
    /** 管家思考中（用户消息后、回复开始前）。 */
    val thinking: Boolean = false,
    val actionCards: List<ActionCardSuggestion> = emptyList(),
    /** FR-20：当前对话角色视图；null=管家 */
    val activeRoleId: String? = null,
    /** FR-20：切换器角色列表（镜像桌面 Sidebar 角色栏） */
    val roles: List<RoleCard> = emptyList(),
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
    /** 多会话：当前视图的会话列表（最新在前，镜像桌面 ChatHeader 历史下拉） */
    val conversations: List<ChatConversation> = emptyList(),
    /** 多会话：当前激活会话 id */
    val currentConversationId: String = "",
) {
    companion object {
        fun sample() = ChatUiState(
            messages = previewInitialChat + ChatMessage(
                id = "m-u1",
                fromButler = false,
                text = "今天下午都有什么安排？",
            ),
            roles = com.egosync.companion.ui.previewRoles,
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
 * 会话列表与历史消息取自快照 conversations 域（STATE_DELTA 全量替换即时刷新，AC2）；
 * 回复轮换/提案等演示数据见 [ChatDemoData]（生成性内容，指令通道 13.3 接入）。
 */
class ChatViewModel(private val store: SnapshotStore) : ViewModel() {

    private var replyIndex = 0
    private var nextId = 100

    /** 按视图隔离的已完成回复轮次（评审修正：全局计数会让角色视图绕行吞掉第 2 轮触发）；null 键=管家。
     *  切会话不清轮次：多会话下第 2 轮拆分提案等 mock 触发行为不回归。 */
    private val completedReplyRounds = mutableMapOf<String?, Int>()

    /** 委派第一段（交接声明）消息 id：第二段开始前被停止则回滚，防悬空孤气泡 */
    private var pendingHandoffMsgId: String? = null

    /** 按角色视图保存多会话列表（快照 conversations 域 + 本地新建/回复暂存）；null 键=管家 */
    private val history: MutableMap<String?, MutableList<ChatConversation>> = mutableMapOf()

    /** 各视图（null=管家）离开时的活跃会话 id：切回视图恢复原会话，不重置为最新。 */
    private val activeConversationIdByRole = mutableMapOf<String?, String>()

    /** 当前流式协程句柄：停止按钮（FR-33 Square）取消之 */
    private var streamJob: Job? = null
    private var streamingMsgId: String? = null

    private val _uiState: MutableStateFlow<ChatUiState>
    val uiState: StateFlow<ChatUiState>

    init {
        // 种子会话：快照 conversations 域（未收到快照时为空 → 兜底新建空会话防消息无处持久）
        seedFromStore()
        var current = conversationsOf(null).firstOrNull()
            ?: createConversationIn(null)
        activeConversationIdByRole[null] = current.id
        _uiState = MutableStateFlow(
            ChatUiState(
                roles = store.roles,
                messages = current.messages,
                conversations = conversationsOf(null),
                currentConversationId = current.id,
            )
        )
        uiState = _uiState
        // AC2：STATE_DELTA 即时刷新——快照全量替换后重建会话列表与消息流
        //（本地暂存〔新建会话/本地回复〕被覆盖为已知过渡态，指令通道 13.3 收口）。
        // 跳过构造时已加载的同对象首发：否则首帧会误触重建，抹掉 selectRole 等本地交互暂存
        viewModelScope.launch {
            var lastSnapshot = store.state.value.snapshot
            store.state.collect { state ->
                when {
                    state.loaded && state.snapshot !== lastSnapshot -> {
                        lastSnapshot = state.snapshot
                        onSnapshotReplaced()
                    }
                    // unpair/密钥失效自愈（store.clear 不导航）：会话与角色一并清空，
                    // 不残留已解配桌面的陈旧数据（评审 P2）；重建兜底空会话防消息无处持久
                    !state.loaded -> {
                        lastSnapshot = null
                        stopStreaming()
                        history.clear()
                        activeConversationIdByRole.clear()
                        val fresh = createConversationIn(null)
                        activeConversationIdByRole[null] = fresh.id
                        _uiState.update {
                            it.copy(
                                roles = emptyList(),
                                activeRoleId = null,
                                messages = emptyList(),
                                conversations = conversationsOf(null),
                                currentConversationId = fresh.id,
                                thinking = false,
                                responding = false,
                                streamingToolTitle = null,
                                actionCards = emptyList(),
                                decomposition = null,
                                roleProposal = null,
                                traceByMessageId = emptyMap(),
                            )
                        }
                    }
                }
            }
        }
    }

    /** 用快照 conversations 域重建各视图会话列表。 */
    private fun seedFromStore() {
        history.clear()
        history[null] = store.conversationsOf(null).toMutableList()
        store.roles.forEach { role ->
            history[role.id] = store.conversationsOf(role.id).toMutableList()
        }
    }

    /** 快照全量替换后重建：保持当前选中会话（消失则回退最新），空视图兜底空会话。 */
    private fun onSnapshotReplaced() {
        // 快照替换与流式互斥（评审 P3）：打字机/思考进行中到达 STATE_DELTA，先停流再重建——
        // 否则打字机会在重建后的消息流上空转续写，回复静默丢失且 responding 滞留
        stopStreaming()
        val viewRoleId = _uiState.value.activeRoleId
        seedFromStore()
        // 当前查看角色被桌面删除：回退管家视图（切换器已无该角色入口，不留幽灵空视图，评审 P5）
        val roleId = if (viewRoleId != null && store.roles.none { it.id == viewRoleId }) null else viewRoleId
        if (roleId != viewRoleId) {
            // 被删角色的会话暂存与活跃会话记忆一并清掉（数据源已不含该角色）
            activeConversationIdByRole.remove(viewRoleId)
        }
        if (history[null].isNullOrEmpty()) createConversationIn(null)
        if (roleId != null && history[roleId].isNullOrEmpty()) createConversationIn(roleId)
        val conversations = conversationsOf(roleId)
        val currentId = _uiState.value.currentConversationId
        val current = conversations.firstOrNull { it.id == currentId } ?: conversations.first()
        // 回落换会话或视角回退：卡片态随会话清空（与 selectConversation 同一不变量，评审 P4）
        val switched = current.id != currentId || roleId != viewRoleId
        activeConversationIdByRole[roleId] = current.id
        _uiState.update {
            it.copy(
                roles = store.roles,
                activeRoleId = roleId,
                messages = current.messages,
                conversations = conversations,
                currentConversationId = current.id,
                thinking = false,
                responding = false,
                streamingToolTitle = null,
                actionCards = if (switched) emptyList() else it.actionCards,
                decomposition = if (switched) null else it.decomposition,
                roleProposal = if (switched) null else it.roleProposal,
                traceByMessageId = if (switched) emptyMap() else it.traceByMessageId,
            )
        }
    }

    // ── FR-20 角色切换 ─────────────────────────────────────────────────

    fun selectRole(roleId: String?) {
        if (roleId == _uiState.value.activeRoleId) return
        // 切换前终止进行中的流式回复（镜像桌面切视图即重置流式态）
        stopStreaming()
        streamJob?.cancel()
        streamJob = null
        var conversations = conversationsOf(roleId)
        // 恢复该视图离开时的会话（不因往返切换丢失用户上下文）；无记忆则取最新
        var current = conversations.firstOrNull { it.id == activeConversationIdByRole[roleId] }
            ?: conversations.firstOrNull()
        if (current == null) {
            // 该角色无任何会话（未种子化角色）：兜底新建空会话，防消息无处持久
            current = createConversationIn(roleId)
            conversations = conversationsOf(roleId)
        }
        activeConversationIdByRole[roleId] = current.id
        _uiState.update {
            it.copy(
                activeRoleId = roleId,
                messages = current.messages,
                conversations = conversations,
                currentConversationId = current.id,
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

    // ── 多会话：新建 / 切换 / 删除（移植桌面 ChatHeader 语义）────────────

    /** 新建会话：中断流式（同 selectRole），追加空会话并设为当前。 */
    fun newConversation() {
        stopStreaming()
        streamJob?.cancel()
        streamJob = null
        val roleId = _uiState.value.activeRoleId
        val conversation = createConversationIn(roleId)
        activeConversationIdByRole[roleId] = conversation.id
        _uiState.update {
            it.copy(
                messages = emptyList(),
                conversations = conversationsOf(roleId),
                currentConversationId = conversation.id,
                thinking = false,
                responding = false,
                streamingToolTitle = null,
                // 卡片态随会话隔离：新空会话不承载旧会话的提案/建议卡（镜像桌面按 conversation 隔离）
                actionCards = emptyList(),
                decomposition = null,
                roleProposal = null,
            )
        }
    }

    /** 在指定角色会话列表中新建空会话并落库（updatedAt 严格最大，防同毫秒排序不稳）。 */
    private fun createConversationIn(roleId: String?): ChatConversation {
        val newestExisting = history[roleId].orEmpty().maxOfOrNull { it.updatedAt } ?: 0L
        val conversation = ChatConversation(
            id = UUID.randomUUID().toString(),
            title = "",
            updatedAt = maxOf(System.currentTimeMillis(), newestExisting + 1),
        )
        history.getOrPut(roleId) { mutableListOf() }.add(conversation)
        return conversation
    }

    /** 切换会话：中断流式，换消息流；卡片态随会话清空（不跨会话携带）。 */
    fun selectConversation(id: String) {
        if (id == _uiState.value.currentConversationId) return
        // 先校验存在再中断流式：无效 id 不应无谓打断进行中的回复
        val roleId = _uiState.value.activeRoleId
        val conversation = history[roleId].orEmpty().find { it.id == id } ?: return
        stopStreaming()
        streamJob?.cancel()
        streamJob = null
        activeConversationIdByRole[roleId] = id
        _uiState.update {
            it.copy(
                messages = conversation.messages,
                currentConversationId = id,
                conversations = conversationsOf(roleId),
                thinking = false,
                responding = false,
                streamingToolTitle = null,
                // 卡片态随会话隔离：旧会话的提案/建议卡不带入目标会话
                actionCards = emptyList(),
                decomposition = null,
                roleProposal = null,
            )
        }
    }

    /** 删除会话：仅移除；删当前则回退剩余 updatedAt 最新者，无剩余自动新建空会话。 */
    fun deleteConversation(id: String) {
        val roleId = _uiState.value.activeRoleId
        val list = history[roleId] ?: return
        if (list.none { it.id == id }) return
        val isCurrent = _uiState.value.currentConversationId == id
        if (isCurrent) {
            // 删当前会话先中断流式，防进行中的流式文本回写进回退后的会话
            stopStreaming()
            streamJob?.cancel()
            streamJob = null
        }
        list.removeAll { it.id == id }
        if (!isCurrent) {
            _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
            return
        }
        val fallback = list.maxByOrNull { it.updatedAt }
        if (fallback != null) {
            activeConversationIdByRole[roleId] = fallback.id
            _uiState.update {
                it.copy(
                    messages = fallback.messages,
                    conversations = conversationsOf(roleId),
                    currentConversationId = fallback.id,
                    // 换了当前会话：卡片态随会话清空
                    actionCards = emptyList(),
                    decomposition = null,
                    roleProposal = null,
                )
            }
        } else {
            newConversation()
        }
    }

    // ── 发送与流式回复 ────────────────────────────────────────────────

    fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        // 并发守卫：上一条回复未完成（思考中/流式中）时忽略新发送，防打字机交错
        if (_uiState.value.thinking || _uiState.value.responding) return

        // 空会话（title 为空）收到首条用户消息后：标题取前 16 字符，超过加「…」（镜像桌面会话命名）
        val roleId = _uiState.value.activeRoleId
        val currentId = _uiState.value.currentConversationId
        val currentConversation = history[roleId].orEmpty().find { it.id == currentId }
        if (currentConversation != null && currentConversation.title.isEmpty()) {
            var title = if (trimmed.length > 16) trimmed.take(16) else trimmed
            // 防孤立代理：截断落在代理对（emoji/增补平面字符）中间时退一位，避免残破字符
            if (trimmed.length > 16 && title.isNotEmpty() && Character.isHighSurrogate(title.last())) {
                title = title.dropLast(1)
            }
            if (trimmed.length > 16) title += "…"
            replaceConversation(currentId, currentConversation.copy(title = title))
        }

        appendMessage(ChatMessage(nextId++.toString(), false, trimmed))

        // FR-1：管家视图命中委派关键词 → 两段委派路由。
        // 关键词随快照角色名派生（角色名 → id）：mock 时代写死 role-pm 等演示键，
        // 真实快照角色 id 为 UUID，写死路由会落空（评审 P6）
        val delegationRoleId = if (_uiState.value.activeRoleId == null) {
            store.roles
                .filter { it.name.isNotBlank() }
                .firstOrNull { trimmed.contains(it.name) }?.id
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
        val roleName = store.roles.find { it.id == roleId }?.name ?: "角色"
        _uiState.update { it.copy(thinking = true, responding = true) }
        delay(600)
        _uiState.update { it.copy(thinking = false) }

        // 第一段：管家委派声明（即时落定，不走打字机）；记录 id 供中途停止时回滚
        val handoffId = nextId++.toString()
        pendingHandoffMsgId = handoffId
        appendMessage(ChatMessage(handoffId, true, delegationFirstSegment(roleName)))
        delay(500)
        currentCoroutineContext().ensureActive()
        pendingHandoffMsgId = null

        // 第二段：目标角色反馈（流式打字机）。演示表按演示角色 id 命中保留；
        // 真实快照 UUID 角色未命中时用角色名模板生成同语义反馈（指令通道 13.3 收口）
        val full = delegationReplies[roleId]
            ?: "来自${roleName}的反馈：收到，这事我接下了。我先梳理一下当前进展，稍后给你一个初步安排。"
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
        for (stage in toolExecutionStages) {
            _uiState.update { it.copy(streamingToolTitle = stage) }
            delay(350)
        }
        _uiState.update { it.copy(streamingToolTitle = null) }

        val replies = butlerReplies
        if (replies.isEmpty()) {
            // T8 崩溃守卫：演示回复表为空时 `replyIndex % replies.size` 会除零崩溃——
            // 落定本轮不触发卡片，不产生空气泡（演示回复为编译期常量，守卫仅作防呆）
            _uiState.update { it.copy(responding = false) }
            return
        }
        // FR-30：管家视图第 4 轮回复走低置信（mock confidence<0.7）
        val lowConfidence = viewKey == null && replyIndex % replies.size == 3
        val full = if (lowConfidence) lowConfidenceReply
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
                _uiState.update { it.copy(decomposition = decompositionProposal) }
            }
            _uiState.update {
                it.copy(traceByMessageId = it.traceByMessageId + (msgId to executionTrace))
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
            _uiState.update { it.copy(roleProposal = com.egosync.companion.ui.chat.roleProposal) }
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

    /** 回写当前会话的 messages 与 updatedAt（流式终态/卡片回执等落定点）。 */
    private fun persistCurrentMessages() {
        val roleId = _uiState.value.activeRoleId
        val currentId = _uiState.value.currentConversationId
        if (currentId.isEmpty()) return
        val list = history[roleId] ?: return
        val index = list.indexOfFirst { it.id == currentId }
        if (index < 0) return
        list[index] = list[index].copy(
            messages = _uiState.value.messages,
            updatedAt = System.currentTimeMillis(),
        )
        // 同步刷新会话列表快照：updatedAt 变化影响「最新在前」排序与相对时间显示
        _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
    }

    /** 替换指定会话（保持列表位置不变），并刷新 UiState.conversations。 */
    private fun replaceConversation(conversationId: String, conversation: ChatConversation) {
        val roleId = _uiState.value.activeRoleId
        val list = history[roleId] ?: return
        val index = list.indexOfFirst { it.id == conversationId }
        if (index < 0) return
        list[index] = conversation
        _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
    }

    /** 当前视图的会话列表（最新在前，镜像桌面 ConversationList 排序）。 */
    private fun conversationsOf(roleId: String?): List<ChatConversation> =
        history[roleId].orEmpty().sortedByDescending { it.updatedAt }
}
