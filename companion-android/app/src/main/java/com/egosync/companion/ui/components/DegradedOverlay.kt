package com.egosync.companion.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.connection.ConnectionState
import com.egosync.companion.sync.QuickNoteQueue
import com.egosync.companion.ui.theme.EgoSyncTheme
import kotlinx.coroutines.flow.collectLatest

/**
 * 降级态遮罩（FR-43）：
 * 灰色半透明蒙层（拦截交互 = 引擎功能禁用）+ 数据截止时间标注 +
 * 引擎功能禁用说明 + 底部速记输入条（唯一可用入口）。
 *
 * 只读缓存在蒙层下仍然可见（诚实标注，不以陈旧数据冒充实时的）。
 */
@Composable
fun DegradedOverlayHost(
    state: ConnectionState,
    quickNotes: QuickNoteQueue,
    modifier: Modifier = Modifier,
) {
    if (state !is ConnectionState.Offline) return

    var noteText by remember { mutableStateOf("") }
    var pendingCount by remember { mutableStateOf(quickNotes.pendingCount()) }

    LaunchedEffect(Unit) {
        quickNotes.items.collectLatest { pendingCount = it.count { n -> !n.submitted } }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color(0xFF0F1117).copy(alpha = 0.72f))
            // 降级锁定：消费全部指针事件，杜绝点穿蒙层操作底层引擎功能（FR-43）
            .pointerInput(Unit) {
                awaitEachGesture {
                    while (true) {
                        val event = awaitPointerEvent()
                        event.changes.forEach { it.consume() }
                        if (!event.changes.any { it.pressed }) break
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Spacer(Modifier.height(24.dp))
            Text("📴", style = MaterialTheme.typography.displaySmall)
            Spacer(Modifier.height(12.dp))
            Text(
                if (state.snapshotAvailable) "降级模式 · 只读缓存" else "离线 · 暂无缓存",
                style = MaterialTheme.typography.titleLarge,
                color = Color(0xFFE8E8ED),
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                if (state.snapshotAvailable) {
                    "数据截至 ${state.dataAsOf.orEmpty()}，展示的是最后已知快照。\n依赖引擎的功能（对话、任务操作、建议确认）暂不可用。"
                } else {
                    "与桌面引擎的连接已断开，且本地没有可用快照。\n恢复连接后将自动补齐最新状态。"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = Color(0xFF9CA3AF),
                textAlign = TextAlign.Center,
            )
        }

        // 底部速记输入条（降级态唯一开放入口，FR-43）
        Column(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .navigationBarsPadding()
                .imePadding(),
        ) {
            if (pendingCount > 0) {
                Text(
                    "速记队列：$pendingCount 条待同步（恢复连接后自动提交管家）",
                    style = MaterialTheme.typography.labelSmall,
                    color = Color(0xFF9CA3AF),
                    modifier = Modifier.padding(bottom = 6.dp, start = 4.dp),
                )
            }
            androidx.compose.foundation.layout.Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(14.dp))
                    .background(Color(0xFF252638))
                    .padding(horizontal = 6.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = noteText,
                    onValueChange = { noteText = it },
                    placeholder = {
                        Text("速记一条，恢复连接后交管家处理…", style = MaterialTheme.typography.bodyMedium)
                    },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    colors = OutlinedTextFieldDefaults.colors(
                        unfocusedContainerColor = Color.Transparent,
                        focusedContainerColor = Color.Transparent,
                        unfocusedBorderColor = Color.Transparent,
                        focusedBorderColor = Color.Transparent,
                    ),
                )
                Button(
                    onClick = {
                        quickNotes.submit(noteText)
                        noteText = ""
                    },
                    enabled = noteText.isNotBlank(),
                ) {
                    Text("记下")
                }
            }
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true, backgroundColor = 0xFF0F1117)
@Composable
private fun DegradedOverlayPreview() {
    EgoSyncTheme {
        Box(Modifier.fillMaxSize()) {
            Text("缓存内容在蒙层下仍可阅读", Modifier.align(Alignment.Center))
            DegradedOverlayHost(
                state = ConnectionState.Offline(snapshotAvailable = true, dataAsOf = "今天 08:15"),
                quickNotes = QuickNoteQueue(),
            )
        }
    }
}
