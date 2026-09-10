package com.egosync.companion.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.egosync.companion.sync.RoleProposal
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.icons.RoleIcons

/**
 * FR-5 角色涌现确认弹窗（镜像桌面 RoleConfirmModal）：
 * 标题+X / 预览块 / 名称（≤20 字）/ 图标 24 网格（8 列）/ 品牌色 8 色板 / 目标（≤80 字）/ 不需要-创建。
 * 图标/颜色经 normalize 白名单回退；创建禁用守卫 = 名称非空（桌面 busy 分支不适用：mock 创建即时）。
 * 自 ChatScreen 平移共享，逻辑零变更。
 */
@Composable
fun RoleConfirmDialog(
    proposal: RoleProposal,
    onConfirm: (name: String, icon: String, color: String, goal: String) -> Unit,
    onDismiss: () -> Unit,
) {
    // 每次打开用提议数据重置表单（镜像桌面 useEffect on (open, proposal)）
    var name by remember { mutableStateOf(proposal.name.trim()) }
    var iconId by remember { mutableStateOf(RoleIcons.normalizeIconId(proposal.icon)) }
    var colorHex by remember { mutableStateOf(RoleIcons.normalizeColorHex(proposal.color)) }
    var goal by remember { mutableStateOf(proposal.goal?.trim() ?: "") }

    val trimmedName = name.trim()
    val canConfirm = trimmedName.isNotEmpty()

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(16.dp), // 桌面 rounded-2xl
            color = MaterialTheme.colorScheme.surface,
        ) {
            Column(
                Modifier
                    .padding(20.dp)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                // 头行：标题 + X 关闭（镜像桌面 header）
                Row(verticalAlignment = Alignment.Top) {
                    Text(
                        "需要为您创建这个角色吗？",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(onClick = onDismiss, modifier = Modifier.size(28.dp)) {
                        Icon(
                            LucideIcons.X,
                            contentDescription = "关闭",
                            modifier = Modifier.size(18.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                // 预览块（桌面 slate-50 底：选中色底图标 + 名称/目标截断）
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f),
                    shape = RoundedCornerShape(12.dp), // 桌面 rounded-xl
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            Modifier
                                .size(44.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .background(parseHexColor(colorHex)),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                RoleIcons.getRoleIcon(iconId),
                                contentDescription = null,
                                modifier = Modifier.size(22.dp),
                                tint = Color.White,
                            )
                        }
                        Spacer(Modifier.size(12.dp))
                        Column {
                            Text(
                                trimmedName.ifEmpty { "（请输入角色名）" }, // 桌面占位文案
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurface,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            if (goal.trim().isNotEmpty()) {
                                Text(
                                    goal.trim(),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                    }
                }

                // 名称（桌面 maxLength 20）
                Column {
                    Text(
                        "名称",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.size(4.dp))
                    OutlinedTextField(
                        value = name,
                        onValueChange = { if (it.length <= 20) name = it },
                        placeholder = { Text("例如：产品经理") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }

                // 图标（24 白名单网格，桌面 grid-cols-8）
                Column {
                    Text(
                        "图标",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.size(4.dp))
                    RoleIconGrid(selectedId = iconId, onSelect = { iconId = it })
                }

                // 品牌色（8 色板）
                Column {
                    Text(
                        "品牌色",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.size(4.dp))
                    RoleColorPalette(selectedHex = colorHex, onSelect = { colorHex = it })
                }

                // 目标（可选，桌面 maxLength 80、两行）
                Column {
                    Text(
                        "目标（可选）",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.size(4.dp))
                    OutlinedTextField(
                        value = goal,
                        onValueChange = { if (it.length <= 80) goal = it },
                        placeholder = { Text("例如：打磨更好的产品，与用户共创") },
                        minLines = 2,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }

                // 底部按钮（桌面 justify-end：不需要 / 创建）
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                ) {
                    TextButton(onClick = onDismiss) {
                        Text("不需要")
                    }
                    Spacer(Modifier.size(8.dp))
                    Button(
                        onClick = { onConfirm(trimmedName, iconId, colorHex, goal.trim()) },
                        enabled = canConfirm,
                    ) {
                        Text("创建")
                    }
                }
            }
        }
    }
}

/** 图标 24 选网格（桌面 grid-cols-8：8 列 3 行，选中反色高亮）。 */
@Composable
private fun RoleIconGrid(selectedId: String, onSelect: (String) -> Unit) {
    RoleIcons.ROLE_ICONS.chunked(8).forEach { rowOptions ->
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            rowOptions.forEach { option ->
                val selected = option.id == selectedId
                Surface(
                    onClick = { onSelect(option.id) },
                    shape = RoundedCornerShape(8.dp), // 桌面 rounded-lg
                    color = if (selected) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.surface,
                    border = if (selected) BorderStroke(1.dp, MaterialTheme.colorScheme.primary)
                    else BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                    modifier = Modifier
                        .weight(1f)
                        .aspectRatio(1f),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            RoleIcons.getRoleIcon(option.id),
                            contentDescription = option.label,
                            modifier = Modifier.size(16.dp),
                            tint = if (selected) MaterialTheme.colorScheme.onPrimary
                            else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

/** 品牌色 8 选色板（桌面 flex 单行圆点；选中描边+放大镜像桌面 scale-110）。 */
@Composable
private fun RoleColorPalette(selectedHex: String, onSelect: (String) -> Unit) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        RoleIcons.ROLE_COLORS.forEach { option ->
            val selected = option.hex == selectedHex
            Surface(
                onClick = { onSelect(option.hex) },
                shape = CircleShape,
                color = parseHexColor(option.hex),
                border = if (selected) BorderStroke(2.dp, MaterialTheme.colorScheme.onSurface) else null,
                modifier = Modifier
                    .weight(1f)
                    .aspectRatio(1f)
                    .scale(if (selected) 1.1f else 1f),
            ) {}
        }
    }
}

/** 白名单 hex（#RRGGBB）→ Compose Color。normalizeColorHex 已保证格式；异常输入回退默认色。 */
internal fun parseHexColor(hex: String): Color {
    val value = hex.removePrefix("#").toLongOrNull(16) ?: 0x4F46E5
    return Color(
        red = ((value shr 16) and 0xFF).toInt() / 255f,
        green = ((value shr 8) and 0xFF).toInt() / 255f,
        blue = (value and 0xFF).toInt() / 255f,
    )
}
