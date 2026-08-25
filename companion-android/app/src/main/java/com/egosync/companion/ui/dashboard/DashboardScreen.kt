package com.egosync.companion.ui.dashboard

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.accent
import com.egosync.companion.ui.theme.energyColor

/**
 * ③ 仪表盘 Tab：角色卡横向滑动（图标、能量条呼吸动效、任务/记忆/会话统计数字）。
 * 信息密度控制台模式；呼吸感是全 App 唯一的"活物"动效。
 */
@Composable
fun DashboardScreen(
    uiState: DashboardUiState,
    unreadNoticeCount: Int,
    onOpenBriefing: () -> Unit,
    onOpenReview: () -> Unit,
    onOpenNotifications: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val pagerState = rememberPagerState(pageCount = { uiState.roles.size })

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp),
    ) {
        Spacer(Modifier.height(12.dp))

        // 管家概览卡：二级页入口（简报 / 周复盘 / 通知中心）
        ButlerOverviewCard(
            averageEnergy = uiState.averageEnergy,
            totalPending = uiState.totalPending,
            unreadNoticeCount = unreadNoticeCount,
            onOpenBriefing = onOpenBriefing,
            onOpenReview = onOpenReview,
            onOpenNotifications = onOpenNotifications,
        )

        Spacer(Modifier.height(18.dp))

        // 角色卡横向滑动
        HorizontalPager(
            state = pagerState,
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 4.dp),
        ) { page ->
            RoleCardItem(role = uiState.roles[page])
        }

        Spacer(Modifier.height(10.dp))

        // 页指示点
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.Center,
        ) {
            repeat(uiState.roles.size) { index ->
                val active = pagerState.currentPage == index
                Box(
                    Modifier
                        .padding(horizontal = 4.dp)
                        .size(if (active) 8.dp else 6.dp)
                        .clip(CircleShape)
                        .background(
                            if (active) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.outline
                        )
                )
            }
        }
        Spacer(Modifier.height(8.dp))
    }
}

// ── 管家概览卡 ─────────────────────────────────────────────────────────

@Composable
private fun ButlerOverviewCard(
    averageEnergy: Int,
    totalPending: Int,
    unreadNoticeCount: Int,
    onOpenBriefing: () -> Unit,
    onOpenReview: () -> Unit,
    onOpenNotifications: () -> Unit,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(18.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    Modifier
                        .size(40.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.primaryContainer),
                    contentAlignment = Alignment.Center,
                ) { Text("🤵", style = MaterialTheme.typography.titleLarge) }
                Spacer(Modifier.size(12.dp))
                Column(Modifier.weight(1f)) {
                    Text("管家 · 全局概览", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "角色平均能量 $averageEnergy% · 待办 $totalPending 项",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (unreadNoticeCount > 0) {
                    Box(
                        Modifier
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.error)
                            .padding(horizontal = 8.dp, vertical = 2.dp)
                    ) {
                        Text(
                            "$unreadNoticeCount",
                            style = MaterialTheme.typography.labelSmall,
                            color = Color.White,
                        )
                    }
                }
            }
            Spacer(Modifier.height(14.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OverviewEntry("📄 晨间简报", onOpenBriefing)
                OverviewEntry("📈 周复盘", onOpenReview)
                OverviewEntry("🔔 通知", onOpenNotifications)
            }
        }
    }
}

@Composable
private fun RowScope.OverviewEntry(text: String, onClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier
            .weight(1f)
            .clickable(onClick = onClick),
    ) {
        Text(
            text,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurface,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(vertical = 10.dp),
        )
    }
}

// ── 角色卡 ─────────────────────────────────────────────────────────────

@Composable
internal fun RoleCardItem(role: RoleCard, modifier: Modifier = Modifier) {
    val roleAccent = role.domain.accent()

    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(20.dp),
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 4.dp),
    ) {
        Column(Modifier.padding(18.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    Modifier
                        .size(52.dp)
                        .clip(CircleShape)
                        .background(roleAccent.tint),
                    contentAlignment = Alignment.Center,
                ) { Text(role.icon, style = MaterialTheme.typography.headlineSmall) }

                Spacer(Modifier.size(14.dp))
                Column(Modifier.weight(1f)) {
                    Text(role.name, style = MaterialTheme.typography.titleLarge)
                    Text(
                        "${role.domain.label} · 活跃于 ${role.lastActive}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Box(
                    Modifier
                        .clip(RoundedCornerShape(10.dp))
                        .background(roleAccent.tint)
                        .padding(horizontal = 10.dp, vertical = 4.dp)
                ) {
                    Text(
                        "待办 ${role.pendingCount}",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            Spacer(Modifier.height(18.dp))

            // 能量条（呼吸动效：活的实体）
            BreathingEnergyBar(energy = role.energy, accent = energyColor(role.energy))

            Spacer(Modifier.height(20.dp))

            // 统计数字行（信息密集 · 控制台模式）
            Row(Modifier.fillMaxWidth()) {
                StatCell("任务", role.taskCount, Modifier.weight(1f))
                StatCell("记忆", role.memoryCount, Modifier.weight(1f))
                StatCell("会话", role.sessionCount, Modifier.weight(1f))
                StatCell("待办", role.pendingCount, Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun StatCell(label: String, value: Int, modifier: Modifier = Modifier) {
    Column(modifier, horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            "$value",
            style = MaterialTheme.typography.titleLarge,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/**
 * 呼吸能量条：填充部分 alpha 0.65↔1.0 缓慢起伏（2.2s 周期）。
 * 全 App 唯一的装饰性动效（"角色卡片有呼吸感——活的实体"）。
 */
@Composable
internal fun BreathingEnergyBar(energy: Int, accent: Color, modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "breath")
    val breath by transition.animateFloat(
        initialValue = 0.65f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(1100), RepeatMode.Reverse),
        label = "breathAlpha",
    )

    Column(modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                "能量",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.weight(1f))
            Text(
                "$energy%",
                style = MaterialTheme.typography.labelLarge,
                color = accent,
            )
        }
        Spacer(Modifier.height(6.dp))
        Box(
            Modifier
                .fillMaxWidth()
                .height(8.dp)
                .clip(RoundedCornerShape(4.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant)
        ) {
            Box(
                Modifier
                    .fillMaxWidth(energy / 100f)
                    .height(8.dp)
                    .clip(RoundedCornerShape(4.dp))
                    .background(accent.copy(alpha = breath))
            )
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun DashboardScreenPreview() {
    EgoSyncTheme {
        DashboardScreen(
            uiState = DashboardUiState.sample(),
            unreadNoticeCount = 3,
            onOpenBriefing = {},
            onOpenReview = {},
            onOpenNotifications = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun RoleCardPreview() {
    EgoSyncTheme {
        RoleCardItem(role = SnapshotStore.roles.first(), modifier = Modifier.padding(12.dp))
    }
}
