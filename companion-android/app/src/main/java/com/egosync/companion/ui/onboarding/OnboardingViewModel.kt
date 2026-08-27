package com.egosync.companion.ui.onboarding

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.AppModelContainer
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.RoleProposal
import com.egosync.companion.sync.RoleProposalState
import com.egosync.companion.sync.SnapshotStore
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class OnboardingUiState(
    val messages: List<ChatMessage> = emptyList(),
    /** 管家思考中（用户消息后、回复开始前）。 */
    val thinking: Boolean = false,
    /** 流式打字机进行中（发送/跳过守卫）。 */
    val responding: Boolean = false,
    /** 当前引导步（1..MAX_STEP，镜像桌面 useState(1) 步进）。 */
    val step: Int = 1,
    /** FR-5 角色涌现提案卡；null=未浮现 */
    val roleProposal: RoleProposal? = null,
) {
    companion object {
        const val MAX_STEP = 5

        /** 第几轮完整回复后浮现角色涌现提案卡。 */
        const val PROPOSAL_ROUND = 2

        /** 步进递增并在第 5 步封顶（镜像桌面 Math.min(step + 1, 5)）。 */
        fun nextStep(current: Int): Int = (current + 1).coerceAtMost(MAX_STEP)

        /** 步进到第 5 步即标记引导完成（镜像桌面 L215：防杀进程后重复引导）。 */
        fun marksCompletion(nextStep: Int): Boolean = nextStep >= MAX_STEP

        /** 「跳过角色引导」链接第 2 步起可见（镜像桌面 L362 step >= 2）。 */
        fun skipVisible(step: Int): Boolean = step >= 2

        /** 输入 placeholder 随步切换。索引直接用 step（镜像桌面 min(step, len-1)）：
         *  首条 placeholder 桌面本身不展示（step 从 1 起），逐字镜像而非“修正”。 */
        fun placeholderFor(step: Int): String =
            SnapshotStore.onboardingPlaceholders[step.coerceIn(0, SnapshotStore.onboardingPlaceholders.lastIndex)]
    }
}

/**
 * FR-21 空状态引导 mock 状态机：发送 → 思考 → 打字机回复（镜像桌面 OnboardingView llm:stream 语义）。
 * 第 2 轮回复后浮现角色涌现提案卡（FR-5）；完成路径三条（镜像桌面）：
 * 确认创建（800ms 后进主界面）/「跳过角色引导」/第 5 步标记（留在屏上）。
 */
class OnboardingViewModel(private val container: AppModelContainer) : ViewModel() {

    private val _uiState = MutableStateFlow(
        OnboardingUiState(messages = listOf(greetingMessage()))
    )
    val uiState: StateFlow<OnboardingUiState> = _uiState.asStateFlow()

    private var replyIndex = 0
    private var completedRounds = 0
    private var nextId = 0

    // ── 发送与流式回复 ────────────────────────────────────────────────

    fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        // 并发守卫：流式回复未完成时忽略新发送（镜像桌面 input disabled）
        if (_uiState.value.thinking || _uiState.value.responding) return

        val next = OnboardingUiState.nextStep(_uiState.value.step)
        _uiState.update { it.copy(step = next, messages = it.messages + userMessage(trimmed)) }

        // 桌面 L213-217：第 5 步是引导最后一步，无论是否创建角色都标记完成，
        // 避免下次打开应用重复引导（留在屏上可继续对话）
        if (OnboardingUiState.marksCompletion(next)) {
            container.completeOnboarding()
        }

        viewModelScope.launch { runReply() }
    }

    private suspend fun runReply() {
        _uiState.update { it.copy(thinking = true, responding = true) }
        delay(700)
        _uiState.update { it.copy(thinking = false) }

        val replies = container.snapshotStore.onboardingReplies
        val full = replies[replyIndex % replies.size]
        replyIndex++

        val msgId = streamTypewriter(full)
        _uiState.update { it.copy(responding = false) }
        if (msgId == null) return // 已被取消中断：不计轮次、不触发提案

        completedRounds++
        // FR-5：第 2 轮完整回复后浮现角色涌现提案卡（一次性守卫）
        if (completedRounds == OnboardingUiState.PROPOSAL_ROUND && _uiState.value.roleProposal == null) {
            _uiState.update { it.copy(roleProposal = container.snapshotStore.roleProposal) }
        }
    }

    /** 打字机流式输出，返回消息 id；被取消中断时返回 null（ChatViewModel 同款模式）。 */
    private suspend fun streamTypewriter(full: String): String? {
        currentCoroutineContext().ensureActive()
        val id = "ob-${nextId++}"
        appendMessage(ChatMessage(id, true, "", streaming = true))
        val builder = StringBuilder()
        for (ch in full) {
            builder.append(ch)
            val acc = builder.toString()
            currentCoroutineContext().ensureActive()
            _uiState.update { state ->
                state.copy(messages = state.messages.map {
                    if (it.id == id) it.copy(text = acc, streaming = true) else it
                })
            }
            delay(24)
        }
        _uiState.update { state ->
            state.copy(messages = state.messages.map {
                if (it.id == id) it.copy(streaming = false) else it
            })
        }
        return id
    }

    // ── 三条完成路径 ──────────────────────────────────────────────────

    /** 路径一：提案确认创建（镜像桌面 handleProposalConfirm：创建成功 800ms 后进主界面）。 */
    fun confirmRoleProposal(
        name: String,
        icon: String,
        color: String,
        goal: String,
        onCompleted: () -> Unit,
    ) {
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
                    id = "ob-${nextId++}",
                    fromButler = true,
                    text = "好的，已创建角色「$name」。以后这类事可以交给它跟进。",
                ),
            )
        }
        // 桌面 L123-124：先落完成标记、再等 800ms 进主界面——顺序颠倒会让 800ms 窗口内
        // 杀进程丢标记（viewModelScope 随之取消），下次启动重新引导
        container.completeOnboarding()
        viewModelScope.launch {
            delay(800) // 桌面 setTimeout(onComplete, 800)
            onCompleted()
        }
    }

    /** 提案「不需要」：卡置已跳过态 + 管家回执，引导继续（镜像桌面 onCancel 汇一）。 */
    fun skipRoleProposal() {
        if (_uiState.value.thinking || _uiState.value.responding) return
        val proposal = _uiState.value.roleProposal ?: return
        if (proposal.state != RoleProposalState.PENDING) return

        _uiState.update {
            it.copy(
                roleProposal = proposal.copy(state = RoleProposalState.SKIPPED),
                messages = it.messages + ChatMessage(
                    id = "ob-${nextId++}",
                    fromButler = true,
                    text = "明白，这个角色先不创建。需要时随时跟我说。",
                ),
            )
        }
    }

    /** 路径二：「跳过角色引导」——立即完成并进主界面（镜像桌面 handleSkipOnboarding）。 */
    fun skipOnboarding(onComplete: () -> Unit) {
        if (_uiState.value.thinking || _uiState.value.responding) return
        container.completeOnboarding()
        onComplete()
    }

    // ── 内部工具 ──────────────────────────────────────────────────────

    private fun greetingMessage(): ChatMessage = ChatMessage(
        id = "ob-greeting",
        fromButler = true,
        text = container.snapshotStore.onboardingGreeting,
    )

    private fun userMessage(text: String): ChatMessage = ChatMessage(
        id = "ob-${nextId++}",
        fromButler = false,
        text = text,
    )

    private fun appendMessage(message: ChatMessage) {
        _uiState.update { it.copy(messages = it.messages + message) }
    }
}
