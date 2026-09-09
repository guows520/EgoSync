package com.egosync.companion.ui.onboarding

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
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
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.ChatMessage
import com.egosync.companion.sync.RoleProposalState
import com.egosync.companion.ui.onboarding.onboardingRoleProposal
import com.egosync.companion.ui.components.RoleConfirmDialog
import com.egosync.companion.ui.components.RoleProposalCard
import com.egosync.companion.ui.components.ThinkingDots
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.InfoDensity
import com.egosync.companion.ui.theme.densitySpec

/**
 * FR-21 空状态引导屏（全屏，无底栏；桌面 OnboardingView.tsx 的移动语义适配）：
 * 头部 Home 磁贴 + 「数字分身管家」标题 / 管家气泡访谈流（思考点 + 打字机流式）/
 * 步进 placeholder 输入条 + Play 发送钮 / 「跳过角色引导」链接（第 2 步起）。
 * 角色提议确认弹窗复用 FR-5 共享组件（24 图标 + 8 色板）。
 */
@Composable
fun OnboardingScreen(
    uiState: OnboardingUiState,
    commandReady: Boolean,
    onSendMessage: (String) -> Unit,
    onSkipOnboarding: () -> Unit,
    onRoleProposalConfirm: (name: String, icon: String, color: String, goal: String) -> Unit,
    onRoleProposalSkip: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var input by remember { mutableStateOf("") }
    // FR-5 角色涌现确认弹窗开关（弹窗数据取 uiState.roleProposal）
    var showRoleConfirm by remember { mutableStateOf(false) }
    val listState = rememberLazyListState()
    // 对话流轻量模式（PRD §4.14）：消息流间距与气泡内边距走密度 token
    val d = densitySpec(InfoDensity.CONVERSATIONAL)
    val busy = uiState.thinking || uiState.responding

    // 新消息时跟随滚动到底部
    LaunchedEffect(
        uiState.messages.size,
        uiState.thinking,
        uiState.roleProposal,
        uiState.messages.lastOrNull()?.text,
    ) {
        val totalItems = uiState.messages.size +
            (if (uiState.thinking) 1 else 0) +
            (if (uiState.roleProposal != null) 1 else 0)
        if (totalItems > 0) listState.animateScrollToItem(totalItems - 1)
    }

    // 系统栏 inset 由 MainActivity 根 Scaffold 的 contentPadding 提供（勿重复 systemBarsPadding）
    Surface(modifier = modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(Modifier.fillMaxSize().imePadding()) {
            // 头部（镜像桌面 h-76 header：Home 磁贴 + 标题 + 底分隔线）
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    Modifier
                        .size(40.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(MaterialTheme.colorScheme.primary),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        LucideIcons.Home,
                        contentDescription = "管家",
                        modifier = Modifier.size(20.dp),
                        tint = MaterialTheme.colorScheme.onPrimary,
                    )
                }
                Spacer(Modifier.size(12.dp))
                Text(
                    "数字分身管家",
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onBackground,
                )
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.5f))

            // 气泡访谈流
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp),
                verticalArrangement = Arrangement.spacedBy(d.itemSpacing),
            ) {
                items(uiState.messages, key = { it.id }) { message ->
                    OnboardingBubble(message)
                }
                if (uiState.thinking) {
                    item(key = "thinking") { ThinkingIndicator() }
                }
                // FR-5 角色涌现提案卡（确认交互在弹窗内完成）
                uiState.roleProposal?.let { proposal ->
                    item(key = "role-proposal") {
                        RoleProposalCard(
                            proposal = proposal,
                            enabled = commandReady,
                            onOpenConfirm = { showRoleConfirm = true },
                            onSkip = onRoleProposalSkip,
                        )
                    }
                }
                item { Spacer(Modifier.size(6.dp)) }
            }

            // 输入条（镜像桌面：流式中禁用，Play 发送钮，右下跳过链接）
            Surface(color = MaterialTheme.colorScheme.surface) {
                Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 10.dp)) {
                    if (!commandReady) {
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
                            placeholder = { Text(OnboardingUiState.placeholderFor(uiState.step)) },
                            modifier = Modifier.weight(1f),
                            enabled = commandReady && !busy,
                            maxLines = 3,
                            shape = RoundedCornerShape(24.dp),
                        )
                        Spacer(Modifier.size(10.dp))
                        Button(
                            onClick = {
                                onSendMessage(input)
                                input = ""
                            },
                            enabled = commandReady && input.isNotBlank() && !busy,
                            modifier = Modifier.size(40.dp),
                            contentPadding = PaddingValues(0.dp),
                            shape = RoundedCornerShape(10.dp), // 桌面发送钮 rounded-lg
                        ) {
                            Icon(
                                LucideIcons.Play,
                                contentDescription = "发送",
                                modifier = Modifier.size(16.dp),
                            )
                        }
                    }
                    if (OnboardingUiState.skipVisible(uiState.step)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.End,
                        ) {
                            TextButton(onClick = onSkipOnboarding, enabled = !busy) {
                                Text(
                                    "跳过角色引导",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    // FR-5 角色涌现确认弹窗（仅待处理态可开）
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

// ── 气泡 ───────────────────────────────────────────────────────────────

/** 访谈气泡：管家（带头像+描边）左对齐 / 用户（主色）右对齐；流式尾部带光标。 */
@Composable
private fun OnboardingBubble(message: ChatMessage) {
    val d = densitySpec(InfoDensity.CONVERSATIONAL)
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (message.fromButler) Arrangement.Start else Arrangement.End,
    ) {
        if (message.fromButler) {
            Box(
                Modifier
                    .size(30.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primary),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    LucideIcons.Home,
                    contentDescription = "管家",
                    modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onPrimary,
                )
            }
            Spacer(Modifier.size(8.dp))
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
            Text(
                text = if (message.streaming) "${message.text}▍" else message.text,
                style = MaterialTheme.typography.bodyMedium,
                color = if (message.fromButler) MaterialTheme.colorScheme.onSurface
                else MaterialTheme.colorScheme.onPrimary,
                modifier = Modifier
                    .widthIn(max = 290.dp)
                    .padding(horizontal = d.unitPaddingX, vertical = d.unitPaddingY),
            )
        }
    }
}

/** 思考中指示：管家头像 + 三点弹跳（共享 ThinkingDots）。 */
@Composable
private fun ThinkingIndicator() {
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
            ThinkingDots(Modifier.padding(horizontal = 16.dp, vertical = 14.dp))
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun OnboardingScreenPreview() {
    EgoSyncTheme {
        OnboardingScreen(
            uiState = OnboardingUiState(
                messages = listOf(
                    ChatMessage("ob-1", true, onboardingGreeting),
                    ChatMessage("ob-2", false, "我是 boss，一个产品经理"),
                ),
                step = 2,
            ),
            commandReady = true,
            onSendMessage = {},
            onSkipOnboarding = {},
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun OnboardingScreenProposalPreview() {
    EgoSyncTheme {
        OnboardingScreen(
            uiState = OnboardingUiState(
                messages = listOf(
                    ChatMessage("ob-1", true, onboardingGreeting),
                ),
                step = 3,
                roleProposal = onboardingRoleProposal,
            ),
            commandReady = true,
            onSendMessage = {},
            onSkipOnboarding = {},
            onRoleProposalConfirm = { _, _, _, _ -> },
            onRoleProposalSkip = {},
        )
    }
}
