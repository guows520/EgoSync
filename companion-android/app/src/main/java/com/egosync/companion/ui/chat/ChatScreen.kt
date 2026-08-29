package com.egosync.companion.ui.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.ActionCardSuggestion
import com.egosync.companion.sync.ActionCardState
import com.egosync.companion.sync.ChatConversation
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.DecompositionState
import com.egosync.companion.sync.ExecutionTraceBlock
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.RoleProposalState
import com.egosync.companion.ui.previewRoles
import com.egosync.companion.sync.TaskDecompositionProposal
import com.egosync.companion.sync.ToolStatus
import com.egosync.companion.ui.components.RoleConfirmDialog
import com.egosync.companion.ui.components.RoleProposalCard
import com.egosync.companion.ui.components.ThinkingDots
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.icons.RoleIcons
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.InfoDensity
import com.egosync.companion.ui.theme.accent
import com.egosync.companion.ui.theme.densitySpec
import java.util.Calendar
import java.util.Locale

/**
 * ① 管家对话 Tab：消息流（用户/管家气泡、思考中态、流式打字机）、输入框、
 * 待确认建议 ActionCard（确认/拒绝按钮态）。
 * 组 1 chat：顶部角色切换器（FR-20）、两段委派气泡（FR-1）、拆分提案卡（FR-2）、
 * 执行溯源折叠区（FR-29）、低置信标注（FR-30）、工具执行状态行+停止（FR-33）。
 * 组 3：角色涌现提案卡（FR-5，对话流触发）+ 确认弹窗（24 图标网格 + 8 色板）。
 */
@Composable
fun ChatScreen(
    uiState: ChatUiState,
    engineAvailable: Boolean,
    dataCutoffLabel: String? = null,
    onSendMessage: (String) -> Unit,
    onActionCardRespond: (cardId: String, confirmed: Boolean) -> Unit,
    onRoleSelected: (roleId: String?) -> Unit,
    onStopStreaming: () -> Unit,
    onDecompositionRespond: (proposalId: String, accepted: Boolean) -> Unit,
    onRoleProposalConfirm: (name: String, icon: String, color: String, goal: String) -> Unit,
    onRoleProposalSkip: () -> Unit,
    onNewConversation: () -> Unit,
    onSelectConversation: (conversationId: String) -> Unit,
    onDeleteConversation: (conversationId: String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var input by remember { mutableStateOf("") }
    // FR-5 角色涌现确认弹窗开关（弹窗数据取 uiState.roleProposal）
    var showRoleConfirm by remember { mutableStateOf(false) }
    val listState = rememberLazyListState()
    // 对话流轻量模式（PRD §4.14）：消息流间距与气泡内边距走密度 token
    val d = densitySpec(InfoDensity.CONVERSATIONAL)
    val activeRole = uiState.activeRoleId?.let { id -> uiState.roles.find { it.id == id } }
    val roleById = remember(uiState.roles) { uiState.roles.associateBy { it.id } }

    // 新消息时跟随滚动到底部
    LaunchedEffect(
        uiState.messages.size,
        uiState.actionCards.size,
        uiState.decomposition,
        uiState.roleProposal,
        uiState.thinking,
        uiState.streamingToolTitle,
        uiState.messages.lastOrNull()?.text,
    ) {
        val totalItems = uiState.messages.size +
            (if (uiState.thinking) 1 else 0) +
            (if (uiState.streamingToolTitle != null) 1 else 0) +
            uiState.actionCards.size +
            (if (uiState.decomposition != null) 1 else 0) +
            (if (uiState.roleProposal != null) 1 else 0)
        if (totalItems > 0) listState.animateScrollToItem(totalItems - 1)
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .imePadding(),
    ) {
        // 会话头：移植桌面 ChatHeader（新对话 + 历史对话下拉）
        ChatHeaderRow(
            conversations = uiState.conversations,
            currentConversationId = uiState.currentConversationId,
            onNewConversation = onNewConversation,
            onSelectConversation = onSelectConversation,
            onDeleteConversation = onDeleteConversation,
        )

        // T7：conversations 域被 10MB 截断时明示数据截止（不以缺失冒充完整，AC4）
        if (dataCutoffLabel != null) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    LucideIcons.AlertTriangle,
                    contentDescription = null,
                    modifier = Modifier.size(11.dp),
                    tint = MaterialTheme.colorScheme.error,
                )
                Spacer(Modifier.size(4.dp))
                Text(
                    "会话数据截至 $dataCutoffLabel（更早消息未随快照下发）",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }

        // FR-20 角色切换器：桌面 64px 侧栏的移动语义适配（顶部水平滚动）
        RoleSwitcherRow(
            activeRoleId = uiState.activeRoleId,
            roles = uiState.roles,
            onSelect = onRoleSelected,
        )

        LazyColumn(
            state = listState,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(d.itemSpacing),
        ) {
            items(uiState.messages, key = { it.id }) { message ->
                // FR-29 执行溯源：桌面渲染在气泡上方，默认折叠
                val trace = uiState.traceByMessageId[message.id]
                Column {
                    if (trace != null) {
                        ExecutionTrace(trace)
                        Spacer(Modifier.size(4.dp))
                    }
                    MessageBubble(message, roleById[message.senderRoleId])
                }
            }
            if (uiState.thinking) {
                item(key = "thinking") { ThinkingBubble() }
            }
            // FR-33 工具执行可视：工具名 + 运行中指示
            uiState.streamingToolTitle?.let { stage ->
                item(key = "tool-stage") { ToolStatusRow(stage) }
            }
            items(uiState.actionCards, key = { it.id }) { card ->
                ActionCard(
                    card = card,
                    enabled = engineAvailable,
                    onRespond = { confirmed -> onActionCardRespond(card.id, confirmed) },
                )
            }
            // FR-2 任务拆分提案卡（对话流内嵌，镜像桌面 TaskDecompositionCard）
            uiState.decomposition?.let { proposal ->
                item(key = proposal.id) {
                    TaskDecompositionCard(
                        proposal = proposal,
                        enabled = engineAvailable,
                        onRespond = { accepted ->
                            onDecompositionRespond(proposal.id, accepted)
                        },
                    )
                }
            }
            // FR-5 角色涌现提案卡（对话流触发器；确认交互在弹窗内完成）
            uiState.roleProposal?.let { proposal ->
                item(key = "role-proposal") {
                    RoleProposalCard(
                        proposal = proposal,
                        enabled = engineAvailable,
                        onOpenConfirm = { showRoleConfirm = true },
                        onSkip = onRoleProposalSkip,
                    )
                }
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
                val busy = uiState.thinking || uiState.responding
                Row(verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = { input = it },
                        placeholder = {
                            Text(if (activeRole != null) "跟 ${activeRole.name} 说点什么…" else "对管家说点什么…")
                        },
                        modifier = Modifier.weight(1f),
                        enabled = engineAvailable && !busy,
                        maxLines = 3,
                        shape = RoundedCornerShape(24.dp),
                    )
                    Spacer(Modifier.size(10.dp))
                    if (busy) {
                        // FR-33 停止（镜像桌面 ChatInput Square 停止钮）
                        IconButton(onClick = onStopStreaming) {
                            Icon(
                                LucideIcons.Square,
                                contentDescription = "停止",
                                modifier = Modifier.size(16.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    } else {
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

    // FR-5 角色涌现确认弹窗（仅待处理态可开；X/「不需要」/点外部 → 关闭并跳过，镜像桌面 onCancel 汇一）
    val pendingProposal = uiState.roleProposal
    if (showRoleConfirm && pendingProposal?.state == RoleProposalState.PENDING) {
        RoleConfirmDialog(
            proposal = pendingProposal,
            onConfirm = { name, icon, color, goal ->
                showRoleConfirm = false
                onRoleProposalConfirm(name, icon, color, goal)
            },
            onDismiss = {
                showRoleConfirm = false
                onRoleProposalSkip()
            },
        )
    }
}

// ── 会话头：新对话 + 历史对话下拉（移植桌面 ChatHeader）────────────────

/** 相对时间显示，镜像桌面 ConversationList.tsx formatRelativeTime。 */
private fun formatRelativeTime(nowMs: Long, updatedAt: Long): String {
    val diff = nowMs - updatedAt
    val minutes = diff / 60_000L
    if (minutes < 1) return "刚刚"
    if (minutes < 60) return "${minutes}分钟前"
    val hours = minutes / 60
    if (hours < 24) return "${hours}小时前"
    val cal = Calendar.getInstance().apply { timeInMillis = updatedAt }
    val month = cal.get(Calendar.MONTH) + 1
    val day = cal.get(Calendar.DAY_OF_MONTH)
    val hh = "%02d".format(Locale.ROOT, cal.get(Calendar.HOUR_OF_DAY))
    val mm = "%02d".format(Locale.ROOT, cal.get(Calendar.MINUTE))
    return "${month}月${day}日 $hh:$mm"
}

/**
 * 会话头（镜像桌面 ChatHeader.tsx）：M3 小号文本按钮「新对话」(Plus 14dp) +
 * 「历史对话」(History 14dp) → DropdownMenu 列出当前角色会话（标题+相对时间+删除）。
 */
@Composable
private fun ChatHeaderRow(
    conversations: List<ChatConversation>,
    currentConversationId: String,
    onNewConversation: () -> Unit,
    onSelectConversation: (conversationId: String) -> Unit,
    onDeleteConversation: (conversationId: String) -> Unit,
) {
    var showHistory by remember { mutableStateOf(false) }
    val nowMs = System.currentTimeMillis()

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = onNewConversation) {
            Icon(
                LucideIcons.Plus,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(6.dp))
            Text("新对话", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }

        Box {
            TextButton(onClick = { showHistory = !showHistory }) {
                Icon(
                    LucideIcons.History,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.size(6.dp))
                Text("历史对话", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            DropdownMenu(
                expanded = showHistory,
                onDismissRequest = { showHistory = false },
            ) {
                if (conversations.isEmpty()) {
                    // 空态：一条禁用项（镜像桌面「暂无历史对话」）
                    DropdownMenuItem(
                        text = { Text("暂无历史对话") },
                        onClick = {},
                        enabled = false,
                    )
                } else {
                    conversations.forEach { conv ->
                        DropdownMenuItem(
                            text = {
                                Column {
                                    Text(
                                        conv.title.ifEmpty { "新对话" },
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = if (conv.id == currentConversationId)
                                            MaterialTheme.colorScheme.primary
                                        else MaterialTheme.colorScheme.onSurface,
                                    )
                                    Text(
                                        formatRelativeTime(nowMs, conv.updatedAt),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            },
                            onClick = {
                                onSelectConversation(conv.id)
                                showHistory = false
                            },
                            trailingIcon = {
                                IconButton(onClick = { onDeleteConversation(conv.id) }) {
                                    Icon(
                                        LucideIcons.Trash2,
                                        contentDescription = "删除对话",
                                        modifier = Modifier.size(14.dp),
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}

// ── FR-20 顶部角色切换器 ───────────────────────────────────────────────

/** 首位=管家（Home 图标，镜像桌面侧栏首位）；其后为角色 chip，图标走 RoleIcons.getRoleIcon。 */
@Composable
private fun RoleSwitcherRow(
    activeRoleId: String?,
    roles: List<RoleCard>,
    onSelect: (String?) -> Unit,
) {
    LazyRow(
        modifier = Modifier.fillMaxWidth(),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item(key = "butler") {
            RoleChip(
                label = "管家",
                icon = LucideIcons.Home,
                accentColor = null,
                selected = activeRoleId == null,
                onClick = { onSelect(null) },
            )
        }
        items(roles, key = { it.id }) { role ->
            RoleChip(
                label = role.name,
                icon = RoleIcons.getRoleIcon(role.icon),
                accentColor = role.domain.accent().accent,
                selected = activeRoleId == role.id,
                onClick = { onSelect(role.id) },
            )
        }
    }
}

@Composable
private fun RoleChip(
    label: String,
    icon: ImageVector,
    accentColor: Color?,
    selected: Boolean,
    onClick: () -> Unit,
) {
    // 镜像桌面 Sidebar 选中态：indigo 高亮圆角；未选中时角色图标带角色色温
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp),
        color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface,
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                icon,
                contentDescription = label,
                modifier = Modifier.size(14.dp),
                tint = when {
                    selected -> MaterialTheme.colorScheme.onPrimary
                    accentColor != null -> accentColor
                    else -> MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
            Spacer(Modifier.size(6.dp))
            Text(
                label,
                style = MaterialTheme.typography.labelMedium,
                color = if (selected) MaterialTheme.colorScheme.onPrimary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ── FR-29 执行溯源（可折叠，ChevronRight）──────────────────────────────

/** 镜像桌面 ChatBubble.ExecutionTrace：折叠头（ChevronRight）+ Think/Narration/Action 三类块。 */
@Composable
private fun ExecutionTrace(blocks: List<ExecutionTraceBlock>) {
    var expanded by remember(blocks) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .clip(RoundedCornerShape(6.dp))
                .clickable { expanded = !expanded }
                .padding(horizontal = 4.dp, vertical = 2.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                LucideIcons.ChevronRight,
                contentDescription = null,
                modifier = Modifier
                    .size(12.dp)
                    .rotate(if (expanded) 90f else 0f),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(4.dp))
            Text(
                "执行过程",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (expanded) {
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.5f)),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(10.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    blocks.forEach { block -> TraceBlockView(block) }
                }
            }
        }
    }
}

@Composable
private fun TraceBlockView(block: ExecutionTraceBlock) {
    when (block) {
        is ExecutionTraceBlock.Thinking -> {
            Text(
                if (block.elapsedSeconds == null) "Think 思考时长未知"
                else "Think 思考了${block.elapsedSeconds}秒",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                block.content,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        is ExecutionTraceBlock.Narration -> Text(
            block.content,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        is ExecutionTraceBlock.Action -> Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                LucideIcons.ChevronRight,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            actionTypeLabel(block.actionType)?.let { label ->
                Text(
                    label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.size(6.dp))
            }
            Text(
                block.title,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f, fill = false),
            )
            Spacer(Modifier.size(8.dp))
            Text(
                when (block.status) {
                    ToolStatus.RUNNING -> "运行中"
                    ToolStatus.COMPLETED -> "完成"
                    ToolStatus.FAILED -> "失败"
                },
                style = MaterialTheme.typography.labelSmall,
                color = when (block.status) {
                    ToolStatus.RUNNING -> MaterialTheme.colorScheme.primary
                    ToolStatus.COMPLETED -> MaterialTheme.colorScheme.onSurfaceVariant
                    ToolStatus.FAILED -> MaterialTheme.colorScheme.error
                },
            )
        }
    }
}

/** 镜像桌面 actionTypeLabel：read 不显示标签 */
private fun actionTypeLabel(type: String): String? = when (type) {
    "shell" -> "Shell"
    "read" -> null
    "edit" -> "Edit"
    "write" -> "Write"
    "skill" -> "Skill"
    "explore" -> "Explore"
    else -> "Tool"
}

// ── 消息气泡 ───────────────────────────────────────────────────────────

/** 发言人感知：senderRole 非空时用角色头像（getRoleIcon+色温）并在气泡上方显示角色名。 */
@Composable
private fun MessageBubble(message: ChatMessage, senderRole: RoleCard?) {
    val d = densitySpec(InfoDensity.CONVERSATIONAL)
    val isUser = !message.fromButler
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (message.fromButler) Arrangement.Start else Arrangement.End,
    ) {
        if (message.fromButler) {
            Box(
                Modifier
                    .size(30.dp)
                    .clip(CircleShape)
                    .background(senderRole?.domain?.accent()?.accent ?: MaterialTheme.colorScheme.primaryContainer),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    senderRole?.icon?.let { RoleIcons.getRoleIcon(it) } ?: LucideIcons.Home,
                    contentDescription = senderRole?.name ?: "管家",
                    modifier = Modifier.size(16.dp),
                    tint = if (senderRole != null) MaterialTheme.colorScheme.onPrimary
                    else MaterialTheme.colorScheme.onPrimaryContainer,
                )
            }
            Spacer(Modifier.size(8.dp))
        }
        Column(modifier = Modifier.widthIn(max = 290.dp)) {
            if (senderRole != null) {
                // FR-1 第二段/FR-20 角色视图：气泡上方角色名（镜像桌面 ChatBubble 助手名标）
                Text(
                    senderRole.name,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(bottom = 2.dp),
                )
            }
            Surface(
                color = if (message.fromButler) MaterialTheme.colorScheme.surface
                else MaterialTheme.colorScheme.primary,
                border = if (message.fromButler)
                    BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
                else null,
                shape = RoundedCornerShape(
                    topStart = 16.dp,
                    topEnd = 16.dp,
                    bottomStart = if (message.fromButler) 4.dp else 16.dp,
                    bottomEnd = if (message.fromButler) 16.dp else 4.dp,
                ),
            ) {
                Column(Modifier.padding(horizontal = d.unitPaddingX, vertical = d.unitPaddingY)) {
                    Text(
                        text = if (message.streaming) "${message.text}▍" else message.text,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (message.fromButler) MaterialTheme.colorScheme.onSurface
                        else MaterialTheme.colorScheme.onPrimary,
                    )
                    // FR-30 不确定性表达：confidence<0.7 → 内联标注（镜像桌面文案）
                    if (message.lowConfidence) {
                        Spacer(Modifier.size(4.dp))
                        Text(
                            "（置信度较低，仅供参考）",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

// ── FR-33 工具执行状态行 ───────────────────────────────────────────────

@Composable
private fun ToolStatusRow(stageTitle: String) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier.padding(top = 2.dp),
    ) {
        Icon(
            LucideIcons.Play,
            contentDescription = null,
            modifier = Modifier.size(12.dp),
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(Modifier.size(6.dp))
        Text(
            "正在执行：$stageTitle",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.size(8.dp))
        CircularProgressIndicator(
            modifier = Modifier.size(10.dp),
            strokeWidth = 1.5.dp,
        )
    }
}

@Composable
private fun ThinkingBubble() {
    Row {
        Box(
            Modifier
                .size(30.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primary),
            contentAlignment = Alignment.Center,
        ) {
            Icon(LucideIcons.Home, contentDescription = "管家", modifier = Modifier.size(16.dp))
        }
        Spacer(Modifier.size(8.dp))
        Surface(
            color = MaterialTheme.colorScheme.surface,
            shape = RoundedCornerShape(16.dp, 16.dp, 4.dp, 16.dp),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        ) {
            // 母本 ChatBubble.BounceDots：三圆点 160ms 交错弹跳（1.4s 周期）
            ThinkingDots(Modifier.padding(horizontal = 16.dp, vertical = 14.dp))
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
    val stateBorder = when (card.state) {
        // 母本 ActionCard.tsx：confirmed → border-indigo-300；rejected → border-slate-300
        ActionCardState.CONFIRMED -> BorderStroke(
            1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.6f),
        )
        ActionCardState.REJECTED -> BorderStroke(
            1.dp, MaterialTheme.colorScheme.outline,
        )
        ActionCardState.PENDING -> BorderStroke(
            1.dp, MaterialTheme.colorScheme.outline,
        )
    }
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp), // 母本 rounded-[10px]
        modifier = modifier.fillMaxWidth(),
        border = stateBorder,
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
                ActionCardState.CONFIRMED -> Row(verticalAlignment = Alignment.CenterVertically) {
                    // 母本：w-7 圆底 + Check（indigo）
                    Box(
                        Modifier
                            .size(28.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)),
                        contentAlignment = Alignment.Center,
                    ) { Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary) }
                    Spacer(Modifier.size(8.dp))
                    Text(
                        "已确认 · 已转交管家执行",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
                ActionCardState.REJECTED -> Row(verticalAlignment = Alignment.CenterVertically) {
                    // 母本：w-7 灰圆底 + X
                    Box(
                        Modifier
                            .size(28.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.surfaceVariant),
                        contentAlignment = Alignment.Center,
                    ) { Text("×", color = MaterialTheme.colorScheme.onSurfaceVariant) }
                    Spacer(Modifier.size(8.dp))
                    Text(
                        "已拒绝 · 建议已放下",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

// ── FR-2 任务拆分提案卡 ───────────────────────────────────────────────

/** 镜像桌面 TaskDecompositionCard.tsx：ListTodo 图标 + 条目列表 + 接受/不要拆分。 */
@Composable
private fun TaskDecompositionCard(
    proposal: TaskDecompositionProposal,
    enabled: Boolean,
    onRespond: (accepted: Boolean) -> Unit,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp), // 母本 rounded-[10px]
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.35f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(14.dp)) {
            Box(
                Modifier
                    .size(32.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.08f)),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    LucideIcons.ListTodo, // 母本 ListTree 的移动等效（fr-groups 裁决）
                    contentDescription = null,
                    modifier = Modifier.size(17.dp),
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
            Spacer(Modifier.size(12.dp))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "建议拆分为 ${proposal.items.size} 个任务",
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(Modifier.size(8.dp))
                    Icon(
                        RoleIcons.getRoleIcon(proposal.roleIcon),
                        contentDescription = proposal.roleName,
                        modifier = Modifier.size(12.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.size(4.dp))
                    Text(
                        proposal.roleName,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(Modifier.size(4.dp))
                Text(
                    "原事项：${proposal.taskSummary}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.size(10.dp))
                proposal.items.forEachIndexed { index, item ->
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(Modifier.padding(horizontal = 10.dp, vertical = 8.dp)) {
                            Text(
                                "${index + 1}. ${item.title}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            item.deadline?.let { deadline ->
                                Spacer(Modifier.size(8.dp))
                                Text(
                                    "截止 $deadline",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                    Spacer(Modifier.size(6.dp))
                }
                when (proposal.state) {
                    DecompositionState.PENDING -> Row(Modifier.align(Alignment.End)) {
                        OutlinedButton(onClick = { onRespond(false) }, enabled = enabled) {
                            Text("不要拆分")
                        }
                        Spacer(Modifier.size(8.dp))
                        Button(onClick = { onRespond(true) }, enabled = enabled) {
                            Text("接受拆分")
                        }
                    }
                    DecompositionState.ACCEPTED -> DecompositionResultRow(
                        text = "已拆分 · ${proposal.items.size} 个任务已创建",
                        confirmed = true,
                    )
                    DecompositionState.KEPT_SINGLE -> DecompositionResultRow(
                        text = "保持单任务 · 未拆分",
                        confirmed = false,
                    )
                }
            }
        }
    }
}

@Composable
private fun DecompositionResultRow(text: String, confirmed: Boolean) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(
            Modifier
                .size(28.dp)
                .clip(CircleShape)
                .background(
                    if (confirmed) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)
                    else MaterialTheme.colorScheme.surfaceVariant
                ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                if (confirmed) LucideIcons.Check else LucideIcons.ListTodo,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = if (confirmed) MaterialTheme.colorScheme.primary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(Modifier.size(8.dp))
        Text(
            text,
            style = MaterialTheme.typography.labelMedium,
            color = if (confirmed) MaterialTheme.colorScheme.primary
            else MaterialTheme.colorScheme.onSurfaceVariant,
        )
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
            onRoleSelected = {},
            onStopStreaming = {},
            onDecompositionRespond = { _, _ -> },
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
            onNewConversation = {},
            onSelectConversation = {},
            onDeleteConversation = {},
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
                    previewInitialChat.first(),
                    ChatMessage("u1", false, "今天下午都有什么安排？"),
                    ChatMessage("b2", true, "下午 2 点是产品评审，材料已备好。四点", streaming = true),
                ),
                thinking = false,
                actionCards = emptyList(),
                responding = true,
                streamingToolTitle = "整理竞品对比要点",
            ),
            engineAvailable = true,
            onSendMessage = {},
            onActionCardRespond = { _, _ -> },
            onRoleSelected = {},
            onStopStreaming = {},
            onDecompositionRespond = { _, _ -> },
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
            onNewConversation = {},
            onSelectConversation = {},
            onDeleteConversation = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun ChatScreenRoleViewPreview() {
    EgoSyncTheme {
        val seed = previewRoleChatSeeds.getValue("role-pm")
        ChatScreen(
            uiState = ChatUiState(
                messages = seed,
                activeRoleId = "role-pm",
                roles = previewRoles,
                traceByMessageId = mapOf(seed.first().id to executionTrace),
            ),
            engineAvailable = true,
            onSendMessage = {},
            onActionCardRespond = { _, _ -> },
            onRoleSelected = {},
            onStopStreaming = {},
            onDecompositionRespond = { _, _ -> },
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
            onNewConversation = {},
            onSelectConversation = {},
            onDeleteConversation = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun ChatScreenRoleProposalPreview() {
    EgoSyncTheme {
        ChatScreen(
            uiState = ChatUiState(
                messages = previewInitialChat,
                roleProposal = roleProposal,
            ),
            engineAvailable = true,
            onSendMessage = {},
            onActionCardRespond = { _, _ -> },
            onRoleSelected = {},
            onStopStreaming = {},
            onDecompositionRespond = { _, _ -> },
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
            onNewConversation = {},
            onSelectConversation = {},
            onDeleteConversation = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun RoleConfirmDialogPreview() {
    EgoSyncTheme {
        RoleConfirmDialog(
            proposal = roleProposal,
            onConfirm = { _, _, _, _ -> },
            onDismiss = {},
        )
    }
}
