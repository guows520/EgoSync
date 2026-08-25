package com.egosync.companion.ui.tasks

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.color

/**
 * ② 任务 Tab：四象限分组列表（Q1~Q4 分色、大石头星标、勾选完成交互）。
 */
@Composable
fun TasksScreen(
    uiState: TasksUiState,
    engineAvailable: Boolean,
    onToggleTask: (taskId: String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val grouped = uiState.grouped()

    Column(modifier = modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                horizontal = 16.dp, vertical = 12.dp,
            ),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (!engineAvailable) {
                item(key = "offline-hint") {
                    Text(
                        "桌面引擎离线：任务操作需桌面引擎在线，暂不可交互。",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Quadrant.entries.forEach { quadrant ->
                val tasks = grouped[quadrant].orEmpty()
                if (tasks.isNotEmpty()) {
                    item(key = quadrant.code) {
                        QuadrantHeader(quadrant, tasks.size)
                    }
                    items(tasks.size, key = { tasks[it].id }) { index ->
                        TaskRow(
                            task = tasks[index],
                            enabled = engineAvailable,
                            onToggle = { onToggleTask(tasks[index].id) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun QuadrantHeader(quadrant: Quadrant, count: Int) {
    val color = quadrant.color()
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(color.copy(alpha = 0.12f))
            .padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        Box(
            Modifier
                .size(10.dp)
                .clip(CircleShape)
                .background(color)
        )
        Spacer(Modifier.size(10.dp))
        Text(
            "${quadrant.code} ${quadrant.title}",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.size(8.dp))
        Text(
            quadrant.subtitle,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.weight(1f))
        Text(
            "$count 项",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun TaskRow(
    task: TaskItem,
    enabled: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(14.dp),
        modifier = modifier
            .fillMaxWidth()
            .then(if (enabled) Modifier.clickable(onClick = onToggle) else Modifier),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // 勾选圆圈
            Box(
                Modifier
                    .size(22.dp)
                    .clip(CircleShape)
                    .then(
                        if (task.done) Modifier.background(Quadrant.Q2.color())
                        else Modifier.border(1.5.dp, MaterialTheme.colorScheme.outline, CircleShape)
                    ),
                contentAlignment = Alignment.Center,
            ) {
                if (task.done) {
                    Text(
                        "✓",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
            }
            Spacer(Modifier.size(12.dp))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        task.title,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (task.done) MaterialTheme.colorScheme.onSurfaceVariant
                        else MaterialTheme.colorScheme.onSurface,
                        textDecoration = if (task.done) TextDecoration.LineThrough else null,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false),
                    )
                    if (task.bigRock) {
                        Spacer(Modifier.size(6.dp))
                        Text("🪨", style = MaterialTheme.typography.bodySmall)
                    }
                }
                Spacer(Modifier.size(3.dp))
                Row {
                    Text(
                        task.roleName,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    task.due?.let {
                        Spacer(Modifier.size(10.dp))
                        Text(
                            it,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun TasksScreenPreview() {
    EgoSyncTheme {
        TasksScreen(
            uiState = TasksUiState.sample(),
            engineAvailable = true,
            onToggleTask = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun TaskRowPreview() {
    EgoSyncTheme {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            TaskRow(
                task = SnapshotStore.tasks.first(),
                enabled = true,
                onToggle = {},
            )
            TaskRow(
                task = SnapshotStore.tasks.first().copy(done = true),
                enabled = true,
                onToggle = {},
            )
        }
    }
}
