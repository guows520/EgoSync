package com.egosync.companion.ui.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.ActionCardSuggestion
import com.egosync.companion.sync.ActionCardState
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.EgoSyncTheme

/**
 * ① 管家对话 Tab：消息流（用户/管家气泡、思考中态、流式打字机）、
 * 输入框、待确认建议 ActionCard（确认/拒绝按钮态）。
 */
@Composable
fun ChatScreen(
    uiState: ChatUiState,
    engineAvailable: Boolean,
    onSendMessage: (String) -> Unit,
    onActionCardRespond: (cardId: String, confirmed: Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    var input by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val totalItems = uiState.messages.size + uiState.actionCards.size

    // 新消息时跟随滚动到底部
    LaunchedEffect(totalItems, uiState.messages.lastOrNull()?.text) {
        if (totalItems > 0) listState.animateScrollToItem(totalItems - 1)
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .imePadding(),
    ) {
        LazyColumn(
            state = listState,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(uiState.messages, key = { it.id }) { message ->
                MessageBubble(message)
            }
            if (uiState.thinking) {
                item(key = "thinking") { ThinkingBubble() }
            }
            items(uiState.actionCards, key = { it.id }) { card ->
                ActionCard(
                    card = card,
                    enabled = engineAvailable,
                    onRespond = { confirmed -> onActionCardRespond(card.id, confirmed) },
                )
            }
            item { Spacer(Modifier.size(6.dp)) }
        }

        // 输入条
        Surface(color = MaterialTheme.colorScheme.surface) {
            Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 10.dp)) {
                if (!engineAvailable) {
                    Text(
                        "桌面引擎离线，对话暂不可用（依赖引擎的功能已禁用）",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
                    )
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = { input = it },
                        placeholder = { Text("对管家说点什么…") },
                        modifier = Modifier.weight(1f),
                        enabled = engineAvailable,
                        maxLines = 3,
                        shape = RoundedCornerShape(14.dp),
                    )
                    Spacer(Modifier.size(10.dp))
                    Button(
                        onClick = {
                            onSendMessage(input)
                            input = ""
                        },
                        enabled = engineAvailable && input.isNotBlank(),
                    ) {
                        Text("发送")
                    }
                }
            }
        }
    }
}

// ── 消息气泡 ───────────────────────────────────────────────────────────

@Composable
private fun MessageBubble(message: ChatMessage) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (message.fromButler) Arrangement.Start else Arrangement.End,
    ) {
        if (message.fromButler) {
            Box(
                Modifier
                    .size(30.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Text("🤵", style = MaterialTheme.typography.bodySmall)
            }
            Spacer(Modifier.size(8.dp))
        }
        Surface(
            color = if (message.fromButler) MaterialTheme.colorScheme.surfaceVariant
            else MaterialTheme.colorScheme.primaryContainer,
            shape = RoundedCornerShape(
                topStart = 16.dp,
                topEnd = 16.dp,
                bottomStart = if (message.fromButler) 4.dp else 16.dp,
                bottomEnd = if (message.fromButler) 16.dp else 4.dp,
            ),
            modifier = Modifier.widthIn(max = 290.dp),
        ) {
            Text(
                text = if (message.streaming) "${message.text}▍" else message.text,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
            )
        }
    }
}

@Composable
private fun ThinkingBubble() {
    Row {
        Box(
            Modifier
                .size(30.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primaryContainer),
            contentAlignment = Alignment.Center,
        ) {
            Text("🤵", style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.size(8.dp))
        Surface(
            color = MaterialTheme.colorScheme.surfaceVariant,
            shape = RoundedCornerShape(16.dp, 16.dp, 4.dp, 16.dp),
        ) {
            Text(
                "管家正在思考…",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
            )
        }
    }
}

// ── 建议 ActionCard（确认/拒绝按钮态）──────────────────────────────────

@Composable
fun ActionCard(
    card: ActionCardSuggestion,
    enabled: Boolean,
    onRespond: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = modifier.fillMaxWidth(),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
    ) {
        Column(Modifier.padding(14.dp)) {
            Text(
                "建议 · ${card.fromRole}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.size(6.dp))
            Text(card.title, style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.size(4.dp))
            Text(
                card.detail,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(12.dp))
            when (card.state) {
                ActionCardState.PENDING -> {
                    Row {
                        Button(
                            onClick = { onRespond(true) },
                            enabled = enabled,
                            colors = ButtonDefaults.buttonColors(containerColor = BrandGreen),
                        ) { Text("确认") }
                        Spacer(Modifier.size(10.dp))
                        OutlinedButton(onClick = { onRespond(false) }, enabled = enabled) {
                            Text("拒绝")
                        }
                    }
                    if (!enabled) {
                        Spacer(Modifier.size(4.dp))
                        Text(
                            "桌面引擎离线，暂无法确认建议",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                ActionCardState.CONFIRMED -> Text(
                    "✓ 已确认 · 已转交管家执行",
                    style = MaterialTheme.typography.labelMedium,
                    color = BrandGreen,
                )
                ActionCardState.REJECTED -> Text(
                    "已拒绝 · 建议已放下",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun ChatScreenPreview() {
    EgoSyncTheme {
        ChatScreen(
            uiState = ChatUiState.sample(),
            engineAvailable = true,
            onSendMessage = {},
            onActionCardRespond = { _, _ -> },
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun ChatScreenStreamingPreview() {
    EgoSyncTheme {
        ChatScreen(
            uiState = ChatUiState(
                messages = listOf(
                    SnapshotInitial,
                    ChatMessage("u1", false, "今天下午都有什么安排？"),
                    ChatMessage("b2", true, "下午 2 点是产品评审，材料已备好。四点", streaming = true),
                ),
                thinking = false,
                actionCards = emptyList(),
            ),
            engineAvailable = true,
            onSendMessage = {},
            onActionCardRespond = { _, _ -> },
        )
    }
}

private val SnapshotInitial = ChatMessage(
    id = "m-1",
    fromButler = true,
    text = "早上好 boss。今天最重要的一件事是下午 2 点的产品评审，材料产品经理已经备好了。另外，女儿钢琴课在四点半，我提前提醒您。",
)
