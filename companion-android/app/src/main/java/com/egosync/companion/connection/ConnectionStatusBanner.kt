package com.egosync.companion.connection

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandError
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.EgoSyncTheme

/**
 * 顶部三态连接横幅（FR-40）：局域网直连（绿）/ 中继转发（蓝）/ 离线（红）。
 * 颜色 + 圆点图标 + 状态文案，连接状态在手机端持续可见。
 */
@Composable
fun ConnectionStatusBanner(
    state: ConnectionState,
    modifier: Modifier = Modifier,
) {
    val (dotColor, title, subtitle) = when (state) {
        is ConnectionState.Direct -> Triple(
            BrandGreen, "局域网直连", "已连接桌面引擎 · 数据实时同步",
        )
        is ConnectionState.Relay -> Triple(
            BrandBlue, "中继转发", "经云端中继 · 端到端加密",
        )
        is ConnectionState.Offline -> Triple(
            BrandError, "离线",
            when {
                state.snapshotAvailable && state.dataAsOf != null ->
                    "桌面不可达 · 显示缓存数据（截至 ${state.dataAsOf}）"
                state.snapshotAvailable -> "桌面不可达 · 显示缓存数据"
                else -> "桌面不可达 · 暂无缓存数据"
            }
        )
    }

    Surface(
        modifier = modifier.fillMaxWidth(),
        color = dotColor.copy(alpha = 0.14f),
    ) {
        Row(
            modifier = Modifier
                .statusBarsPadding()
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                Modifier
                    .size(9.dp)
                    .clip(CircleShape)
                    .background(dotColor)
            )
            Spacer(Modifier.size(10.dp))
            Text(
                title,
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(Modifier.size(10.dp))
            Text(
                subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
                maxLines = 1,
            )
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun BannerDirectPreview() {
    EgoSyncTheme {
        ConnectionStatusBanner(state = ConnectionState.Direct)
    }
}

@Preview(showBackground = true)
@Composable
private fun BannerRelayPreview() {
    EgoSyncTheme {
        ConnectionStatusBanner(state = ConnectionState.Relay)
    }
}

@Preview(showBackground = true)
@Composable
private fun BannerOfflinePreview() {
    EgoSyncTheme {
        ConnectionStatusBanner(
            state = ConnectionState.Offline(snapshotAvailable = true, dataAsOf = "今天 08:15"),
        )
    }
}
