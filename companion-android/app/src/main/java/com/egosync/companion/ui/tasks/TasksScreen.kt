package com.egosync.companion.ui.tasks

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.material3.Icon
import com.egosync.companion.ui.icons.LucideIcons

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
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
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.SnapshotStore
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.sync.TaskProtectionStatus
import com.egosync.companion.ui.theme.BrandIndigo
import com.egosync.companion.ui.theme.BrandWarn
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.color
import com.egosync.companion.ui.theme.rememberReducedMotion

/**
 * ② 任务 Tab：四象限分组列表（Q1~Q4 分色、大石头星标、勾选完成交互）。
 */
@Composable
fun TasksScreen(
    uiState: TasksUiState,
    engineAvailable: Boolean,
    onToggleTask: (taskId: String) -> Unit,
    onQuadrantFilterSelected: (Quadrant?) -> Unit,
    modifier: Modifier = Modifier,
) {
    val grouped = uiState.grouped()

    Column(modifier = modifier.fillMaxSize()) {
        // FR-23 象限筛选行（镜像桌面 TaskOverviewTab.tsx:286-301：Filter 图标 + 全部/Q1~Q4 单选 chip）
        QuadrantFilterRow(
            selected = uiState.quadrantFilter,
            onSelect = onQuadrantFilterSelected,
            modifier = Modifier.padding(horizontal = 16.dp),
        )
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                horizontal = 16.dp, vertical = 12.dp,
            ),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (!engineAvailable) {
                item(key = "offline-hint") {
                    Text(
                        "桌面引擎离线：任务操作需桌面引擎在线，暂不可交互。",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (grouped.values.all { it.isEmpty() }) {
                item(key = "empty-state") {
                    Text(
                        // 镜像桌面 TaskOverviewTab.tsx:354-357 空态文案分支
                        if (uiState.quadrantFilter == null) "所有角色都很轻松，可以考虑添加新目标"
                        else "当前筛选无匹配任务",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Quadrant.entries.forEach { quadrant ->
                val tasks = grouped[quadrant].orEmpty()
                if (tasks.isNotEmpty()) {
                    item(key = quadrant.code) {
                        QuadrantHeader(quadrant, tasks.size)
                    }
                    items(tasks.size, key = { tasks[it].id }) { index ->
                        TaskRow(
                            task = tasks[index],
                            enabled = engineAvailable,
                            isClassifying = uiState.classifyingIds.contains(tasks[index].id),
                            onToggle = { onToggleTask(tasks[index].id) },
                        )
                    }
                }
            }
        }
    }
}

/** 象限筛选行：Filter 图标 + 全部/Q1~Q4 单选 chip（镜像桌面 quadrantChips，选中 indigo 高亮）。 */
@Composable
private fun QuadrantFilterRow(
    selected: Quadrant?,
    onSelect: (Quadrant?) -> Unit,
    modifier: Modifier = Modifier,
) {
    // 横向滚动承载（同组 1 RoleSwitcherRow）：窄屏/大字体下末尾 chip 仍可达
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 12.dp),
    ) {
        Icon(
            LucideIcons.Filter,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.size(6.dp))
        LazyRow(
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            modifier = Modifier.weight(1f),
        ) {
            item(key = "all") {
                QuadrantFilterChip(
                    label = "全部",
                    selected = selected == null,
                    onClick = { onSelect(null) },
                )
            }
            items(Quadrant.entries.size, key = { Quadrant.entries[it].code }) { index ->
                val quadrant = Quadrant.entries[index]
                QuadrantFilterChip(
                    label = quadrant.code,
                    selected = selected == quadrant,
                    onClick = { onSelect(quadrant) },
                )
            }
        }
    }
}

@Composable
private fun QuadrantFilterChip(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
) {
    // 选中态同组 1 RoleChip 母本：primary 底 + onPrimary 字 + primary 描边
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface,
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
    ) {
        Text(
            label,
            style = MaterialTheme.typography.labelMedium,
            color = if (selected) MaterialTheme.colorScheme.onPrimary
            else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        )
    }
}

@Composable
private fun QuadrantHeader(quadrant: Quadrant, count: Int) {
    val color = quadrant.color()
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(color.copy(alpha = 0.12f))
            .padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        Box(
            Modifier
                .size(10.dp)
                .clip(CircleShape)
                .background(color)
        )
        Spacer(Modifier.size(10.dp))
        Text(
            "${quadrant.code} ${quadrant.title}",
            style = MaterialTheme.typography.titleSmall,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.size(8.dp))
        Text(
            quadrant.subtitle,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.weight(1f))
        Text(
            "$count 项",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun TaskRow(
    task: TaskItem,
    enabled: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
    isClassifying: Boolean = false,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp),
        modifier = modifier
            .fillMaxWidth()
            .then(
                // FR-24 at_risk 左边框（母本 TaskOverviewTab.tsx:86 border-l-4 border-l-amber-400）
                if (task.protectionStatus == TaskProtectionStatus.AT_RISK) Modifier.atRiskLeftBorder()
                else Modifier
            )
            .then(if (enabled) Modifier.clickable(onClick = onToggle) else Modifier),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // 勾选圆圈
            Box(
                Modifier
                    .size(22.dp)
                    .clip(CircleShape)
                    .then(
                        if (task.done) Modifier.background(Quadrant.Q2.color())
                        else Modifier.border(1.5.dp, MaterialTheme.colorScheme.outline, CircleShape)
                    ),
                contentAlignment = Alignment.Center,
            ) {
                if (task.done) {
                    Icon(
                        LucideIcons.Check,
                        contentDescription = "已完成",
                        modifier = Modifier.size(14.dp),
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
            }
            Spacer(Modifier.size(12.dp))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        task.title,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (task.done) MaterialTheme.colorScheme.onSurfaceVariant
                        else MaterialTheme.colorScheme.onSurface,
                        textDecoration = if (task.done) TextDecoration.LineThrough else null,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false),
                    )
                    if (task.bigRock) {
                        Spacer(Modifier.size(6.dp))
                        BigRockBadge()
                    }
                }
                Spacer(Modifier.size(3.dp))
                Row {
                    Text(
                        task.roleName,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    task.due?.let {
                        Spacer(Modifier.size(10.dp))
                        Text(
                            it,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (task.protectionStatus == TaskProtectionStatus.AT_RISK) {
                        Spacer(Modifier.size(6.dp))
                        AtRiskBadge()
                    }
                    if (isClassifying) {
                        Spacer(Modifier.size(6.dp))
                        ClassifyingBadge()
                    }
                }
            }
        }
    }
}

/** 大石头徽章 — 母本：text-amber-600 border-amber-400 透明底小徽章。 */
@Composable
internal fun BigRockBadge(modifier: Modifier = Modifier) {
    Text(
        "大石头",
        style = MaterialTheme.typography.labelSmall,
        color = BrandWarn,
        modifier = modifier
            .clip(RoundedCornerShape(4.dp))
            .border(1.dp, BrandWarn, RoundedCornerShape(4.dp))
            .padding(horizontal = 5.dp, vertical = 1.dp),
    )
}

/** 「智能分类中…」徽章 — 母本：TaskOverviewTab.tsx:124-132（indigo 底 + Loader2 旋转 + 文案）。 */
@Composable
internal fun ClassifyingBadge(modifier: Modifier = Modifier) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier
            .clip(RoundedCornerShape(4.dp))
            .background(BrandIndigo.copy(alpha = 0.08f))
            .border(1.dp, BrandIndigo.copy(alpha = 0.25f), RoundedCornerShape(4.dp))
            .padding(horizontal = 5.dp, vertical = 1.dp),
    ) {
        SpinningLoader()
        Spacer(Modifier.size(3.dp))
        Text(
            "智能分类中…",
            style = MaterialTheme.typography.labelSmall,
            color = BrandIndigo,
        )
    }
}

/** Loader2 旋转 — 母本：animate-spin + motion-reduce:animate-none（reduced-motion 时静止显示）。 */
@Composable
private fun SpinningLoader(modifier: Modifier = Modifier) {
    if (rememberReducedMotion()) {
        Icon(
            LucideIcons.Loader2,
            contentDescription = null,
            modifier = modifier.size(11.dp),
            tint = BrandIndigo,
        )
        return
    }
    val transition = rememberInfiniteTransition(label = "loader")
    val angle by transition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(tween(1000, easing = LinearEasing)),
        label = "loaderAngle",
    )
    Icon(
        LucideIcons.Loader2,
        contentDescription = null,
        modifier = modifier
            .size(11.dp)
            .graphicsLayer { rotationZ = angle },
        tint = BrandIndigo,
    )
}

/** 「被挤压」徽章 — 母本：TaskOverviewTab.tsx:115-123（amber 底 + AlertTriangle 11px + 文案；aria=重要任务被持续挤压）。 */
@Composable
internal fun AtRiskBadge(modifier: Modifier = Modifier) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier
            .clip(RoundedCornerShape(4.dp))
            .background(BrandWarn.copy(alpha = 0.08f))
            .border(1.dp, BrandWarn.copy(alpha = 0.25f), RoundedCornerShape(4.dp))
            .padding(horizontal = 5.dp, vertical = 1.dp),
    ) {
        Icon(
            LucideIcons.AlertTriangle,
            contentDescription = "重要任务被持续挤压，建议尽快处理",
            modifier = Modifier.size(11.dp),
            tint = BrandWarn,
        )
        Spacer(Modifier.size(3.dp))
        Text(
            "被挤压",
            style = MaterialTheme.typography.labelSmall,
            color = BrandWarn,
        )
    }
}

/**
 * at_risk 卡片 4dp 琥珀左边条 — 母本 TaskOverviewTab.tsx:86 border-l-4 border-l-amber-400。
 * 在 Surface 内容之上按卡片圆角裁剪绘制，跟随左边圆弧（仅遮左侧 4dp 内边距区域，不覆盖正文）。
 */
private fun Modifier.atRiskLeftBorder(): Modifier = drawWithContent {
    drawContent()
    val radius = 10.dp.toPx()
    clipPath(
        Path().apply {
            addRoundRect(RoundRect(0f, 0f, size.width, size.height, CornerRadius(radius)))
        },
    ) {
        drawRect(
            color = BrandWarn,
            topLeft = Offset.Zero,
            size = Size(4.dp.toPx(), size.height),
        )
    }
}

// ── Preview ────────────────────────────────────────────────────────────

@Preview(showBackground = true)
@Composable
private fun TasksScreenPreview() {
    EgoSyncTheme {
        TasksScreen(
            uiState = TasksUiState.sample(),
            engineAvailable = true,
            onToggleTask = {},
            onQuadrantFilterSelected = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun TaskRowPreview() {
    EgoSyncTheme {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            TaskRow(
                task = SnapshotStore.tasks.first(),
                enabled = true,
                onToggle = {},
            )
            TaskRow(
                task = SnapshotStore.tasks.first().copy(done = true),
                enabled = true,
                onToggle = {},
            )
            TaskRow(
                task = SnapshotStore.tasks.first(),
                enabled = true,
                isClassifying = true,
                onToggle = {},
            )
            // FR-24：at_risk 任务卡（左边框 + 被挤压徽章）
            TaskRow(
                task = SnapshotStore.tasks.first { it.protectionStatus == TaskProtectionStatus.AT_RISK },
                enabled = true,
                onToggle = {},
            )
        }
    }
}
