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
import androidx.compose.foundation.clickable
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
    /** T-S2：写操作（选择性遗忘）判据——离线时确认钮禁用，不发不可达请求。 */
    commandReady: Boolean = true,
    onBack: () -> Unit,
    onCategorySelected: (MemoryCategory?) -> Unit,
    onToggleSource: (memoryId: String) -> Unit,
    onOpenForgetConfirm: (memoryId: String) -> Unit,
    onCancelForget: (memoryId: String) -> Unit,
    onConfirmForget: (memoryId: String) -> Unit,
    onRetry: () -> Unit = {},
    onRetrySources: (memoryId: String) -> Unit = {},
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

            // 13.3 指令通道：列表现查现显（memory.list）——加载/错误/空三态
            // 复用原 AC5 占位槽（不再宣称「通道尚未接通」）
            when {
                uiState.loading -> Text(
                    "正在加载记忆…",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                uiState.error != null -> Column {
                    Text(
                        "记忆加载失败：${uiState.error}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.error,
                    )
                    Spacer(Modifier.height(8.dp))
                    OutlinedButton(onClick = onRetry) { Text("重试") }
                }
                visible.isEmpty() -> Text(
                    // AC5 契约空态：记忆内容永不进入快照（防快照膨胀），此处即真实空
                    "这个角色还没有记忆",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            visible.forEach { memory ->
                MemoryCard(
                    memory = memory,
                    accent = roleAccent,
                    commandReady = commandReady,
                    expanded = uiState.expandedMemoryId == memory.id,
                    confirming = uiState.confirmingMemoryId == memory.id,
                    sources = uiState.sourcesByMemoryId[memory.id],
                    sourcesLoading = memory.id in uiState.loadingSourceIds,
                    onToggleSource = { onToggleSource(memory.id) },
                    onOpenForgetConfirm = { onOpenForgetConfirm(memory.id) },
                    onCancelForget = { onCancelForget(memory.id) },
                    onConfirmForget = { onConfirmForget(memory.id) },
                    onRetrySources = { onRetrySources(memory.id) },
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
 * 13.3 偏差：来源列表经 `sources` 参数注入（story 原文「组件零改动复用」不准确——
 * 13.2 并未预留来源槽位，此处补齐并记录为已批准偏差）。
 */
@Composable
private fun MemoryCard(
    memory: MemoryItem,
    accent: Color,
    /** T-S2：写操作（选择性遗忘）判据——透传至确认面板。 */
    commandReady: Boolean,
    expanded: Boolean,
    confirming: Boolean,
    sources: List<MemorySourceMessage>?,
    sourcesLoading: Boolean,
    onToggleSource: () -> Unit,
    onOpenForgetConfirm: () -> Unit,
    onCancelForget: () -> Unit,
    onConfirmForget: () -> Unit,
    onRetrySources: () -> Unit = {},
) {
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
                ForgetConfirmPanel(commandReady = commandReady, onConfirm = onConfirmForget, onCancel = onCancelForget)
            }

            Spacer(Modifier.size(10.dp))

            // FR-8 来源触发行
            SourceTriggerRow(memory = memory, expanded = expanded, onClick = onToggleSource)

            // FR-8 展开来源列表：memory.sources 现查（懒加载，展开时触发）
            if (expanded) {
                Spacer(Modifier.size(10.dp))
                when {
                    sourcesLoading -> Text(
                        "正在加载来源对话…",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    sources == null -> Row(verticalAlignment = Alignment.CenterVertically) {
                        // 失败即显式重试入口（评审 C18）：文案称「请重试」却无可点元素
                        // 是断裂交互；失败不缓存，重查即重发
                        Text(
                            "来源对话加载失败，请重试",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                        Spacer(Modifier.size(8.dp))
                        Text(
                            "重试",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.clickable { onRetrySources() },
                        )
                    }
                    sources.isEmpty() -> Text(
                        "暂无来源对话",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    else -> sources.forEach { message ->
                        SourceMessageRow(message = message, accent = accent)
                        Spacer(Modifier.size(6.dp))
                    }
                }
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
 * T-S2：确认钮按 commandReady 禁用（离线不可写，与 Tasks/通知响应同判据）。
 */
@Composable
private fun ForgetConfirmPanel(commandReady: Boolean, onConfirm: () -> Unit, onCancel: () -> Unit) {
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
                    enabled = commandReady,
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

/** 来源消息行（镜像桌面来源列表条目：发言方 + 内容 + 时间；来源条目 accent 左缘标记）。 */
@Composable
private fun SourceMessageRow(message: MemorySourceMessage, accent: Color) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
        shape = RoundedCornerShape(6.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(
                    color = if (message.isSource) accent.copy(alpha = 0.15f)
                    else MaterialTheme.colorScheme.surfaceVariant,
                    shape = RoundedCornerShape(4.dp),
                ) {
                    Text(
                        if (message.role == "assistant") "角色" else "我",
                        style = MaterialTheme.typography.labelSmall,
                        color = if (message.isSource) accent
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                    )
                }
                Spacer(Modifier.size(6.dp))
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
            onRetry = {},
            onRetrySources = {},
        )
    }
}
