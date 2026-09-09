package com.egosync.companion.ui.notify

import com.egosync.companion.ui.icons.LucideIcons

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.NoticeItem
import com.egosync.companion.sync.NoticeLevel
import com.egosync.companion.ui.previewNotices
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandError
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.QuadrantGray

/**
 * 二级页 · 通知中心（FR-22 V1 应用内形态）：whisper/tap/knock 三级分组列表 + 未读圆点。
 * 敲门级通知附确认/拒绝快捷操作（等效 App 内 ActionCard）。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotificationCenterScreen(
    notices: List<NoticeItem>,
    commandReady: Boolean,
    onBack: () -> Unit,
    onMarkAllRead: () -> Unit,
    onRespond: (id: String, confirmed: Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val unread = notices.count { !it.read }

    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("通知中心") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
        floatingActionButton = {
            if (unread > 0) {
                Button(onClick = onMarkAllRead) { Text("全部已读") }
            }
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
        ) {
            NoticeLevel.entries.forEach { level ->
                val levelNotices = notices.filter { it.level == level }
                if (levelNotices.isEmpty()) return@forEach

                item(key = "header-${level.name}") {
                    LevelHeader(level, levelNotices.count { !it.read })
                }
                items(levelNotices.size, key = { levelNotices[it].id }) { index ->
                    NoticeRow(
                        notice = levelNotices[index],
                        commandReady = commandReady,
                        onRespond = { confirmed -> onRespond(levelNotices[index].id, confirmed) },
                    )
                    Spacer(Modifier.height(8.dp))
                }
            }
            item { Spacer(Modifier.height(80.dp)) }
        }
    }
}

@Composable
private fun LevelHeader(level: NoticeLevel, unreadCount: Int) {
    // 母本 NotificationPanel levelConfig：whisper 灰 / tap 蓝 / knock 红
    val color = when (level) {
        NoticeLevel.WHISPER -> QuadrantGray
        NoticeLevel.TAP -> BrandBlue
        NoticeLevel.KNOCK -> BrandError
    }
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 10.dp, bottom = 8.dp),
    ) {
        Icon(LucideIcons.Bell, contentDescription = level.label, modifier = Modifier.size(20.dp), tint = color)
        Spacer(Modifier.size(8.dp))
        Text(
            "${level.label} · ${level.description}",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        if (unreadCount > 0) {
            Box(
                Modifier
                    .size(8.dp)
                    .clip(CircleShape)
                    .background(color)
            )
        }
    }
}

/** 级别徽章 — 母本 NotificationPanel：whisper 灰底灰字 / tap 蓝底蓝字 / knock 红底红字。 */
@Composable
private fun LevelBadge(level: NoticeLevel) {
    val (label, fg, bg) = when (level) {
        NoticeLevel.WHISPER ->
            Triple(level.label, QuadrantGray, QuadrantGray.copy(alpha = 0.15f))
        NoticeLevel.TAP ->
            Triple(level.label, BrandBlue, BrandBlue.copy(alpha = 0.12f))
        NoticeLevel.KNOCK ->
            Triple(level.label, BrandError, BrandError.copy(alpha = 0.12f))
    }
    Box(
        Modifier
            .clip(RoundedCornerShape(4.dp))
            .background(bg)
            .padding(horizontal = 6.dp, vertical = 2.dp)
    ) {
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = fg,
        )
    }
}

@Composable
private fun NoticeRow(
    notice: NoticeItem,
    commandReady: Boolean,
    onRespond: (Boolean) -> Unit,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (!notice.read) {
                    Box(
                        Modifier
                            .size(8.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary)
                    )
                    Spacer(Modifier.size(8.dp))
                }
                Text(
                    notice.fromRole,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.size(6.dp))
                LevelBadge(notice.level)
                Spacer(Modifier.weight(1f))
                Text(
                    notice.time,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.height(6.dp))
            Text(
                notice.text,
                style = MaterialTheme.typography.bodyMedium,
                color = if (notice.read) MaterialTheme.colorScheme.onSurfaceVariant
                else MaterialTheme.colorScheme.onSurface,
            )

            // 敲门级：待决策 ActionCard（确认/拒绝）
            if (notice.actionable) {
                Spacer(Modifier.height(10.dp))
                when (notice.actionState) {
                    null -> {
                        Row {
                            Button(
                                onClick = { onRespond(true) },
                                enabled = commandReady,
                            ) { Text("确认") }
                            Spacer(Modifier.size(10.dp))
                            OutlinedButton(
                                onClick = { onRespond(false) },
                                enabled = commandReady,
                            ) { Text("拒绝") }
                        }
                        if (!commandReady) {
                            Spacer(Modifier.height(4.dp))
                            Text(
                                "桌面引擎离线，暂无法处理敲门通知",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    true -> Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(16.dp), tint = BrandGreen)
                        Spacer(Modifier.size(4.dp))
                        Text("已确认", style = MaterialTheme.typography.labelMedium, color = BrandGreen)
                    }
                    false -> Text(
                        "已拒绝",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun NotificationCenterScreenPreview() {
    EgoSyncTheme {
        NotificationCenterScreen(
            notices = previewNotices,
            commandReady = true,
            onBack = {},
            onMarkAllRead = {},
            onRespond = { _, _ -> },
        )
    }
}
