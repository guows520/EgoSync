package com.egosync.companion.ui.review

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.BrandWarn
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.energyColor

/**
 * 二级页 · 周复盘（FR-18 移动呈现）：
 * 正向叙事成绩单 + 能量趋势柱状图（Canvas 自绘）+ 大石头推进结果。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WeeklyReviewScreen(
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val review = SnapshotStore.weeklyReview

    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("周复盘") },
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
            Text(
                review.weekLabel,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 4.dp),
            )
            Spacer(Modifier.height(10.dp))

            // 正向叙事（"看看你的进步"，绝不制造被评判感）
            Surface(
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(10.dp),
            ) {
                Row(Modifier.padding(14.dp)) {
                    Text("🤵", style = MaterialTheme.typography.titleLarge)
                    Spacer(Modifier.size(12.dp))
                    Text(
                        review.narrative,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // 成绩单
            Text(
                "本周成绩单",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
            )
            Surface(
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(vertical = 8.dp)) {
                    review.roleScores.forEachIndexed { index, score ->
                        RoleScoreRow(score)
                        if (index < review.roleScores.lastIndex) {
                            androidx.compose.material3.HorizontalDivider(
                                color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f),
                                modifier = Modifier.padding(horizontal = 14.dp),
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(16.dp))

            // 能量趋势柱状图（Canvas 自绘）
            Text(
                "能量趋势（近 7 天）",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
            )
            Surface(
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(14.dp)) {
                    EnergyTrendChart(
                        values = review.energyTrend,
                        dayLabels = review.energyTrendDays,
                    )
                }
            }

            Spacer(Modifier.height(16.dp))

            // 大石头推进（✓ 完成 / → 继续推进，不是失败）
            Text(
                "大石头推进",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
            )
            Surface(
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    review.bigRockResults.forEach { rock ->
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                if (rock.completed) "✓" else "→",
                                style = MaterialTheme.typography.titleMedium,
                                color = if (rock.completed) BrandGreen
                                else MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(Modifier.size(10.dp))
                            Text(
                                rock.text,
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (rock.completed) MaterialTheme.colorScheme.onSurface
                                else MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(16.dp))

            // 管家引导反思
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer,
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    review.closingQuestion,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    modifier = Modifier.padding(14.dp),
                )
            }
            Spacer(Modifier.height(24.dp))
        }
    }
}

// ── 成绩单行 ───────────────────────────────────────────────────────────

@Composable
private fun RoleScoreRow(score: com.egosync.companion.sync.RoleScore) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 14.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(score.roleName, style = MaterialTheme.typography.bodyMedium)
            Text(
                "完成 ${score.completed}/${score.total} 项任务",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(
            "能量 ${score.energyNow}%",
            style = MaterialTheme.typography.labelMedium,
            color = energyColor(score.energyNow),
        )
        Spacer(Modifier.size(8.dp))
        val deltaColor = if (score.energyDelta >= 0) BrandGreen else BrandWarn
        Text(
            (if (score.energyDelta >= 0) "↑" else "↓") + kotlin.math.abs(score.energyDelta),
            style = MaterialTheme.typography.labelMedium,
            color = deltaColor,
        )
    }
}

// ── 能量趋势柱状图（Canvas 自绘）───────────────────────────────────────

/**
 * 自绘柱状图：7 根能量柱 + 周标签。
 * 无第三方图表库——动效克制，静态柱形即可表达趋势。
 */
@Composable
internal fun EnergyTrendChart(
    values: List<Int>,
    dayLabels: List<String>,
    modifier: Modifier = Modifier,
) {
    val barColor = MaterialTheme.colorScheme.primary
    val trackColor = MaterialTheme.colorScheme.surfaceVariant
    val labelColor = MaterialTheme.colorScheme.onSurfaceVariant

    Column(modifier.fillMaxWidth()) {
        Canvas(
            Modifier
                .fillMaxWidth()
                .height(120.dp)
        ) {
            val count = values.size.coerceAtLeast(1)
            val slotWidth = size.width / count
            val barWidth = slotWidth * 0.42f
            val gap = (slotWidth - barWidth) / 2
            val chartHeight = size.height

            values.forEachIndexed { index, value ->
                val barHeight = (value / 100f) * chartHeight
                val left = index * slotWidth + gap
                // 背景轨道（满格暗柱）
                drawRoundRect(
                    color = trackColor,
                    topLeft = Offset(left, 0f),
                    size = Size(barWidth, chartHeight),
                    cornerRadius = CornerRadius(barWidth / 3),
                )
                // 数值柱
                drawRoundRect(
                    color = barColor.copy(alpha = 0.35f + 0.65f * (value / 100f)),
                    topLeft = Offset(left, chartHeight - barHeight),
                    size = Size(barWidth, barHeight),
                    cornerRadius = CornerRadius(barWidth / 3),
                )
            }
        }
        Spacer(Modifier.height(6.dp))
        Row(Modifier.fillMaxWidth()) {
            dayLabels.forEach { label ->
                Text(
                    label,
                    style = MaterialTheme.typography.labelSmall,
                    color = labelColor,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun WeeklyReviewScreenPreview() {
    EgoSyncTheme {
        WeeklyReviewScreen(onBack = {})
    }
}

@Preview(showBackground = true)
@Composable
private fun EnergyTrendChartPreview() {
    EgoSyncTheme {
        Surface(color = MaterialTheme.colorScheme.surface) {
            EnergyTrendChart(
                values = SnapshotStore.weeklyReview.energyTrend,
                dayLabels = SnapshotStore.weeklyReview.energyTrendDays,
                modifier = Modifier.padding(16.dp),
            )
        }
    }
}
