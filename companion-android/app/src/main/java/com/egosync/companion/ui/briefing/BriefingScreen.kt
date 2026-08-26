package com.egosync.companion.ui.briefing

import com.egosync.companion.ui.icons.LucideIcons
import androidx.compose.ui.Alignment

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.BriefingActionPoint
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.EgoSyncTheme

/**
 * 二级页 · 晨间简报（FR-16 移动呈现）：分节自然语言文本 + 可操作行动点卡片。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BriefingScreen(
    onBack: () -> Unit,
    onRespondActionPoint: (pointId: String, confirmed: Boolean) -> Unit = { _, _ -> },
    modifier: Modifier = Modifier,
) {
    val briefing = SnapshotStore.briefing
    // 行动点决策的本地状态（mock）：confirmed/null→待处理
    val decided = remember { mutableStateMapOf<String, Boolean>() }

    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("晨间简报") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
        ) {
            Text(
                briefing.date,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 4.dp),
            )
            Spacer(Modifier.height(10.dp))

            // 问候段（管家语气）
            Surface(
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(10.dp),
            ) {
                Row(Modifier.padding(14.dp)) {
                    Icon(LucideIcons.ConciergeBell, contentDescription = "管家", modifier = Modifier.size(24.dp))
                    Spacer(Modifier.size(12.dp))
                    Text(
                        briefing.greeting,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // 分节内容
            briefing.sections.forEach { section ->
                Text(
                    section.title,
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
                )
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(10.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        section.body,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(14.dp),
                    )
                }
                Spacer(Modifier.height(16.dp))
            }

            // 行动点卡片（可直接确认/拒绝，无需切到角色）
            Text(
                "今天可以顺手定下的事",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
            )
            briefing.actionPoints.forEach { point ->
                val state = if (decided.containsKey(point.id)) decided[point.id] else null
                ActionPointCard(point.copy(confirmed = state)) { confirmed ->
                    decided[point.id] = confirmed
                    onRespondActionPoint(point.id, confirmed)
                }
                Spacer(Modifier.height(10.dp))
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun ActionPointCard(point: BriefingActionPoint, onRespond: (Boolean) -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp)) {
            Text(
                "建议 · ${point.fromRole}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.height(6.dp))
            Text(point.text, style = MaterialTheme.typography.bodyMedium)
            Spacer(Modifier.height(10.dp))
            when (point.confirmed) {
                null -> {
                    Row {
                        Button(onClick = { onRespond(true) }) { Text("确认") }
                        Spacer(Modifier.size(10.dp))
                        OutlinedButton(onClick = { onRespond(false) }) { Text("稍后") }
                    }
                }
                true -> Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(16.dp), tint = BrandGreen)
                    Spacer(Modifier.size(4.dp))
                    Text("已确认", style = MaterialTheme.typography.labelMedium, color = BrandGreen)
                }
                false -> Text(
                    "已放下",
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
private fun BriefingScreenPreview() {
    EgoSyncTheme {
        BriefingScreen(onBack = {})
    }
}
