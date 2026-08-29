package com.egosync.companion.ui.tasks

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.egosync.companion.sync.CreateTaskInput
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.ui.previewRoles
import com.egosync.companion.ui.previewTasks
import com.egosync.companion.sync.TASK_OWNER_BUTLER_KEY
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.sync.TaskOwner
import com.egosync.companion.sync.TaskProtectionStatus
import com.egosync.companion.ui.theme.BrandIndigo
import com.egosync.companion.ui.theme.BrandWarn
import com.egosync.companion.ui.theme.EgoSyncTheme
import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.accent
import com.egosync.companion.ui.theme.color
import com.egosync.companion.ui.theme.rememberReducedMotion

/**
 * ② 任务 Tab：四象限分组列表（Q1~Q4 分色、大石头星标、勾选完成交互）。
 * 筛选区镜像桌面 TaskOverviewTab：象限单选 + 只看大石头 + 归属多选排除；
 * 「新增任务」入口 + 底部弹层表单镜像桌面 TaskModal 字段集。
 */
@Composable
fun TasksScreen(
    uiState: TasksUiState,
    roles: List<RoleCard>,
    engineAvailable: Boolean,
    onToggleTask: (taskId: String) -> Unit,
    onQuadrantFilterSelected: (Quadrant?) -> Unit,
    onToggleBigRocksOnly: () -> Unit,
    onToggleOwner: (ownerKey: String) -> Unit,
    onToggleAllOwners: () -> Unit,
    onCreateTask: (CreateTaskInput) -> Unit,
    modifier: Modifier = Modifier,
) {
    val grouped = uiState.grouped()
    // 新建表单弹层可见性（表单草稿状态收在弹层内部，镜像桌面 TaskModal 自持 useState）；
    // rememberSaveable：旋转/进程重建后弹层与草稿不丢
    var showCreateSheet by rememberSaveable { mutableStateOf(false) }

    Column(modifier = modifier.fillMaxSize()) {
        // 新建任务入口（镜像桌面 TaskOverviewTab.tsx:276-282：Plus + 文字按钮）；
        // 离线时与任务行同步禁用（任务操作需桌面引擎在线）
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.End,
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 16.dp, end = 16.dp, top = 12.dp)
                .clickable(
                    enabled = engineAvailable,
                    role = Role.Button,
                    onClickLabel = "新增任务",
                ) { showCreateSheet = true },
        ) {
            Icon(
                LucideIcons.Plus,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
                tint = if (engineAvailable) MaterialTheme.colorScheme.primary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(4.dp))
            Text(
                "新增任务",
                style = MaterialTheme.typography.labelLarge,
                color = if (engineAvailable) MaterialTheme.colorScheme.primary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        // FR-23 象限筛选行（镜像桌面 TaskOverviewTab.tsx:286-301：Filter 图标 + 全部/Q1~Q4 单选 chip，
        // 同行分隔后接「只看大石头」开关 :303-313）
        QuadrantFilterRow(
            selected = uiState.quadrantFilter,
            onSelect = onQuadrantFilterSelected,
            bigRocksOnly = uiState.showBigRocksOnly,
            onToggleBigRocks = onToggleBigRocksOnly,
            modifier = Modifier.padding(horizontal = 16.dp),
        )
        // 归属筛选行（镜像桌面 :315-348：Users 图标 + 全部/管家/各角色多选排除 chip）
        OwnerFilterRow(
            roles = roles,
            deselectedOwners = uiState.deselectedOwners,
            onToggleOwner = onToggleOwner,
            onToggleAllOwners = onToggleAllOwners,
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
                    // 镜像桌面 TaskOverviewTab.tsx:356 isDefaultFilter 空态文案分支：
                    // 象限 + 大石头 + 归属全部默认时才提示「很轻松」，否则视为筛选无匹配
                    val isDefaultFilter = uiState.quadrantFilter == null &&
                        !uiState.showBigRocksOnly &&
                        uiState.deselectedOwners.isEmpty()
                    Text(
                        if (isDefaultFilter) "所有角色都很轻松，可以考虑添加新目标"
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

    if (showCreateSheet) {
        CreateTaskSheet(
            roles = roles,
            // 离线门禁：弹层在独立 Dialog window，全局降级蒙层拦不到——保存按钮与入口同步禁用，
            // 否则「入口离线禁用但表单仍可提交」自相矛盾
            enabled = engineAvailable,
            onCreate = onCreateTask,
            onDismiss = { showCreateSheet = false },
        )
    }
}

/**
 * 新建表单标题校验（镜像桌面 TaskModal.tsx:104「任务内容不能为空」）：
 * 空/纯空格返回报错文案，否则 null。空标题提交必须被拦下且表单不关闭。
 */
internal fun createTaskTitleError(title: String): String? =
    if (title.isBlank()) "任务内容不能为空" else null

/** 象限筛选行：Filter 图标 + 全部/Q1~Q4 单选 chip + 「只看大石头」开关（镜像桌面 quadrantChips + :303-313，选中 indigo 高亮）。 */
@Composable
private fun QuadrantFilterRow(
    selected: Quadrant?,
    onSelect: (Quadrant?) -> Unit,
    bigRocksOnly: Boolean,
    onToggleBigRocks: () -> Unit,
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
            // 只看大石头（桌面与象限 chip 同行、以竖线分隔；此处接在 chip 行末尾）
            item(key = "big-rocks") {
                BigRocksFilterChip(active = bigRocksOnly, onClick = onToggleBigRocks)
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

/** 「只看大石头」开关 chip — 母本 :303-313：Target 图标 + 文案，开启态 amber 高亮（bg-amber-100 text-amber-700）。 */
@Composable
private fun BigRocksFilterChip(active: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (active) BrandWarn.copy(alpha = 0.12f) else MaterialTheme.colorScheme.surface,
        border = BorderStroke(1.dp, if (active) BrandWarn else MaterialTheme.colorScheme.outline),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        ) {
            Icon(
                LucideIcons.Target,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = if (active) BrandWarn else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(4.dp))
            Text(
                "只看大石头",
                style = MaterialTheme.typography.labelMedium,
                color = if (active) BrandWarn else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/**
 * 归属筛选行 — 母本 :315-348：Users 图标 + 「全部」+ 管家/各角色 chip。
 * 多选排除语义（deselectedOwners 含 key = 被排除），点击行为见 [TasksUiState.toggleOwner]；
 * 「全部」高亮 = 无任何排除（镜像桌面 aria-pressed 判定）。
 */
@Composable
private fun OwnerFilterRow(
    roles: List<RoleCard>,
    deselectedOwners: Set<String>,
    onToggleOwner: (ownerKey: String) -> Unit,
    onToggleAllOwners: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier
            .fillMaxWidth()
            .padding(bottom = 12.dp),
    ) {
        Icon(
            LucideIcons.Users,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.size(6.dp))
        LazyRow(
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            modifier = Modifier.weight(1f),
        ) {
            item(key = "all-owners") {
                OwnerFilterChip(
                    label = "全部",
                    selected = deselectedOwners.isEmpty(),
                    dotColor = null,
                    onClick = onToggleAllOwners,
                )
            }
            // 管家在前 + 全部角色（对齐桌面 ownerOptions 顺序 :184-187）
            item(key = TASK_OWNER_BUTLER_KEY) {
                OwnerFilterChip(
                    label = "管家",
                    selected = deselectedOwners.isNotEmpty() && TASK_OWNER_BUTLER_KEY !in deselectedOwners,
                    // 桌面 ownerOptions 管家色 #6366F1 = BrandIndigo
                    dotColor = BrandIndigo,
                    onClick = { onToggleOwner(TASK_OWNER_BUTLER_KEY) },
                )
            }
            items(roles.size, key = { roles[it].id }) { index ->
                val role = roles[index]
                OwnerFilterChip(
                    label = role.name,
                    selected = deselectedOwners.isNotEmpty() && role.id !in deselectedOwners,
                    // 桌面取 role.color；移动端按域预映射 accent（README 色温系统约定）
                    dotColor = role.domain.accent().accent,
                    onClick = { onToggleOwner(role.id) },
                )
            }
        }
    }
}

/** 归属 chip：选中 = 该归属可见（排除语义的反面）；色点用归属色（母本 :342：选中显色、未选灰点）。 */
@Composable
private fun OwnerFilterChip(
    label: String,
    selected: Boolean,
    dotColor: Color?,
    onClick: () -> Unit,
) {
    // 选中态同 QuadrantFilterChip 母本：primary 底 + onPrimary 字 + primary 描边
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface,
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        ) {
            if (dotColor != null) {
                Box(
                    Modifier
                        .size(8.dp)
                        .clip(CircleShape)
                        .background(if (selected) dotColor else MaterialTheme.colorScheme.outline)
                )
                Spacer(Modifier.size(6.dp))
            }
            Text(
                label,
                style = MaterialTheme.typography.labelMedium,
                color = if (selected) MaterialTheme.colorScheme.onPrimary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** 「智能判断」象限选项 — 母本 TaskModal ✨ 智能判断（Sparkles 图标承载 ✨，新建默认选中）。 */
@Composable
private fun AutoQuadrantChip(selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface,
        border = BorderStroke(
            1.dp,
            if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
        ),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
        ) {
            Icon(
                LucideIcons.Sparkles,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = if (selected) MaterialTheme.colorScheme.onPrimary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.size(4.dp))
            Text(
                "智能判断",
                style = MaterialTheme.typography.labelMedium,
                color = if (selected) MaterialTheme.colorScheme.onPrimary
                else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/**
 * 新建任务底部弹层 — 字段集镜像桌面 TaskModal.tsx：
 * 任务归属（管家+角色）/ 任务内容（必填行内校验）/ 四象限分类（含智能判断）/ 截止时间 / 大石头。
 * 截止时间桌面为日期时间选择器，原型以自由文本承载（TaskItem.due 本为展示字符串）。
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CreateTaskSheet(
    roles: List<RoleCard>,
    enabled: Boolean,
    onCreate: (CreateTaskInput) -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    // 表单草稿（镜像桌面 TaskModal useState 初始值：归属默认管家、象限默认智能判断）；
    // rememberSaveable：旋转/进程重建后草稿不丢；正常关闭弹层即离场重置（同桌面 Modal 卸载）
    var ownerKey by rememberSaveable { mutableStateOf(TASK_OWNER_BUTLER_KEY) }
    var title by rememberSaveable { mutableStateOf("") }
    var quadrant by rememberSaveable { mutableStateOf<Quadrant?>(null) } // null = 智能判断（桌面 AUTO_QUADRANT）
    var due by rememberSaveable { mutableStateOf("") }
    var bigRock by rememberSaveable { mutableStateOf(false) }
    var titleError by remember { mutableStateOf(false) }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // 头行：标题 + X 关闭（镜像桌面 TaskModal 头行）
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "新建任务",
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

            // 任务归属（镜像桌面 :159-174：管家 + 全部角色；桌面为下拉，移动端单选 chip 承载）
            FieldLabel("任务归属")
            LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                item(key = "butler") {
                    OwnerFilterChip(
                        label = "管家",
                        selected = ownerKey == TASK_OWNER_BUTLER_KEY,
                        dotColor = BrandIndigo,
                        onClick = { ownerKey = TASK_OWNER_BUTLER_KEY },
                    )
                }
                items(roles.size, key = { roles[it].id }) { index ->
                    val role = roles[index]
                    OwnerFilterChip(
                        label = role.name,
                        selected = ownerKey == role.id,
                        dotColor = role.domain.accent().accent,
                        onClick = { ownerKey = role.id },
                    )
                }
            }

            // 任务内容（镜像桌面 :175-185；空标题校验见提交按钮）
            FieldLabel("任务内容")
            OutlinedTextField(
                value = title,
                onValueChange = {
                    title = it
                    if (titleError) titleError = false
                },
                placeholder = { Text("例如：准备Q3 OKR规划") },
                singleLine = true,
                isError = titleError,
                supportingText = if (titleError) {
                    // 镜像桌面 handleSubmit 校验文案（spec I/O 矩阵：空标题 → 不关表单、行内报错）
                    { Text("任务内容不能为空") }
                } else null,
                modifier = Modifier.fillMaxWidth(),
            )

            // 四象限分类（镜像桌面 :187-209：默认「智能判断」，选项文案 "Q1: 重要且紧急" 逐字对齐）
            FieldLabel("四象限分类")
            LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                item(key = "auto") {
                    AutoQuadrantChip(
                        selected = quadrant == null,
                        onClick = { quadrant = null },
                    )
                }
                items(Quadrant.entries.size, key = { Quadrant.entries[it].code }) { index ->
                    val q = Quadrant.entries[index]
                    QuadrantFilterChip(
                        label = "${q.code}: ${q.title}",
                        selected = quadrant == q,
                        onClick = { quadrant = q },
                    )
                }
            }

            // 截止时间（可选）
            FieldLabel("截止时间")
            OutlinedTextField(
                value = due,
                onValueChange = { due = it },
                placeholder = { Text("例如：周五 17:00") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            // 大石头（镜像桌面 :281-294：amber 边框卡 + Checkbox + 说明文案 + Target）
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = BrandWarn.copy(alpha = 0.06f),
                border = BorderStroke(1.dp, BrandWarn.copy(alpha = 0.4f)),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                ) {
                    Checkbox(checked = bigRock, onCheckedChange = { bigRock = it })
                    Column(Modifier.weight(1f)) {
                        Text(
                            "标记为本周大石头",
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            "在任务清单中作为本周重点展示",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Icon(
                        LucideIcons.Target,
                        contentDescription = null,
                        modifier = Modifier.size(20.dp),
                        tint = BrandWarn.copy(alpha = 0.5f),
                    )
                }
            }

            // 底部按钮（镜像桌面 :298-301：取消 / 保存任务）。
            // 桌面按钮在空标题时禁用 + handleSubmit 双保险；移动端保留提交路径以呈现行内报错（spec I/O 矩阵）
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextButton(onClick = onDismiss) { Text("取消") }
                Spacer(Modifier.size(8.dp))
                Button(
                    enabled = enabled,
                    onClick = {
                        if (createTaskTitleError(title) != null) {
                            titleError = true
                        } else {
                            onCreate(
                                CreateTaskInput(
                                    title = title.trim(),
                                    ownerType = if (ownerKey == TASK_OWNER_BUTLER_KEY) TaskOwner.BUTLER else TaskOwner.ROLE,
                                    roleId = if (ownerKey == TASK_OWNER_BUTLER_KEY) null else ownerKey,
                                    // null = 智能判断 → VM 走 classifying 过渡态
                                    quadrant = quadrant,
                                    due = due.trim().ifEmpty { null },
                                    bigRock = bigRock,
                                )
                            )
                            onDismiss()
                        }
                    },
                ) { Text("保存任务") }
            }
            Spacer(Modifier.size(8.dp))
        }
    }
}

/** 表单字段标签（镜像桌面 TaskModal label 行）。 */
@Composable
private fun FieldLabel(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
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
            roles = previewRoles,
            engineAvailable = true,
            onToggleTask = {},
            onQuadrantFilterSelected = {},
            onToggleBigRocksOnly = {},
            onToggleOwner = {},
            onToggleAllOwners = {},
            onCreateTask = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun TaskRowPreview() {
    EgoSyncTheme {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            TaskRow(
                task = previewTasks.first(),
                enabled = true,
                onToggle = {},
            )
            TaskRow(
                task = previewTasks.first().copy(done = true),
                enabled = true,
                onToggle = {},
            )
            TaskRow(
                task = previewTasks.first(),
                enabled = true,
                isClassifying = true,
                onToggle = {},
            )
            // FR-24：at_risk 任务卡（左边框 + 被挤压徽章）
            TaskRow(
                task = previewTasks.first { it.protectionStatus == TaskProtectionStatus.AT_RISK },
                enabled = true,
                onToggle = {},
            )
        }
    }
}
