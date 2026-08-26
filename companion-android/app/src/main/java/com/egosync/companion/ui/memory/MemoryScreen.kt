package com.egosync.companion.ui.memory

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ButtonDefaults
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.MemoryCategory
import com.egosync.companion.sync.MemoryItem
import com.egosync.companion.sync.MemorySourceMessage
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.formatMemoryTime
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.accent

/**
 * 二级页 · 记忆（FR-8 查询溯源 / FR-9 选择性遗忘）：镜像桌面 MemoryTab。
 * 类别筛选 chip + 记忆卡（来源展开区 + 遗忘确认面板）；按角色进入（roleId 来自导航参数）。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MemoryScreen(
    uiState: MemoryUiState,
    onBack: () -> Unit,
    onCategorySelected: (MemoryCategory?) -> Unit,
    onToggleSource: (memoryId: String) -> Unit,
    onOpenForgetConfirm: (memoryId: String) -> Unit,
    onCancelForget: (memoryId: String) -> Unit,
    onConfirmForget: (memoryId: String) -> Unit,
    modifier: Modifier = Modifier,
) {
    // 色温随角色域（桌面 --role-accent 同语义）
    val roleAccent = uiState.role?.domain?.accent()?.accent ?: MaterialTheme.colorScheme.primary
    val visible = uiState.visibleMemories

    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text(uiState.role?.let { "记忆 · ${it.name}" } ?: "记忆") },
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
            // 类别筛选（镜像桌面 categoryFilters：全部/事实/偏好/认知模式；task_status 无入口）
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                MemoryFilterChip("全部", uiState.category == null, roleAccent) { onCategorySelected(null) }
                MemoryFilterChip("事实", uiState.category == MemoryCategory.FACT, roleAccent) {
                    onCategorySelected(MemoryCategory.FACT)
                }
                MemoryFilterChip("偏好", uiState.category == MemoryCategory.PREFERENCE, roleAccent) {
                    onCategorySelected(MemoryCategory.PREFERENCE)
                }
                MemoryFilterChip("认知模式", uiState.category == MemoryCategory.COGNITION_UPDATE, roleAccent) {
                    onCategorySelected(MemoryCategory.COGNITION_UPDATE)
                }
            }

            Spacer(Modifier.height(12.dp))

            if (visible.isEmpty()) {
                // 空态文案镜像桌面（role 视图分支）
                Text(
                    "还没有记忆，多和这个角色聊聊吧",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            visible.forEach { memory ->
                MemoryCard(
                    memory = memory,
                    accent = roleAccent,
                    expanded = uiState.expandedMemoryId == memory.id,
                    confirming = uiState.confirmingMemoryId == memory.id,
                    onToggleSource = { onToggleSource(memory.id) },
                    onOpenForgetConfirm = { onOpenForgetConfirm(memory.id) },
                    onCancelForget = { onCancelForget(memory.id) },
                    onConfirmForget = { onConfirmForget(memory.id) },
                )
                Spacer(Modifier.height(12.dp))
            }
            Spacer(Modifier.height(12.dp))
        }
    }
}

// ── 类别筛选 chip ──────────────────────────────────────────────────────

/** 镜像桌面选中态：accent 15% 底 + accent 字（桌面 color-mix 15%）。 */
@Composable
private fun MemoryFilterChip(label: String, selected: Boolean, accent: Color, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp), // 桌面 rounded-md
        color = if (selected) accent.copy(alpha = 0.15f)
        else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f),
        border = if (selected) null
        else BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.5f)),
    ) {
        Text(
            label,
            style = MaterialTheme.typography.labelMedium,
            color = if (selected) accent else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
        )
    }
}

// ── 记忆卡 ─────────────────────────────────────────────────────────────

/**
 * 记忆卡（镜像桌面 memory card 结构）：
 * 类别头 + 遗忘钮 → 内容 → 遗忘确认面板 → 来源触发行 → 展开来源列表。
 */
@Composable
private fun MemoryCard(
    memory: MemoryItem,
    accent: Color,
    expanded: Boolean,
    confirming: Boolean,
    onToggleSource: () -> Unit,
    onOpenForgetConfirm: () -> Unit,
    onCancelForget: () -> Unit,
    onConfirmForget: () -> Unit,
) {
    val sources = SnapshotStore.memorySources[memory.id].orEmpty()

    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(12.dp), // 桌面 rounded-xl
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp)) {
            // 头行：类别（桌面 🧠 的无 emoji 等效：Brain 线框图标）+ 遗忘钮
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    LucideIcons.Brain,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = accent,
                )
                Spacer(Modifier.size(6.dp))
                Text(
                    memory.category.label,
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.weight(1f))
                ForgetButton(onClick = onOpenForgetConfirm)
            }
            Spacer(Modifier.size(8.dp))
            Text(
                memory.content,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )

            // FR-9 遗忘确认面板（卡内嵌，镜像桌面红色调面板）
            if (confirming) {
                Spacer(Modifier.size(10.dp))
                ForgetConfirmPanel(onConfirm = onConfirmForget, onCancel = onCancelForget)
            }

            Spacer(Modifier.size(10.dp))

            // FR-8 来源触发行
            SourceTriggerRow(memory = memory, expanded = expanded, onClick = onToggleSource)

            // FR-8 展开来源列表
            if (expanded) {
                Spacer(Modifier.size(10.dp))
                SourceMessageList(sources = sources)
            }
        }
    }
}

// ── FR-9 遗忘 ──────────────────────────────────────────────────────────

/** 遗忘钮（fr-groups 指定 Trash2 图标；桌面为红色描边文字钮）。 */
@Composable
private fun ForgetButton(onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(6.dp), // 桌面 rounded-md
        color = MaterialTheme.colorScheme.surface,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.5f)),
    ) {
        Row(
            Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                LucideIcons.Trash2,
                contentDescription = null,
                modifier = Modifier.size(11.dp),
                tint = MaterialTheme.colorScheme.error,
            )
            Spacer(Modifier.size(4.dp))
            Text(
                "遗忘",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

/**
 * 遗忘确认面板（FR-9，文案逐字镜像桌面）。
 * 桌面「确认中双钮禁用」为异步删除守卫；mock 删除即时，该分支不适用（不造假加载态）。
 */
@Composable
private fun ForgetConfirmPanel(onConfirm: () -> Unit, onCancel: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f), // 桌面 red-50/70
        shape = RoundedCornerShape(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(10.dp)) {
            Text(
                "确定要忘记这条吗？忘了就真忘了哦。原始对话还会留在历史里。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(Modifier.size(8.dp))
            Row {
                OutlinedButton(
                    onClick = onConfirm,
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error,
                    ),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.5f)),
                ) { Text("确认遗忘") }
                Spacer(Modifier.size(8.dp))
                OutlinedButton(onClick = onCancel) { Text("再想想") }
            }
        }
    }
}

// ── FR-8 来源溯源 ──────────────────────────────────────────────────────

/** 来源触发行（镜像桌面：Clock + 来源对话 + 查看原文/收起 + 时间小字）。 */
@Composable
private fun SourceTriggerRow(memory: MemoryItem, expanded: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f), // 桌面 slate-50
        shape = RoundedCornerShape(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(10.dp), verticalAlignment = Alignment.Top) {
            Icon(
                LucideIcons.Clock,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(8.dp))
            Column(Modifier.weight(1f)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "来源对话",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(Modifier.weight(1f))
                    Text(
                        if (expanded) "收起" else "查看原文",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary, // 桌面 text-indigo-500
                    )
                }
                Spacer(Modifier.size(2.dp))
                Text(
                    formatMemoryTime(memory.createdAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** 来源消息列表（镜像桌面：左 indigo 竖线 + 消息卡；空来源 → 已不可用）。 */
@Composable
private fun SourceMessageList(sources: List<MemorySourceMessage>) {
    if (sources.isEmpty()) {
        // 桌面三分支归一文案（加载失败/来源为空均显示此句）
        Text(
            "来源对话已不可用",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        return
    }
    Row(Modifier.height(IntrinsicSize.Min)) {
        // 左竖线（桌面 border-l-2 border-indigo-300 pl-4）
        Box(
            Modifier
                .width(2.dp)
                .fillMaxHeight()
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.4f))
        )
        Spacer(Modifier.size(12.dp))
        Column(
            Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            sources.forEach { message ->
                SourceMessageCard(message)
            }
        }
    }
}

/** 来源消息卡：isSource 高亮着色（桌面 indigo-50/70 底 + border-indigo-200）。 */
@Composable
private fun SourceMessageCard(message: MemorySourceMessage) {
    Surface(
        color = if (message.isSource) MaterialTheme.colorScheme.primary.copy(alpha = 0.08f)
        else MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(8.dp),
        border = if (message.isSource) {
            BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.35f))
        } else {
            BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
        },
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(10.dp)) {
            Row(Modifier.fillMaxWidth()) {
                Text(
                    roleLabel(message.role),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.weight(1f))
                Text(
                    formatMemoryTime(message.createdAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.size(4.dp))
            Text(
                message.content,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}

/** 镜像桌面 roleLabel：user→用户、assistant→助手。 */
private fun roleLabel(role: String): String = when (role) {
    "user" -> "用户"
    "assistant" -> "助手"
    else -> role
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun MemoryScreenPreview() {
    EgoSyncTheme {
        MemoryScreen(
            uiState = MemoryUiState.sample(),
            onBack = {},
            onCategorySelected = {},
            onToggleSource = {},
            onOpenForgetConfirm = {},
            onCancelForget = {},
            onConfirmForget = {},
        )
    }
}
