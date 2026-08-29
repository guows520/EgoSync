package com.egosync.companion.ui.dashboard

import androidx.compose.ui.graphics.vector.ImageVector

import androidx.compose.material3.Icon
import com.egosync.companion.ui.icons.LucideIcons
import com.egosync.companion.ui.icons.RoleIcons

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.EaseInOut
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.ActivityWindow
import com.egosync.companion.sync.MetricScope
import com.egosync.companion.sync.MetricType
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.ui.previewRoles
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.BreathDurationMillis
import com.egosync.companion.ui.theme.ColorTransitionMillis
import com.egosync.companion.ui.theme.InfoDensity
import com.egosync.companion.ui.theme.BrandBlue
import com.egosync.companion.ui.theme.BrandIndigo
import com.egosync.companion.ui.theme.BrandWarn
import com.egosync.companion.ui.theme.accent
import com.egosync.companion.ui.theme.densitySpec
import com.egosync.companion.ui.theme.energyColor
import com.egosync.companion.ui.theme.rememberReducedMotion
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.util.Locale

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
    onMetricsScopeSelected: (String) -> Unit,
    onActivityWindowSelected: (ActivityWindow) -> Unit,
    onOpenMemory: (roleId: String) -> Unit,
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

        // FR-38 活动统计：scope+时间筛选 + 指标网格（镜像桌面 DashboardTab「活动统计」节）
        ActivityStatsSection(
            scopeId = uiState.metricsScopeId,
            window = uiState.activityWindow,
            roles = uiState.roles,
            metrics = uiState.activityMetrics,
            onScopeSelected = onMetricsScopeSelected,
            onWindowSelected = onActivityWindowSelected,
        )

        Spacer(Modifier.height(14.dp))

        // 角色卡横向滑动（页内垂直滚动：矮屏下卡片固有高度超出页高时可滚至"查看记忆"入口，
        // 避免统计标签底部被裁——横向翻页与纵向滚动正交，互不冲突）
        HorizontalPager(
            state = pagerState,
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 4.dp),
        ) { page ->
            val role = uiState.roles[page]
            Box(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                RoleCardItem(role = role, onOpenMemory = { onOpenMemory(role.id) })
            }
        }

        Spacer(Modifier.height(10.dp))

        // 页指示点（活动点取当前页角色域 accent：色温随角色切换渐变）
        // reduced-motion 时直切不渐变（动效白名单降级）
        val reducedMotion = rememberReducedMotion()
        val colorTransitionSpec = if (reducedMotion) snap<Color>() else tween(ColorTransitionMillis)
        val currentAccent = uiState.roles.getOrNull(pagerState.currentPage)?.domain?.accent()?.accent
            ?: MaterialTheme.colorScheme.primary
        val activeDotColor by animateColorAsState(
            targetValue = currentAccent,
            animationSpec = colorTransitionSpec,
            label = "dotAccent",
        )
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
                            if (active) activeDotColor
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
        shape = RoundedCornerShape(10.dp),
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
                ) { Icon(LucideIcons.Home, contentDescription = "管家", modifier = Modifier.size(24.dp)) }
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
                OverviewEntry(LucideIcons.FileText, "晨间简报", onOpenBriefing)
                OverviewEntry(LucideIcons.TrendingUp, "周复盘", onOpenReview)
                OverviewEntry(LucideIcons.Bell, "通知", onOpenNotifications)
            }
        }
    }
}

@Composable
private fun RowScope.OverviewEntry(icon: ImageVector, text: String, onClick: () -> Unit) {
    val d = densitySpec(InfoDensity.CONSOLE)
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier
            .weight(1f)
            .clickable(onClick = onClick),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = d.unitPaddingY),
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(icon, contentDescription = null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.onSurface)
            Spacer(Modifier.size(4.dp))
            Text(text, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurface)
        }
    }
}

// ── 角色卡 ─────────────────────────────────────────────────────────────

@Composable
internal fun RoleCardItem(role: RoleCard, onOpenMemory: () -> Unit = {}, modifier: Modifier = Modifier) {
    val roleAccent = role.domain.accent()
    // 母本 App.tsx --role-accent 切换 + index.css --duration-color:300ms 过渡：
    // 域 accent 变化时以 300ms 单次过渡渐变（色温随角色域偏移，功能性 transition）
    // reduced-motion 时直切不渐变（动效白名单降级）
    val colorTransitionSpec = if (rememberReducedMotion()) snap<Color>() else tween(ColorTransitionMillis)
    val accentColor by animateColorAsState(
        targetValue = roleAccent.accent,
        animationSpec = colorTransitionSpec,
        label = "roleAccent",
    )
    val tintColor by animateColorAsState(
        targetValue = roleAccent.tint,
        animationSpec = colorTransitionSpec,
        label = "roleTint",
    )

    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(10.dp),
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 4.dp),
    ) {
        // 卡内边距与间距压缩（18→12、18/20/14→12/12/10，合计省约 30dp）：
        // 使卡片固有高度适配矮屏（360×800dp）pager 页高，统计标签不被页底裁切；
        // 更矮屏幕由外层 verticalScroll 兜底
        Column(Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    Modifier
                        .size(46.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(accentColor),
                    contentAlignment = Alignment.Center,
                ) { Icon(RoleIcons.getRoleIcon(role.icon), contentDescription = role.name, modifier = Modifier.size(24.dp), tint = Color.White) }

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
                        .background(tintColor)
                        .padding(horizontal = 10.dp, vertical = 4.dp)
                ) {
                    Text(
                        "待办 ${role.pendingCount}",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }

            Spacer(Modifier.height(12.dp))

            // 能量条（呼吸动效：活的实体）
            BreathingEnergyBar(energy = role.energy, accent = energyColor(role.energy))

            Spacer(Modifier.height(12.dp))

            // 统计数字行（信息密集 · 控制台模式）
            Row(Modifier.fillMaxWidth()) {
                StatCell("任务", role.taskCount.toString(), Modifier.weight(1f))
                // 快照无 per-role 记忆数（NFR-M3 诚实降级）：显示「—」，禁止凑数
                StatCell("记忆", role.memoryCount?.toString() ?: "—", Modifier.weight(1f))
                StatCell("会话", role.sessionCount.toString(), Modifier.weight(1f))
                StatCell("待办", role.pendingCount.toString(), Modifier.weight(1f))
            }

            Spacer(Modifier.height(10.dp))

            // FR-8/9 记忆入口：二级记忆屏 push 入口（Brain + ChevronRight）
            Surface(
                onClick = onOpenMemory,
                shape = RoundedCornerShape(10.dp),
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        LucideIcons.Brain,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = roleAccent.accent,
                    )
                    Spacer(Modifier.size(8.dp))
                    Text(
                        "查看记忆",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(Modifier.weight(1f))
                    Icon(
                        LucideIcons.ChevronRight,
                        contentDescription = null,
                        modifier = Modifier.size(14.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun StatCell(label: String, value: String, modifier: Modifier = Modifier) {
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
 * 呼吸能量条 — 母本 index.css .breathe：opacity 0.6↔1.0、--duration-breath 3s
 * ease-in-out 无限循环（半程 BreathDurationMillis/2 × Reverse = 全周期 3s）。
 * 全 App 唯一的装饰性动效（"角色卡片有呼吸感——活的实体"）。
 * reduced-motion（系统「移除动画」开启）：定格母本关键帧端点 opacity 0.6，静态显示。
 */
@Composable
internal fun BreathingEnergyBar(energy: Int, accent: Color, modifier: Modifier = Modifier) {
    val reducedMotion = rememberReducedMotion()
    val breath: Float = if (reducedMotion) {
        0.6f
    } else {
        val transition = rememberInfiniteTransition(label = "breath")
        val animated by transition.animateFloat(
            initialValue = 0.6f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(BreathDurationMillis / 2, easing = EaseInOut), RepeatMode.Reverse),
            label = "breathAlpha",
        )
        animated
    }

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
                .height(6.dp)
                .clip(RoundedCornerShape(3.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant)
        ) {
            Box(
                Modifier
                    .fillMaxWidth(energy / 100f)
                    .height(6.dp)
                    .clip(RoundedCornerShape(3.dp))
                    .background(accent.copy(alpha = breath))
            )
        }
    }
}

// ── FR-38 活动统计（scope+时间筛选 + 指标网格）─────────────────────────

/** 指标卡配色镜像桌面 DashboardTab metricCards：text-indigo/purple/amber/blue-500。 */
private val MetricPurple = Color(0xFFA855F7)

/**
 * 预设时间窗（标签与顺序镜像桌面 DATE_PRESETS）。
 * 天数上界为分桶模型的近似：桌面「最近1个月」是日历月回退（28–31 天浮动），
 * 移动统一取近 30 天（Recent(29)），与天数桶边界 7–29 对齐——桶粒度降级的一部分。
 */
private val TIME_PRESETS: List<Pair<String, ActivityWindow.Recent>> = listOf(
    "最近3天" to ActivityWindow.Recent(2),
    "最近7天" to ActivityWindow.Recent(6),
    "最近1个月" to ActivityWindow.Recent(29),
)

@Composable
private fun ActivityStatsSection(
    scopeId: String,
    window: ActivityWindow,
    roles: List<RoleCard>,
    metrics: Map<MetricType, Int?>,
    onScopeSelected: (String) -> Unit,
    onWindowSelected: (ActivityWindow) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth()) {
        Text(
            "活动统计",
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(start = 4.dp),
        )
        Spacer(Modifier.height(8.dp))

        // 筛选行：角色 scope 下拉 + 时间范围下拉（镜像桌面 select + DateRangeFilter）
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ScopeFilterChip(scopeId, roles, onScopeSelected, Modifier.weight(1f))
            TimeFilterChip(window, onWindowSelected, Modifier.weight(1f))
        }

        Spacer(Modifier.height(10.dp))

        // 指标网格：2×2（镜像桌面 grid-cols-2，图标/标签/配色逐项对应）
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MetricCard(MetricType.TASK_TOTAL, metrics, Modifier.weight(1f))
                MetricCard(MetricType.MEMORY_COUNT, metrics, Modifier.weight(1f))
            }
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MetricCard(MetricType.PENDING_TASKS, metrics, Modifier.weight(1f))
                MetricCard(MetricType.CONVERSATION_COUNT, metrics, Modifier.weight(1f))
            }
        }
    }
}

/** scope 下拉触发钮与菜单：选项顺序镜像桌面（全部/管家/各角色），无图标纯文本。 */
@Composable
private fun ScopeFilterChip(
    scopeId: String,
    roles: List<RoleCard>,
    onSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    Box(modifier) {
        FilterChipSurface(label = scopeLabel(scopeId, roles), leadingIcon = null) { expanded = true }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            DropdownMenuItem(
                text = { Text("全部") },
                onClick = { onSelected(MetricScope.ALL); expanded = false },
            )
            DropdownMenuItem(
                text = { Text("管家") },
                onClick = { onSelected(MetricScope.BUTLER); expanded = false },
            )
            roles.forEach { role ->
                DropdownMenuItem(
                    text = { Text(role.name) },
                    onClick = { onSelected(role.id); expanded = false },
                )
            }
        }
    }
}

/** 时间下拉：CalendarDays+标签+ChevronDown 触发钮；自定义时间走两次 DatePickerDialog。 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TimeFilterChip(
    window: ActivityWindow,
    onSelected: (ActivityWindow) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    var showStartPicker by remember { mutableStateOf(false) }
    var showEndPicker by remember { mutableStateOf(false) }
    var draftStart by remember { mutableStateOf<LocalDate?>(null) }

    Box(modifier) {
        FilterChipSurface(label = windowLabel(window), leadingIcon = LucideIcons.CalendarDays) {
            expanded = true
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            DropdownMenuItem(
                text = { Text("全部日期") },
                onClick = { onSelected(ActivityWindow.All); expanded = false },
            )
            TIME_PRESETS.forEach { (label, preset) ->
                DropdownMenuItem(
                    text = { Text(label) },
                    onClick = { onSelected(preset); expanded = false },
                )
            }
            // 自定义时间：先选开始再选结束（移动适配桌面双 date input 编辑器）
            DropdownMenuItem(
                text = { Text("自定义时间") },
                onClick = { draftStart = null; showStartPicker = true; expanded = false },
            )
        }
    }

    if (showStartPicker) {
        val state = rememberDatePickerState()
        DatePickerDialog(
            onDismissRequest = { showStartPicker = false },
            confirmButton = {
                // 未选日期禁用推进（镜像桌面：日期未填满则应用钮禁用），避免流程静默中断
                TextButton(
                    enabled = state.selectedDateMillis != null,
                    onClick = {
                        state.selectedDateMillis?.let { draftStart = epochToLocalDate(it) }
                        showStartPicker = false
                        showEndPicker = true
                    },
                ) { Text("下一步") }
            },
            dismissButton = {
                TextButton(onClick = { showStartPicker = false }) { Text("取消") }
            },
        ) {
            Text(
                "开始日期",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 16.dp),
            )
            DatePicker(state = state)
        }
    }

    if (showEndPicker && draftStart != null) {
        val start = draftStart ?: return
        val state = rememberDatePickerState()
        val draftEnd = state.selectedDateMillis?.let(::epochToLocalDate)
        val rangeError =
            if (draftEnd != null && draftEnd < start) "结束日期不能早于开始日期" else ""
        DatePickerDialog(
            onDismissRequest = { showEndPicker = false; draftStart = null },
            confirmButton = {
                TextButton(
                    enabled = draftEnd != null && rangeError.isEmpty(),
                    onClick = {
                        onSelected(
                            ActivityWindow.Custom(
                                oldestDate = start,
                                newestDate = draftEnd!!,
                            )
                        )
                        showEndPicker = false
                        draftStart = null
                    },
                ) { Text("应用日期") }
            },
            dismissButton = {
                TextButton(onClick = { showEndPicker = false; draftStart = null }) { Text("取消") }
            },
        ) {
            Text(
                "结束日期",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 16.dp),
            )
            DatePicker(state = state)
            // 校验失败时保留编辑态、行内报错并禁用「应用日期」（镜像桌面 customRangeError：编辑器不关闭）
            if (rangeError.isNotEmpty()) {
                Text(
                    rangeError,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
            }
        }
    }
}

/** 下拉触发钮外观：描边 Surface + 前置图标 + 标签 + ChevronDown（镜像桌面 DateRangeFilter trigger）。 */
@Composable
private fun FilterChipSurface(
    label: String,
    leadingIcon: ImageVector?,
    onClick: () -> Unit,
) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(10.dp),
        color = MaterialTheme.colorScheme.surface,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 9.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (leadingIcon != null) {
                Icon(
                    leadingIcon,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.size(6.dp))
            }
            Text(
                label,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            Icon(
                LucideIcons.ChevronDown,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/**
 * 指标卡：图标+标签一行、数值右下大字（镜像桌面 metricCards 卡片结构）。
 * mock 同步聚合无加载态——桌面 AC-13「加载保留上次数据」分支不适用。
 */
@Composable
private fun MetricCard(
    type: MetricType,
    metrics: Map<MetricType, Int?>,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        modifier = modifier,
    ) {
        Column(Modifier.padding(horizontal = 12.dp, vertical = 10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    metricIcon(type),
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = type.cardColor(),
                )
                Spacer(Modifier.size(6.dp))
                Text(
                    type.label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Spacer(Modifier.height(6.dp))
            // 快照不可得指标（如 per-role 记忆数）显示「—」，不冒充 0
            Text(
                metrics[type]?.toString() ?: "—",
                style = MaterialTheme.typography.titleLarge,
                textAlign = TextAlign.End,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

private fun metricIcon(type: MetricType): ImageVector = when (type) {
    MetricType.TASK_TOTAL -> LucideIcons.ListTodo
    MetricType.MEMORY_COUNT -> LucideIcons.Brain
    MetricType.PENDING_TASKS -> LucideIcons.Clock
    MetricType.CONVERSATION_COUNT -> LucideIcons.MessageSquare
}

private fun MetricType.cardColor(): Color = when (this) {
    MetricType.TASK_TOTAL -> BrandIndigo
    MetricType.MEMORY_COUNT -> MetricPurple
    MetricType.PENDING_TASKS -> BrandWarn
    MetricType.CONVERSATION_COUNT -> BrandBlue
}

private fun scopeLabel(scopeId: String, roles: List<RoleCard>): String = when (scopeId) {
    MetricScope.ALL -> "全部"
    MetricScope.BUTLER -> "管家"
    else -> roles.firstOrNull { it.id == scopeId }?.name ?: "全部"
}

private fun windowLabel(window: ActivityWindow): String = when (window) {
    ActivityWindow.All -> "全部日期"
    is ActivityWindow.Recent ->
        // 标签单一来源：从 TIME_PRESETS 反查，避免魔数两处维护
        TIME_PRESETS.firstOrNull { it.second == window }?.first
            ?: "最近${window.maxDaysAgo + 1}天"
    is ActivityWindow.Custom -> {
        // T8：窗口携带选择时的绝对日期——相对天数跨午夜后前移会让标签与实际窗口错位
        val start = window.oldestDate
        val end = window.newestDate
        if (start.year != end.year) {
            "%04d-%02d-%02d 至 %04d-%02d-%02d".format(
                Locale.ROOT,
                start.year, start.monthValue, start.dayOfMonth,
                end.year, end.monthValue, end.dayOfMonth,
            )
        } else {
            "%02d-%02d 至 %02d-%02d".format(
                Locale.ROOT,
                start.monthValue, start.dayOfMonth, end.monthValue, end.dayOfMonth,
            )
        }
    }
}

/** DatePicker 的 UTC epoch millis → LocalDate（M3 DatePicker 以 UTC 零点存日期）。 */
private fun epochToLocalDate(millis: Long): LocalDate =
    Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate()

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
            onMetricsScopeSelected = {},
            onActivityWindowSelected = {},
            onOpenMemory = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun RoleCardPreview() {
    EgoSyncTheme {
        RoleCardItem(role = previewRoles.first(), modifier = Modifier.padding(12.dp))
    }
}
