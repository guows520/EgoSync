package com.egosync.companion.ui.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.command.CommandEnvelope
import com.egosync.companion.command.CommandException
import com.egosync.companion.command.CommandSender
import com.egosync.companion.command.StreamCoordinator
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
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.json.JSONObject

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
 * 管家对话状态机（Story 13.3 换装：mock 打字机 → 真实指令通道）。
 * 发送经 `COMMAND(chat.send)`，回复经 `STREAM_TOKEN` 逐 token 回流；思考/工具/
 * 溯源状态全部来自 token payload 的 phase/thinking/processEvent（事件缺席保持
 * null 不造假）。会话生命周期（新建/删除）经 `conversation.new/delete` 指令。
 * 组 1 chat 其余特性（FR-1 委派/FR-2 拆分/FR-5 涌现）演示数据移出生产路径——
 * 真实委派由桌面管家路由、回复照常经 token 流回流；拆分/涌现待桌面结构化数据落地。
 */
class ChatViewModel(
    private val store: SnapshotStore,
    private val commands: CommandSender?,
    private val stream: StreamCoordinator,
    private val onError: (String) -> Unit = {},
) : ViewModel() {

    private var nextId = 100
    /** 本地占位会话 id 集（未与桌面对齐 / 离线兜底）；chat.send 时这类会话不传 conversationId，
     *  由桌面新建并回填真实 id。 */
    private val localConversationIds = mutableSetOf<String>()

    /** 流式渲染基线（流开始时的消息流，多段续写时累加其上）。按会话隔离。 */
    private var streamBase: List<ChatMessage>? = null
    private var renderedStreamConv: String? = null

    /** 流序号：无 messageId 段落的兜底 id 组成部分（跨流唯一，评审 C6）。 */
    private var streamSeq = 0

    /** 当前发送是否已见到流态（ack 前的等待窗口 state 恒 null，不可误判断连）。 */
    private var sawStream = false

    /** 最近一次已收口的流会话（stop/超时后 streamEnded 的 done 回声不得二次收口）。 */
    private var closedStreamConv: String? = null

    /** 按角色视图保存多会话列表（快照 conversations 域 + 本地新建/回复暂存）；null 键=管家 */
    private val history: MutableMap<String?, MutableList<ChatConversation>> = mutableMapOf()

    /** 各视图（null=管家）离开时的活跃会话 id：切回视图恢复原会话，不重置为最新。 */
    private val activeConversationIdByRole = mutableMapOf<String?, String>()

    /** 当前流式发送协程句柄（chat.send + 看门狗）：停止按钮（FR-33 Square）终止之 */
    private var streamJob: Job? = null

    /** 流式看门狗句柄：token 进展即重置。 */
    private var watchdogJob: Job? = null

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
        stream.onViewedConversation(current.id)
        // 13.3 T6：流式状态机由 STREAM_TOKEN 驱动（替代 mock 打字机）
        viewModelScope.launch {
            stream.state.collect { s -> onStreamState(s) }
        }
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
        stream.onViewedConversation(current.id)
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
        stream.onViewedConversation(current.id)
    }

    // ── 多会话：新建 / 切换 / 删除（移植桌面 ChatHeader 语义）────────────

    /** 新建会话：中断流式（同 selectRole），本地乐观占位 + `COMMAND(conversation.new)`
     *  对齐桌面 id（ack 前消息可发——chat.send 对本地占位会话不传 conversationId）。 */
    fun newConversation() {
        stopStreaming()
        streamJob?.cancel()
        streamJob = null
        val roleId = _uiState.value.activeRoleId
        val conversation = createConversationIn(roleId)
        localConversationIds.add(conversation.id)
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
                traceByMessageId = emptyMap(),
            )
        }
        stream.onViewedConversation(conversation.id)
        viewModelScope.launch {
            val sender = commands ?: return@launch // fake 态：保留本地占位（离线兜底）
            try {
                val result = sender.execute(
                    "conversation.new",
                    CommandEnvelope.buildParams(listOf("roleId" to roleId)),
                )
                val desktopId = result.optString("id")
                if (desktopId.isNotEmpty()) {
                    val adopted = adoptConversationId(conversation.id, desktopId, roleId)
                    if (!adopted) {
                        // 本地 id 已被 chat.send 采纳替换（新建后立即发消息的竞速）：
                        // conversation.new 落成的桌面会话成孤儿——删除回收，否则
                        // 桌面永久残留无入口的空会话（评审 C8）
                        runCatching {
                            sender.execute(
                                "conversation.delete",
                                CommandEnvelope.buildParams(listOf("conversationId" to desktopId)),
                            )
                        }
                    }
                }
            } catch (e: CommandException) {
                // 新建指令失败：本地占位保留（桌面无此会话，删除时无需通知），
                // 但不得静默——与 Tasks/Memory 显式失败方针一致（评审 C7）
                onError("新建会话失败：${e.message}")
            }
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
        stream.onViewedConversation(id)
    }

    /** 删除会话：仅移除；删当前则回退剩余 updatedAt 最新者，无剩余自动新建空会话。
     *  桌面对齐会话同步发 `COMMAND(conversation.delete)`（本地占位无需通知桌面）。 */
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
        val wasDesktopOwned = id !in localConversationIds
        list.removeAll { it.id == id }
        localConversationIds.remove(id)
        if (!isCurrent) {
            _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
            if (wasDesktopOwned) notifyConversationDeleted(id)
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
            stream.onViewedConversation(fallback.id)
        } else {
            newConversation()
        }
        if (wasDesktopOwned) notifyConversationDeleted(id)
    }

    private fun notifyConversationDeleted(id: String) {
        viewModelScope.launch {
            try {
                commands?.execute(
                    "conversation.delete",
                    CommandEnvelope.buildParams(listOf("conversationId" to id)),
                )
            } catch (e: CommandException) {
                onError("删除会话失败：${e.message}")
            }
        }
    }

    // ── 发送与流式回复（13.3 指令通道换装）────────────────────────────

    fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        // 并发守卫：上一条回复未完成（思考中/流式中）时忽略新发送，防流式交错
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

        val userMsgId = "u-${nextId++}"
        appendMessage(ChatMessage(userMsgId, false, trimmed))
        _uiState.update { it.copy(thinking = true, responding = true) }

        streamJob = viewModelScope.launch {
            val sender = commands ?: run {
                failSend("桌面引擎不可达")
                return@launch
            }
            // 本地占位会话不传 conversationId——桌面新建会话并回填真实 id
            val convIdParam = currentId.takeIf { it !in localConversationIds }
            try {
                val result = sender.execute(
                    "chat.send",
                    CommandEnvelope.buildParams(
                        listOf(
                            "conversationId" to convIdParam,
                            "roleId" to roleId,
                            "content" to trimmed,
                        ),
                    ),
                )
                val convId = result.optString("conversationId").ifEmpty { currentId }
                val userId = result.optString("userMessageId").ifEmpty { userMsgId }
                adoptChatIds(currentId, convId, userMsgId, userId, roleId)
                stream.streamStarting(convId)
                resetWatchdog(convId)
            } catch (e: CommandException) {
                failSend(e.message ?: "发送失败")
            }
        }
    }

    private fun failSend(message: String) {
        sawStream = false
        closedStreamConv = null
        watchdogJob?.cancel()
        _uiState.update { it.copy(thinking = false, responding = false, streamingToolTitle = null) }
        onError(message)
    }

    /** ack 三 id 对齐：本地临时 id 替换为桌面 id（快照替换时 LazyColumn key 稳定无闪烁）。 */
    private fun adoptChatIds(
        localConvId: String,
        desktopConvId: String,
        localUserId: String,
        desktopUserId: String,
        sendRoleId: String?,
    ) {
        // 归属角色用发送时捕获值（评审 C5）：ack 等待期间切换角色视图后，读当前
        // activeRoleId 会把会话挂错角色 history——本地占位会话永远不被替换。
        if (desktopConvId.isNotEmpty() && desktopConvId != localConvId) {
            adoptConversationId(localConvId, desktopConvId, sendRoleId)
        }
        if (desktopUserId.isNotEmpty() && desktopUserId != localUserId) {
            _uiState.update { state ->
                state.copy(
                    messages = state.messages.map { m ->
                        if (m.id == localUserId) m.copy(id = desktopUserId) else m
                    },
                )
            }
            persistCurrentMessages()
        }
    }

    /** 本地会话 id 替换为桌面 id（history/活跃会话记忆/当前指针同步）。
     *  返回是否采纳成功（本地 id 已被替换/不存在时 false——见 newConversation 孤儿回收）。 */
    private fun adoptConversationId(localId: String, desktopId: String, roleId: String?): Boolean {
        localConversationIds.remove(localId)
        val list = history[roleId] ?: return false
        val index = list.indexOfFirst { it.id == localId }
        if (index < 0) return false
        list[index] = list[index].copy(id = desktopId)
        if (activeConversationIdByRole[roleId] == localId) {
            activeConversationIdByRole[roleId] = desktopId
        }
        if (_uiState.value.currentConversationId == localId) {
            _uiState.update { it.copy(currentConversationId = desktopId) }
        }
        _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
        stream.onViewedConversation(desktopId)
        return true
    }

    // ── STREAM_TOKEN 驱动的流式状态机（13.3 T6）──────────────────────

    private fun onStreamState(s: StreamCoordinator.DesktopStream?) {
        val viewing = _uiState.value.currentConversationId
        if (s == null) {
            // 流态消失（断连/reset）：仅在已进入流式（非 ack 前等待窗口）时收口 + 反馈。
            // sawStream 不依赖查看会话（下方先置位再判 viewing）——流式中切走会话后
            // 断连同样收口，否则 responding 永久滞留锁死全局发送守卫（评审 C2）。
            if (_uiState.value.responding && sawStream) {
                sawStream = false
                finalizeStreamLocally()
                onError("连接中断，回复可能不完整")
            }
            return
        }
        sawStream = true
        // 非查看会话的流：不渲染（消息流属查看会话）；done 仍须收口（评审 C3）——
        // 段落直接落库该会话历史，否则流式中切走再切回，已流出文本凭空消失、
        // streamEnded 不调用、建议不现查。
        if (s.conversationId != viewing) {
            if (s.done) {
                watchdogJob?.cancel()
                closedStreamConv = s.conversationId
                sawStream = false
                streamSeq++
                val segMsgs = s.segments.mapIndexed { i, seg ->
                    ChatMessage(
                        id = seg.messageId ?: "stream-$streamSeq-$i",
                        fromButler = true,
                        text = seg.text,
                    )
                }
                appendStreamSegments(s.conversationId, segMsgs)
                _uiState.update {
                    it.copy(thinking = false, responding = false, streamingToolTitle = null)
                }
                stream.streamEnded(s.conversationId)
                // 建议不现查：actionCards 属当前查看会话——非查看会话的卡注入
                // 即跨会话污染（卡片态随会话隔离不变量）；用户切回后由快照/下次流补
            } else {
                resetWatchdog(s.conversationId)
            }
            return
        }
        renderStream(s)
    }

    private fun renderStream(s: StreamCoordinator.DesktopStream) {
        // 流式渲染基线：按会话捕获（切走再切回可继续）；新流递增序号——
        // 无 messageId 的段落兜底 id 须跨流唯一（同会话下一轮流式的基线已含
        // 旧 "stream-0"，固定后缀会撞 LazyColumn key，评审 C6）
        if (renderedStreamConv != s.conversationId) {
            renderedStreamConv = s.conversationId
            streamSeq++
            streamBase = _uiState.value.messages
        }
        // 停止/超时后的 done 回声：本流已本地收口，跳过二次收口（否则误触发建议现查）
        if (s.done && closedStreamConv == s.conversationId) return
        if (!s.done) closedStreamConv = null // 新流开启：清除上一轮回声记忆
        val base = streamBase.orEmpty()
        val roleId = _uiState.value.activeRoleId
        val segMsgs = s.segments.mapIndexed { i, seg ->
            ChatMessage(
                id = seg.messageId ?: "stream-$streamSeq-$i",
                fromButler = true,
                text = seg.text,
                streaming = !s.done && i == s.segments.lastIndex && !seg.sealed,
                // 委派第二段角色头像：真实流无 roleId 信号——角色视图挂当前角色，管家视图为 null
                senderRoleId = roleId,
            )
        }
        _uiState.update {
            it.copy(
                messages = base + segMsgs,
                thinking = s.thinking,
                responding = !s.done,
                streamingToolTitle = if (s.done) null else s.toolTitle,
            )
        }
        if (s.done) {
            watchdogJob?.cancel()
            closedStreamConv = s.conversationId
            sawStream = false
            val lastId = segMsgs.lastOrNull()?.id
            if (s.traceBlocks.isNotEmpty() && lastId != null) {
                _uiState.update {
                    it.copy(traceByMessageId = it.traceByMessageId + (lastId to s.traceBlocks))
                }
            }
            persistCurrentMessages()
            renderedStreamConv = null
            streamBase = null
            stream.streamEnded(s.conversationId)
            // 流结束后现查 pending 建议（快照无 suggestions 域——UX-M5 手法）
            loadSuggestions(s.conversationId)
        } else {
            resetWatchdog(s.conversationId)
        }
    }

    /** 流结束/失败后的本地收口（保留已浮现文本，清进行态）。 */
    private fun finalizeStreamLocally() {
        watchdogJob?.cancel()
        streamJob = null
        sawStream = false
        closedStreamConv = renderedStreamConv
        if (renderedStreamConv != null) {
            // 有已渲染流（查看会话）：文本落定 + 回写
            _uiState.update {
                it.copy(
                    messages = it.messages.map { m -> if (m.streaming) m.copy(streaming = false) else m },
                    thinking = false,
                    responding = false,
                    streamingToolTitle = null,
                )
            }
            persistCurrentMessages()
        } else {
            // 未渲染流（切走会话期间流式/看门狗超时）：只清全局进行态——
            // 不得触碰查看会话的消息与 updatedAt（评审 C4：看门狗误伤查看会话）
            _uiState.update {
                it.copy(thinking = false, responding = false, streamingToolTitle = null)
            }
        }
        renderedStreamConv = null
        streamBase = null
    }

    /** 流式看门狗：120s 无 token 进展 → 错误态 + 可重试（不悬挂）。 */
    private fun resetWatchdog(conversationId: String) {
        watchdogJob?.cancel()
        watchdogJob = viewModelScope.launch {
            delay(WATCHDOG_TIMEOUT_MS)
            val s = stream.state.value
            if (s != null && !s.done && s.conversationId == conversationId && _uiState.value.responding) {
                stream.streamEnded(conversationId)
                finalizeStreamLocally()
                onError("回复超时（${WATCHDOG_TIMEOUT_MS / 1000}s 无进展），请重试")
            }
        }
    }

    /** 流结束后现查当前会话 pending 建议（存在才渲染；查询失败静默——不阻塞对话）。 */
    private fun loadSuggestions(conversationId: String) {
        val sender = commands ?: return
        viewModelScope.launch {
            try {
                val result = sender.execute(
                    "suggestion.list",
                    CommandEnvelope.buildParams(listOf("conversationId" to conversationId)),
                )
                val arr = result.optJSONArray("suggestions") ?: return@launch
                val cards = (0 until arr.length()).mapNotNull { i ->
                    arr.optJSONObject(i)?.toActionCard()
                }
                if (cards.isNotEmpty()) {
                    _uiState.update { it.copy(actionCards = cards) }
                }
            } catch (_: CommandException) {
                // 建议查询失败不阻塞对话（下次流结束再查）
            }
        }
    }

    private fun JSONObject.toActionCard(): ActionCardSuggestion? {
        val id = optString("id")
        if (id.isEmpty()) return null
        return ActionCardSuggestion(
            id = id,
            title = optString("title"),
            detail = optString("content"),
            fromRole = optString("roleName").ifEmpty { "角色" },
        )
    }

    // ── FR-33 停止 ────────────────────────────────────────────────────

    /** 停止流式回复（镜像桌面 ChatInput 停止，Square 图标）：发 `COMMAND(chat.stop)`，
     *  已浮现内容落定（不回滚——桌面侧亦保留已生成内容）。 */
    fun stopStreaming() {
        val convId = renderedStreamConv ?: _uiState.value.currentConversationId
        if (_uiState.value.responding) {
            // 停止指令（失败不打断本地收口——桌面侧流本就面向本次会话）
            viewModelScope.launch {
                try {
                    commands?.execute(
                        "chat.stop",
                        CommandEnvelope.buildParams(listOf("conversationId" to convId)),
                    )
                } catch (_: CommandException) {
                }
            }
        }
        // 先掐发送协程再收口（评审 C1）：ack 等待窗口点停止后，存活的 chat.send
        // 协程会在 ack 返回时继续 streamStarting + 重臂看门狗——已停止的流「复活」
        streamJob?.cancel()
        streamJob = null
        finalizeStreamLocally()
        stream.streamEnded(convId)
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

        // 13.3 T7：确认/拒绝经桌面执行（ack 后置终态——失败保持 PENDING 可重试，
        // 不伪造回执）；拒绝 reason 固定文案（手机无 reason 输入框）。
        viewModelScope.launch {
            val sender = commands ?: run {
                onError("桌面引擎不可达")
                return@launch
            }
            try {
                if (confirmed) {
                    sender.execute(
                        "suggestion.confirm",
                        CommandEnvelope.buildParams(listOf("suggestionId" to cardId)),
                    )
                } else {
                    sender.execute(
                        "suggestion.reject",
                        CommandEnvelope.buildParams(
                            listOf("suggestionId" to cardId, "reason" to REJECT_REASON),
                        ),
                    )
                }
            } catch (e: CommandException) {
                onError("建议${if (confirmed) "确认" else "拒绝"}失败：${e.message}")
                return@launch
            }
            // ack 成功：终态翻转 + 管家回执（UX-M1：文案与既有「✓ 已确认 · 已转交管家执行」零改动）
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
                        // 建议来源角色随卡片数据（桌面 SuggestionWithRole.roleName）——
                        // 回执不得硬编码「产品经理」与真实来源错位（评审 C9）
                        text = if (confirmed) {
                            "好的，已确认。我这就转给${target.fromRole}去办，完成后第一时间告诉您。"
                        } else {
                            "明白，那这条建议就先放下了。需要时随时叫我。"
                        },
                    ),
                )
            }
            persistCurrentMessages()
        }
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
        persistMessages(currentId, roleId, _uiState.value.messages)
        // 同步刷新会话列表快照：updatedAt 变化影响「最新在前」排序与相对时间显示
        _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
    }

    /** 消息写入指定会话（不依赖当前查看指针——非查看会话的 done 收口路径）。 */
    private fun persistMessages(conversationId: String, roleId: String?, messages: List<ChatMessage>) {
        val list = history[roleId] ?: return
        val index = list.indexOfFirst { it.id == conversationId }
        if (index < 0) return
        list[index] = list[index].copy(
            messages = messages,
            updatedAt = System.currentTimeMillis(),
        )
    }

    /** 段落追加进所属会话（跨 history 检索——管家视图键为 null，不可作未找到哨兵）。 */
    private fun appendStreamSegments(conversationId: String, segments: List<ChatMessage>) {
        history.entries.forEach { (roleId, list) ->
            val index = list.indexOfFirst { it.id == conversationId }
            if (index >= 0) {
                persistMessages(conversationId, roleId, list[index].messages + segments)
                _uiState.update { it.copy(conversations = conversationsOf(roleId)) }
                return
            }
        }
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

    private companion object {
        /** 流式看门狗：120s 无 token 进展即超时（对齐桌面流式节奏上限，Dev Notes §4）。 */
        const val WATCHDOG_TIMEOUT_MS = 120_000L

        /** 拒绝建议固定原因（手机无 reason 输入框；非空即通过桌面校验）。 */
        const val REJECT_REASON = "手机端暂不需要"
    }
}
