package com.egosync.companion.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.RoleProposal
import com.egosync.companion.sync.RoleProposalState
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.icons.RoleIcons

/**
 * FR-5 角色涌现提案卡（对话流触发器；确认交互在 [RoleConfirmDialog] 内）。
 * 自 ChatScreen 平移共享（组 6 onboarding 复用），逻辑零变更。
 */
@Composable
fun RoleProposalCard(
    proposal: RoleProposal,
    enabled: Boolean,
    onOpenConfirm: () -> Unit,
    onSkip: () -> Unit,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp), // 母本 rounded-[10px]
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.35f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp)) {
            Text(
                "角色涌现 · 管家建议",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.size(6.dp))
            Text(
                "需要为您创建这个角色吗？", // 桌面弹窗标题（卡片承载同一问句）
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(Modifier.size(8.dp))
            // 提议预览：图标（白名单回退）+ 名称/目标
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    Modifier
                        .size(36.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(parseHexColor(RoleIcons.normalizeColorHex(proposal.color))),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        RoleIcons.getRoleIcon(proposal.icon),
                        contentDescription = proposal.name,
                        modifier = Modifier.size(18.dp),
                        tint = Color.White,
                    )
                }
                Spacer(Modifier.size(10.dp))
                Column {
                    Text(
                        proposal.name,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    proposal.goal?.takeIf { it.isNotBlank() }?.let { goal ->
                        Text(
                            goal,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
            Spacer(Modifier.size(10.dp))
            when (proposal.state) {
                RoleProposalState.PENDING -> Row(Modifier.align(Alignment.End)) {
                    OutlinedButton(onClick = onSkip, enabled = enabled) {
                        Text("不需要")
                    }
                    Spacer(Modifier.size(8.dp))
                    Button(onClick = onOpenConfirm, enabled = enabled) {
                        Text("创建角色")
                    }
                }
                RoleProposalState.CREATED -> ProposalResultRow(confirmed = true, text = "已创建 · ${proposal.name}")
                RoleProposalState.SKIPPED -> ProposalResultRow(confirmed = false, text = "已跳过 · 未创建角色")
            }
        }
    }
}

@Composable
private fun ProposalResultRow(confirmed: Boolean, text: String) {
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
                if (confirmed) LucideIcons.Check else LucideIcons.X,
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
