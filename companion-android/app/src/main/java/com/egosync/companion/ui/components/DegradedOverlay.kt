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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
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
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.theme.EgoSyncTheme
import kotlinx.coroutines.flow.collectLatest

/**
 * 降级态遮罩（FR-43）：
 * 灰色半透明蒙层（拦截交互 = 引擎功能禁用）+ 数据截止时间标注 +
 * 引擎功能禁用说明 + 底部速记输入条（唯一持续开放入口）+ 低强调
 * 「解除配对并重新扫码」受控出口（不可自愈失配的应用内唯一出路，
 * 破坏性动作需 AlertDialog 二次确认）。
 *
 * 只读缓存在蒙层下仍然可见（诚实标注，不以陈旧数据冒充实时的）。
 */
@Composable
fun DegradedOverlayHost(
    state: ConnectionState,
    quickNotes: QuickNoteQueue,
    onRePair: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (state !is ConnectionState.Offline) return

    var noteText by remember { mutableStateOf("") }
    var pendingCount by remember { mutableStateOf(quickNotes.pendingCount()) }
    var showRePairDialog by rememberSaveable { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        quickNotes.items.collectLatest { pendingCount = it.count { n -> !n.submitted } }
    }

    if (showRePairDialog) {
        AlertDialog(
            onDismissRequest = { showRePairDialog = false },
            title = { Text("解除配对？") },
            text = {
                Text("解除后将清除本机缓存与配对信息，需重新扫码才能再次连接桌面端。")
            },
            confirmButton = {
                TextButton(onClick = { showRePairDialog = false; onRePair() }) {
                    Text("解除配对", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showRePairDialog = false }) { Text("取消") }
            },
        )
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
            Icon(
                LucideIcons.WifiOff,
                contentDescription = "离线",
                modifier = Modifier.size(32.dp),
                tint = Color(0xFFE8E8ED),
            )
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
            // 受控出口（低强调、两态均可见）：自动重连无法自救的失配（如桌面
            // 重装换信任锚）下唯一应用内出路——二次确认后解除配对重扫（FR-40）
            Spacer(Modifier.height(16.dp))
            TextButton(onClick = { showRePairDialog = true }) {
                Text("连接不上？解除配对并重新扫码", color = Color(0xFF9CA3AF))
            }
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
                    .clip(RoundedCornerShape(24.dp))
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
                onRePair = {},
            )
        }
    }
}
