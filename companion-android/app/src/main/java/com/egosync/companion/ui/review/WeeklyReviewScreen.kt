package com.egosync.companion.ui.review

import com.egosync.companion.ui.icons.LucideIcons

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.ui.previewRoles
import com.egosync.companion.ui.previewWeeklyReview
import com.egosync.companion.sync.WeeklyReview
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandGreen
import com.egosync.companion.ui.theme.BrandIndigo
import com.egosync.companion.ui.theme.BrandWarn
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.accent
import com.egosync.companion.ui.theme.energyColor

/**
 * 二级页 · 周复盘（FR-18 移动呈现 + FR-17 大石头规划）：
 * review 阶段 = 正向叙事成绩单 + 能量趋势柱状图 + 大石头推进结果；
 * plan 阶段 = 逐角色建议采纳 / 输入 / 添加 / 移除 + 确认规划（镜像桌面 WeeklyReviewModal L181-262）。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WeeklyReviewScreen(
    uiState: WeeklyReviewUiState,
    review: WeeklyReview,
    roles: List<RoleCard>,
    dataCutoffLabel: String?,
    onBack: () -> Unit,
    onEnterPlan: () -> Unit,
    onBackToReview: () -> Unit,
    onUpdateItem: (roleId: String, index: Int, text: String) -> Unit,
    onRemoveItem: (roleId: String, index: Int) -> Unit,
    onAddItem: (roleId: String) -> Unit,
    onAdoptSuggestion: (roleId: String, suggestion: String) -> Unit,
    onConfirmPlan: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Scaffold(
        modifier = modifier,
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text(if (uiState.phase == ReviewPhase.REVIEW) "周复盘" else "规划下周大石头") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
            )
        },
    ) { padding ->
        if (uiState.phase == ReviewPhase.REVIEW) {
            ReviewPhaseContent(
                review = review,
                dataCutoffLabel = dataCutoffLabel,
                padding = padding,
                onEnterPlan = onEnterPlan,
            )
        } else {
            PlanPhaseContent(
                uiState = uiState,
                roles = roles,
                padding = padding,
                onBackToReview = onBackToReview,
                onUpdateItem = onUpdateItem,
                onRemoveItem = onRemoveItem,
                onAddItem = onAddItem,
                onAdoptSuggestion = onAdoptSuggestion,
                onConfirmPlan = onConfirmPlan,
            )
        }
    }
}

// ── review 阶段（既有只读成绩单 + 规划入口）──────────────────────────────

@Composable
private fun ReviewPhaseContent(
    review: WeeklyReview,
    dataCutoffLabel: String?,
    padding: PaddingValues,
    onEnterPlan: () -> Unit,
) {
    if (review.weekLabel.isEmpty()) {
        // 快照未加载/本周复盘未生成：诚实占位（NFR-M3），不以空成绩单冒充「已生成」（评审 P8）
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.Center,
        ) {
            Text(
                "本周复盘尚未生成。连接桌面完成一周回顾后，成绩单会出现在这里。",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        return
    }
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
        // 截断提示：桌面因域被截断而未含完整数据时明示（§5 诚实降级）
        if (dataCutoffLabel != null) {
            Spacer(Modifier.height(4.dp))
            Row(
                Modifier.padding(start = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    LucideIcons.AlertTriangle,
                    contentDescription = null,
                    modifier = Modifier.size(12.dp),
                    tint = MaterialTheme.colorScheme.error,
                )
                Spacer(Modifier.size(4.dp))
                Text(
                    "数据截至 $dataCutoffLabel（桌面端部分域被截断）",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
        Spacer(Modifier.height(10.dp))

        // 正向叙事（"看看你的进步"，绝不制造被评判感）
        Surface(
            color = MaterialTheme.colorScheme.surface,
            shape = RoundedCornerShape(10.dp),
        ) {
            Row(Modifier.padding(14.dp)) {
                Icon(LucideIcons.Home, contentDescription = "管家", modifier = Modifier.size(24.dp))
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

        // 能量趋势柱状图（Canvas 自绘）；快照无逐日时间序列（§5 裁决①）→ 诚实空态，
        // 图表本体保留（Preview 样例仍渲染，视觉基准不丢）
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
            if (review.energyTrend.isEmpty()) {
                Text(
                    "暂无逐日能量数据（当前快照仅含角色当前能量）",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(14.dp),
                )
            } else {
                Column(Modifier.padding(14.dp)) {
                    EnergyTrendChart(
                        values = review.energyTrend,
                        dayLabels = review.energyTrendDays,
                    )
                }
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
                        Icon(
                            if (rock.completed) LucideIcons.Check else LucideIcons.ArrowRight,
                            contentDescription = if (rock.completed) "完成" else "推进中",
                            modifier = Modifier.size(20.dp),
                            tint = if (rock.completed) BrandGreen
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

        // 规划下周大石头入口（镜像桌面 Modal L177-179：居中 indigo 主按钮 + ArrowRight）
        Button(
            onClick = onEnterPlan,
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Icon(LucideIcons.ArrowRight, contentDescription = null, modifier = Modifier.size(16.dp))
            Spacer(Modifier.size(6.dp))
            Text("规划下周大石头")
        }
        Spacer(Modifier.height(24.dp))
    }
}

// ── plan 阶段（FR-17 大石头规划，镜像桌面 Modal L181-262）───────────────

@Composable
private fun PlanPhaseContent(
    uiState: WeeklyReviewUiState,
    roles: List<RoleCard>,
    padding: PaddingValues,
    onBackToReview: () -> Unit,
    onUpdateItem: (roleId: String, index: Int, text: String) -> Unit,
    onRemoveItem: (roleId: String, index: Int) -> Unit,
    onAddItem: (roleId: String) -> Unit,
    onAdoptSuggestion: (roleId: String, suggestion: String) -> Unit,
    onConfirmPlan: () -> Unit,
) {
    Column(
        Modifier
            .fillMaxSize()
            .padding(padding)
            .imePadding() // 软键盘弹起时避让，防底部「确认规划」被遮（同 ChatScreen 输入条模式）
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp),
    ) {
        // info 条（桌面 indigo-50 底 / indigo-100 边 / indigo-800 字）
        Surface(
            color = BrandIndigo.copy(alpha = 0.08f),
            shape = RoundedCornerShape(10.dp),
            border = BorderStroke(1.dp, BrandIndigo.copy(alpha = 0.25f)),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(
                "为每个角色设定下周最重要的大石头，系统会在日程中优先为它们保留时间。",
                style = MaterialTheme.typography.bodySmall,
                color = BrandIndigo,
                modifier = Modifier.padding(14.dp),
            )
        }

        Spacer(Modifier.height(16.dp))

        roles.forEach { role ->
            val planState = uiState.planStates.firstOrNull { it.roleId == role.id }
                ?: RolePlanState(role.id, listOf(""))
            RolePlanCard(
                role = role,
                planState = planState,
                isLoadingSuggestions = uiState.isLoadingSuggestions,
                suggestions = uiState.suggestionsOf(role.id),
                onUpdateItem = onUpdateItem,
                onRemoveItem = onRemoveItem,
                onAddItem = onAddItem,
                onAdoptSuggestion = onAdoptSuggestion,
            )
            Spacer(Modifier.height(14.dp))
        }

        Spacer(Modifier.height(8.dp))

        // 底部双按钮（镜像桌面 L251-259：返回复盘 / 确认规划）
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedButton(onClick = onBackToReview) {
                Text("返回复盘")
            }
            Button(onClick = onConfirmPlan, enabled = !uiState.isSaving) {
                Icon(LucideIcons.Check, contentDescription = null, modifier = Modifier.size(16.dp))
                Spacer(Modifier.size(6.dp))
                Text(if (uiState.isSaving) "保存中..." else "确认规划")
            }
        }
        Spacer(Modifier.height(24.dp))
    }
}

// ── 角色规划卡（建议区 + 输入区，镜像桌面 Modal L187-246）───────────────

@Composable
private fun RolePlanCard(
    role: RoleCard,
    planState: RolePlanState,
    isLoadingSuggestions: Boolean,
    suggestions: List<String>,
    onUpdateItem: (roleId: String, index: Int, text: String) -> Unit,
    onRemoveItem: (roleId: String, index: Int) -> Unit,
    onAddItem: (roleId: String) -> Unit,
    onAdoptSuggestion: (roleId: String, suggestion: String) -> Unit,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.5f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp)) {
            // 角色名（桌面 normalizeColorHex(role.color) → 移动域色温，见 spec Design Notes）
            Text(
                role.name,
                style = MaterialTheme.typography.titleSmall,
                color = role.domain.accent().accent,
            )
            Spacer(Modifier.height(10.dp))

            // 建议区：加载中 / 建议行 / 手动填写占位（三分支镜像桌面 L194-212）
            if (isLoadingSuggestions) {
                // 兼容分支：13.2 不再模拟 LLM 思考，恒不进此分支；保留以防后续接入复用
                SuggestionShell {
                    Text(
                        "正在思考建议...",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else if (suggestions.isNotEmpty()) {
                suggestions.forEach { suggestion ->
                    SuggestionShell {
                        Row(
                            Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                LucideIcons.Lightbulb,
                                contentDescription = "建议",
                                modifier = Modifier.size(14.dp),
                                tint = BrandIndigo,
                            )
                            Spacer(Modifier.size(8.dp))
                            Text(
                                suggestion,
                                style = MaterialTheme.typography.bodySmall,
                                color = BrandIndigo,
                                modifier = Modifier.weight(1f),
                            )
                            TextButton(
                                onClick = { onAdoptSuggestion(role.id, suggestion) },
                                contentPadding = PaddingValues(horizontal = 8.dp),
                            ) {
                                Text("采纳", style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                }
            } else {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        "暂无建议，可手动填写本周大石头",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    )
                }
            }

            Spacer(Modifier.height(10.dp))

            // 输入行：OutlinedTextField + X 移除（>1 行才显示，镜像桌面 L214-236）
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                planState.items.forEachIndexed { index, text ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        OutlinedTextField(
                            value = text,
                            onValueChange = { onUpdateItem(role.id, index, it) },
                            placeholder = { Text("${role.name} 下周重要的事...") },
                            modifier = Modifier.weight(1f),
                            singleLine = true,
                            shape = RoundedCornerShape(10.dp),
                        )
                        if (planState.items.size > 1) {
                            IconButton(onClick = { onRemoveItem(role.id, index) }) {
                                Icon(
                                    LucideIcons.X,
                                    contentDescription = "移除",
                                    modifier = Modifier.size(16.dp),
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }

            // 添加（镜像桌面 L237-242：Plus + indigo 文字按钮）
            TextButton(
                onClick = { onAddItem(role.id) },
                contentPadding = PaddingValues(horizontal = 4.dp),
            ) {
                Icon(
                    LucideIcons.Plus,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = BrandIndigo,
                )
                Spacer(Modifier.size(6.dp))
                Text("添加", style = MaterialTheme.typography.labelLarge, color = BrandIndigo)
            }
        }
    }
}

/** 建议行外壳（桌面 indigo-50/60 底 + indigo-100 边）。 */
@Composable
private fun SuggestionShell(content: @Composable () -> Unit) {
    Surface(
        color = BrandIndigo.copy(alpha = 0.06f),
        shape = RoundedCornerShape(8.dp),
        border = BorderStroke(1.dp, BrandIndigo.copy(alpha = 0.25f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        androidx.compose.foundation.layout.Box(Modifier.padding(horizontal = 12.dp, vertical = 8.dp)) {
            content()
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
        WeeklyReviewScreen(
            uiState = WeeklyReviewUiState(),
            review = previewWeeklyReview,
            roles = previewRoles,
            dataCutoffLabel = null,
            onBack = {},
            onEnterPlan = {},
            onBackToReview = {},
            onUpdateItem = { _, _, _ -> },
            onRemoveItem = { _, _ -> },
            onAddItem = {},
            onAdoptSuggestion = { _, _ -> },
            onConfirmPlan = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun WeeklyReviewPlanScreenPreview() {
    EgoSyncTheme {
        WeeklyReviewScreen(
            uiState = WeeklyReviewUiState(
                phase = ReviewPhase.PLAN,
                isLoadingSuggestions = false,
                suggestions = emptyMap(),
            ),
            review = previewWeeklyReview,
            roles = previewRoles,
            dataCutoffLabel = null,
            onBack = {},
            onEnterPlan = {},
            onBackToReview = {},
            onUpdateItem = { _, _, _ -> },
            onRemoveItem = { _, _ -> },
            onAddItem = {},
            onAdoptSuggestion = { _, _ -> },
            onConfirmPlan = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun EnergyTrendChartPreview() {
    EgoSyncTheme {
        Surface(color = MaterialTheme.colorScheme.surface) {
            EnergyTrendChart(
                values = previewWeeklyReview.energyTrend,
                dayLabels = previewWeeklyReview.energyTrendDays,
                modifier = Modifier.padding(16.dp),
            )
        }
    }
}
