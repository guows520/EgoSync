package com.egosync.companion.ui.settings

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.connection.ConnectionState
import com.egosync.companion.connection.DebugConnectionMode
import com.egosync.companion.sync.NoticeLevel
import com.egosync.companion.sync.ProactivityLevel
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.icons.RoleIcons
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandError
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.ThemeMode
import com.egosync.companion.ui.theme.accent

/**
 * ④ 我的 Tab：设置列表——主题切换、通知级别开关（仅应用内语义）、
 * 角色主动性级别（FR-12：角色选择 + 三档分段）、配对设备、解除配对；
 * 隐藏"状态模拟"入口（连点版本号 7 次解锁）。
 */
@Composable
fun SettingsScreen(
    uiState: SettingsUiState,
    connectionState: ConnectionState,
    onSetTheme: (ThemeMode) -> Unit,
    onToggleNoticeLevel: (NoticeLevel) -> Unit,
    onProactivityRoleSelected: (String) -> Unit,
    onProactivityLevelSelected: (ProactivityLevel) -> Unit,
    onDebugModeSelected: (DebugConnectionMode) -> Unit,
    onVersionTapped: () -> String?,
    onUnpair: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var unlockHint by remember { mutableStateOf<String?>(null) }
    var showUnpairDialog by remember { mutableStateOf(false) }

    if (showUnpairDialog) {
        AlertDialog(
            onDismissRequest = { showUnpairDialog = false },
            title = { Text("解除配对？") },
            text = {
                Text("解除后手机将与桌面端断开绑定，缓存数据不再更新。重新扫码即可再次配对，桌面端无需重置。")
            },
            confirmButton = {
                TextButton(onClick = { showUnpairDialog = false; onUnpair() }) {
                    Text("解除配对", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showUnpairDialog = false }) { Text("取消") }
            },
        )
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp),
    ) {
        Spacer(Modifier.height(12.dp))

        SectionTitle("外观")
        Spacer(Modifier.height(8.dp))
        Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
            Column(Modifier.padding(16.dp)) {
                Text("主题", style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.height(4.dp))
                Text(
                    "深色默认（保护专注力），可切换浅色",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(10.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    ThemeOption(LucideIcons.Moon, "深色", ThemeMode.DARK, uiState.themeMode, onSetTheme, Modifier.weight(1f))
                    ThemeOption(LucideIcons.Sun, "浅色", ThemeMode.LIGHT, uiState.themeMode, onSetTheme, Modifier.weight(1f))
                }
            }
        }

        Spacer(Modifier.height(18.dp))

        SectionTitle("通知")
        Spacer(Modifier.height(8.dp))
        Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
            Column(Modifier.padding(vertical = 6.dp)) {
                Text(
                    "三级通知 · 仅应用内（系统推送将随后续版本提供）",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
                NoticeLevelRow("耳语 Whisper", "角色日常沉淀，静默积累进简报", uiState.whisperEnabled) {
                    onToggleNoticeLevel(NoticeLevel.WHISPER)
                }
                NoticeLevelRow("轻触 Tap", "管家顺嘴一提的动态", uiState.tapEnabled) {
                    onToggleNoticeLevel(NoticeLevel.TAP)
                }
                NoticeLevelRow("敲门 Knock", "需要你决定的事（每日上限 3 次）", uiState.knockEnabled) {
                    onToggleNoticeLevel(NoticeLevel.KNOCK)
                }
            }
        }

        Spacer(Modifier.height(18.dp))

        // ── FR-12 主动性级别：角色选择 chip 行 + 三档分段控件（镜像桌面 ProactivityToggle）──
        SectionTitle("主动性级别")
        Spacer(Modifier.height(8.dp))
        Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
            Column(Modifier.padding(16.dp)) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    uiState.proactivityRoles.forEach { role ->
                        ProactivityRoleChip(
                            role = role,
                            selected = uiState.proactivityRoleId == role.id,
                            onClick = { onProactivityRoleSelected(role.id) },
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
                Spacer(Modifier.height(12.dp))
                SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                    ProactivityLevel.entries.forEach { level ->
                        SegmentedButton(
                            selected = uiState.selectedProactivity == level,
                            onClick = { onProactivityLevelSelected(level) },
                            shape = SegmentedButtonDefaults.itemShape(
                                index = level.ordinal,
                                count = ProactivityLevel.entries.size,
                            ),
                        ) {
                            Text(level.label)
                        }
                    }
                }
            }
        }

        Spacer(Modifier.height(18.dp))

        SectionTitle("配对设备")
        Spacer(Modifier.height(8.dp))
        Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        Modifier
                            .size(44.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primaryContainer),
                        contentAlignment = Alignment.Center,
                    ) { Icon(LucideIcons.Monitor, contentDescription = null, modifier = Modifier.size(24.dp)) }
                    Spacer(Modifier.size(12.dp))
                    Column(Modifier.weight(1f)) {
                        Text("桌面端 EgoSync", style = MaterialTheme.typography.titleMedium)
                        Text(
                            "已配对 · ${connectionStateLabel(connectionState)}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    ConnectionDot(connectionState)
                }
                Spacer(Modifier.height(6.dp))
                Text(
                    "V1 支持绑定一台桌面设备；重装 App 后重新扫码即可恢复，桌面端无需重置。",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Spacer(Modifier.height(8.dp))
        OutlinedButton(
            onClick = { showUnpairDialog = true },
            modifier = Modifier.fillMaxWidth(),
        ) { Text("解除配对", color = MaterialTheme.colorScheme.error) }

        Spacer(Modifier.height(18.dp))

        // ── 隐藏入口：状态模拟（连点版本号 7 次解锁）──
        if (uiState.debugUnlocked) {
            SectionTitle("状态模拟（Debug）")
            Spacer(Modifier.height(8.dp))
            Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
                Column(Modifier.padding(16.dp)) {
                    Text(
                        "手动切换连接状态，驱动降级态遮罩与操作禁用态实时变化。",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(10.dp))
                    DebugConnectionMode.entries.forEach { mode ->
                        DebugModeRow(
                            mode = mode,
                            selected = uiState.debugMode == mode,
                            onClick = { onDebugModeSelected(mode) },
                        )
                    }
                }
            }
            Spacer(Modifier.height(18.dp))
        }

        SectionTitle("关于")
        Spacer(Modifier.height(8.dp))
        Surface(color = MaterialTheme.colorScheme.surface, shape = RoundedCornerShape(10.dp)) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .clickable { unlockHint = onVersionTapped() }
                    .padding(16.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("EgoSync 伴侣（高保真原型）", style = MaterialTheme.typography.titleSmall)
                Spacer(Modifier.height(2.dp))
                Text(
                    "版本 0.1.0 · 纯前端 + Mock 数据",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                unlockHint?.let {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        it,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }
        }

        Spacer(Modifier.height(32.dp))
    }
}

// ── 组件 ───────────────────────────────────────────────────────────────

@Composable
private fun SectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 4.dp),
    )
}

@Composable
private fun ThemeOption(
    icon: ImageVector,
    label: String,
    mode: ThemeMode,
    current: ThemeMode,
    onSetTheme: (ThemeMode) -> Unit,
    modifier: Modifier = Modifier,
) {
    val selected = mode == current
    Surface(
        color = if (selected) MaterialTheme.colorScheme.primaryContainer
        else MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(10.dp),
        modifier = modifier.clickable { onSetTheme(mode) },
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(icon, contentDescription = null, modifier = Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onSurface)
            Spacer(Modifier.size(4.dp))
            Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurface)
        }
    }
}

/**
 * FR-12 角色 chip：视觉语言复用聊天屏角色切换器（getRoleIcon + 域 accent，选中 primary 高亮）。
 * 与 ChatScreen.RoleChip 保持同一套观感；独立私有实现以维持各屏自包含的既有组织方式。
 */
@Composable
private fun ProactivityRoleChip(
    role: RoleCard,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp),
        color = if (selected) MaterialTheme.colorScheme.primary
        else MaterialTheme.colorScheme.surfaceVariant,
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
        modifier = modifier,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                RoleIcons.getRoleIcon(role.icon),
                contentDescription = role.name,
                modifier = Modifier.size(14.dp),
                tint = if (selected) MaterialTheme.colorScheme.onPrimary
                else role.domain.accent().accent,
            )
            Spacer(Modifier.size(6.dp))
            Text(
                role.name,
                style = MaterialTheme.typography.labelMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = if (selected) MaterialTheme.colorScheme.onPrimary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun NoticeLevelRow(
    title: String,
    subtitle: String,
    enabled: Boolean,
    onToggle: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.bodyMedium)
            Text(
                subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Switch(checked = enabled, onCheckedChange = { onToggle() })
    }
}

@Composable
private fun DebugModeRow(mode: DebugConnectionMode, selected: Boolean, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(
                if (selected) MaterialTheme.colorScheme.primaryContainer
                else MaterialTheme.colorScheme.surfaceVariant
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            mode.label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        if (selected) {
            Spacer(Modifier.weight(1f))
            Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
        }
    }
}

@Composable
private fun ConnectionDot(state: ConnectionState) {
    val color = when (state) {
        is ConnectionState.Direct -> BrandGreen
        is ConnectionState.Relay -> BrandBlue
        is ConnectionState.Offline -> BrandError
    }
    Box(
        Modifier
            .size(10.dp)
            .clip(CircleShape)
            .background(color)
    )
}

private fun connectionStateLabel(state: ConnectionState): String = when (state) {
    is ConnectionState.Direct -> "局域网直连"
    is ConnectionState.Relay -> "中继转发"
    is ConnectionState.Offline -> "离线"
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun SettingsScreenPreview() {
    EgoSyncTheme {
        SettingsScreen(
            uiState = SettingsUiState.sample(),
            connectionState = ConnectionState.Direct,
            onSetTheme = {},
            onToggleNoticeLevel = {},
            onProactivityRoleSelected = {},
            onProactivityLevelSelected = {},
            onDebugModeSelected = {},
            onVersionTapped = { null },
            onUnpair = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun SettingsScreenDebugPreview() {
    EgoSyncTheme {
        SettingsScreen(
            uiState = SettingsUiState.sample().copy(
                debugUnlocked = true,
                debugMode = DebugConnectionMode.DEGRADED,
            ),
            connectionState = ConnectionState.Offline(snapshotAvailable = true, dataAsOf = "今天 08:15"),
            onSetTheme = {},
            onToggleNoticeLevel = {},
            onProactivityRoleSelected = {},
            onProactivityLevelSelected = {},
            onDebugModeSelected = {},
            onVersionTapped = { null },
            onUnpair = {},
        )
    }
}
