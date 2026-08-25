package com.egosync.companion.ui.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.egosync.companion.sync.ActionCardSuggestion
import com.egosync.companion.sync.ActionCardState
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.SnapshotStore
import java.util.UUID
import kotlinx.coroutines.delay
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
 * 管家对话 mock 状态机：发送 → 思考中 → 流式打字机回复（镜像 llm:stream 语义）。
 * 接入真实连接层后：发送改为 COMMAND 帧，回复改为 STREAM_TOKEN 帧驱动。
 */
class ChatViewModel : ViewModel() {

    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private var replyIndex = 0
    private var userCount = 0
    private var nextId = 100

    fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        userCount++

        _uiState.update { it.copy(messages = it.messages + ChatMessage(nextId++.toString(), false, trimmed)) }

        viewModelScope.launch {
            _uiState.update { it.copy(thinking = true) }
            delay(900)
            _uiState.update { it.copy(thinking = false) }

            // 流式打字机：逐字浮现（原型模拟 STREAM_TOKEN 帧）
            val full = SnapshotStore.butlerReplies[replyIndex % SnapshotStore.butlerReplies.size]
            replyIndex++
            val msgId = nextId++.toString()
            _uiState.update {
                it.copy(messages = it.messages + ChatMessage(msgId, true, "", streaming = true))
            }
            val builder = StringBuilder()
            for (ch in full) {
                builder.append(ch)
                val acc = builder.toString()
                _uiState.update { state ->
                    state.copy(messages = state.messages.map {
                        if (it.id == msgId) it.copy(text = acc, streaming = true) else it
                    })
                }
                delay(24)
            }
            _uiState.update { state ->
                state.copy(messages = state.messages.map {
                    if (it.id == msgId) it.copy(streaming = false) else it
                })
            }

            // 第二轮对话后浮现一张待确认建议卡（FR-11/12 ActionCard）
            if (userCount == 2 && _uiState.value.actionCards.isEmpty()) {
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
        }
    }

    fun respondActionCard(cardId: String, confirmed: Boolean) {
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
    }
}
