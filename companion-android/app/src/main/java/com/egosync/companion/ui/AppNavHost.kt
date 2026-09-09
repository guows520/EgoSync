package com.egosync.companion.ui

import androidx.compose.material3.Icon
import com.egosync.companion.ui.icons.LucideIcons
import androidx.compose.ui.graphics.vector.ImageVector

import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.navArgument
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.egosync.companion.AppModelContainer
import com.egosync.companion.connection.TransportStatus
import com.egosync.companion.ui.briefing.BriefingScreen
import com.egosync.companion.ui.chat.ChatScreen
import com.egosync.companion.ui.chat.ChatViewModel
import com.egosync.companion.ui.dashboard.DashboardScreen
import com.egosync.companion.ui.dashboard.DashboardViewModel
import com.egosync.companion.ui.memory.MemoryScreen
import com.egosync.companion.ui.memory.MemoryViewModel
import com.egosync.companion.ui.notify.NotificationCenterScreen
import com.egosync.companion.ui.onboarding.OnboardingScreen
import com.egosync.companion.ui.onboarding.OnboardingViewModel
import com.egosync.companion.ui.review.WeeklyReviewScreen
import com.egosync.companion.ui.review.WeeklyReviewViewModel
import com.egosync.companion.ui.settings.SettingsScreen
import com.egosync.companion.ui.settings.SettingsViewModel
import com.egosync.companion.ui.tasks.TasksScreen
import com.egosync.companion.ui.tasks.TasksViewModel
import com.egosync.companion.ui.theme.BrandAmber
import com.egosync.companion.ui.theme.BrandError
import com.egosync.companion.ui.theme.rememberReducedMotion

private data class TabSpec(
    val route: String,
    val label: String,
    val icon: ImageVector,
)

private val tabs = listOf(
    TabSpec("chat", "对话", LucideIcons.MessageSquare),
    TabSpec("tasks", "任务", LucideIcons.ListTodo),
    TabSpec("dashboard", "仪表盘", LucideIcons.LayoutDashboard),
    TabSpec("settings", "我的", LucideIcons.User),
)

/** 路由表：pairing 独立根；onboarding 空状态引导（FR-21）；home 壳内四 Tab + 二级页 push。 */
object Routes {
    const val PAIRING = "pairing"
    const val ONBOARDING = "onboarding"
    const val CHAT = "chat"
    const val TASKS = "tasks"
    const val DASHBOARD = "dashboard"
    const val SETTINGS = "settings"
    const val BRIEFING = "briefing"
    const val REVIEW = "review"
    const val NOTIFICATIONS = "notifications"
    const val MEMORY = "memory"
}

@Composable
fun AppNavHost(container: AppModelContainer) {
    val navController = rememberNavController()
    // 首次组合时定格起始页（配对/解除配对由显式导航处理，避免图重建重置返回栈）
    // FR-21：未配对→配对流；已配对未引导→空状态引导；否则主界面
    val startDestination = remember {
        when {
            !container.connection.paired.value -> Routes.PAIRING
            !container.isOnboarded() -> Routes.ONBOARDING
            else -> Routes.DASHBOARD
        }
    }
    val transportStatus by container.connection.state.collectAsState()
    // reduced-motion（系统「移除动画」开启）：页面转场瞬时，不做淡入淡出
    val reducedMotion = rememberReducedMotion()

    // T-S4：配对/凭据失效恢复事件 → 回配对流重扫（容器已并行清快照/通知并
    // Snackbar 原因）。冷启动检测不发事件（健康态经 PairingRoute 提示），此处
    // 只处理运行中失效——任何非配对页都会被带回。已在配对页则不重复压栈。
    androidx.compose.runtime.LaunchedEffect(container) {
        container.connection.pairingRecovery.collect {
            if (navController.currentDestination?.route != Routes.PAIRING) {
                navController.navigate(Routes.PAIRING) {
                    popUpTo(0) { inclusive = true }
                }
            }
        }
    }

    androidx.compose.foundation.layout.Box(Modifier.fillMaxSize()) {
        NavHost(
            navController = navController,
            startDestination = startDestination,
            enterTransition = { if (reducedMotion) EnterTransition.None else fadeIn(tween(220)) },
            exitTransition = { if (reducedMotion) ExitTransition.None else fadeOut(tween(180)) },
            popEnterTransition = { if (reducedMotion) EnterTransition.None else fadeIn(tween(220)) },
            popExitTransition = { if (reducedMotion) ExitTransition.None else fadeOut(tween(180)) },
        ) {
        composable(Routes.PAIRING) {
            PairingRoute(navController, container)
        }
        composable(Routes.ONBOARDING) { OnboardingRoute(navController, container) }
        composable(Routes.CHAT) { MainShellRoute(navController, container, Routes.CHAT) }
        composable(Routes.TASKS) { MainShellRoute(navController, container, Routes.TASKS) }
        composable(Routes.DASHBOARD) { MainShellRoute(navController, container, Routes.DASHBOARD) }
        composable(Routes.SETTINGS) { MainShellRoute(navController, container, Routes.SETTINGS) }
        composable(Routes.BRIEFING) { BriefingRoute(navController, container) }
        composable(Routes.REVIEW) { WeeklyReviewRoute(navController, container) }
        composable(Routes.NOTIFICATIONS) { NotificationCenterRoute(navController, container) }
        // FR-8/9 记忆屏：按角色进入（仪表盘角色卡「查看记忆」入口）
        composable(
            route = "${Routes.MEMORY}/{roleId}",
            arguments = listOf(navArgument("roleId") { type = NavType.StringType }),
        ) { entry ->
            val roleId = entry.arguments?.getString("roleId").orEmpty()
            if (roleId.isNotEmpty()) {
                MemoryRoute(navController, container, roleId)
            }
        }
        }

        // T-S3 紧凑非阻断指示（SPEC supersessions.md：FR-43 全屏遮罩阻断形态退役）：
        // Connecting/Reconnecting 轻量状态条；Degraded 降级横幅（数据截止 + 写操作
        // 禁用说明）。paired 守门：未配对（配对流内）的承载失败由配对屏自有错误态呈现。
        val paired by container.connection.paired.collectAsState()
        if (paired) {
            ConnectionStatusBanner(
                status = transportStatus,
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .fillMaxWidth(),
            )
        }
    }
}

/**
 * 紧凑非阻断连接指示（T-S3）：仅 Connecting/Reconnecting/Degraded 显示——
 * 小状态条贴顶，不拦截任何交互（写操作禁用由各屏 commandReady 判据承担）。
 */
@Composable
private fun ConnectionStatusBanner(status: TransportStatus, modifier: Modifier = Modifier) {
    if (status is TransportStatus.Direct || status is TransportStatus.Relay) return
    val text = when (status) {
        TransportStatus.Connecting -> "正在连接桌面引擎…"
        TransportStatus.Reconnecting -> "连接已断开，正在重连…"
        is TransportStatus.Degraded ->
            if (status.snapshotAvailable) {
                "离线降级 · 缓存数据截至${status.dataAsOf ?: "未知"}，写操作暂不可用，重连后自动恢复"
            } else {
                "离线降级 · 暂无缓存数据，写操作暂不可用，重连后自动恢复"
            }
        else -> return
    }
    val color = if (status is TransportStatus.Degraded) BrandError else BrandAmber
    Surface(
        modifier = modifier,
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.96f),
        shadowElevation = 4.dp,
    ) {
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 7.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                Modifier
                    .size(8.dp)
                    .clip(CircleShape)
                    .background(color),
            )
            Spacer(Modifier.size(8.dp))
            Text(
                text,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ── 配对流路由 ─────────────────────────────────────────────────────────

@Composable
private fun PairingRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: com.egosync.companion.pairing.PairingViewModel = viewModel(
        factory = viewModelFactory {
            initializer {
                com.egosync.companion.pairing.PairingViewModel(
                    container.connection,
                    container.pairingConnector,
                )
            }
        }
    )
    val step by vm.step.collectAsState()
    val connectStage by vm.connectStage.collectAsState()
    val waitDesktopConfirm by vm.waitDesktopConfirm.collectAsState()
    val pairingError by vm.pairingError.collectAsState()
    val qrHasRelay by vm.qrHasRelay.collectAsState()
    // T-S4：健康面失效原因 → 配对屏顶部提示（冷启动 CredentialMissing 与
    // 运行中恢复后落到配对页共用此呈现）
    val pairingHealth by container.connection.pairingHealth.collectAsState()
    val recoveryHint = when (pairingHealth) {
        com.egosync.companion.connection.PairingHealth.Ok -> null
        com.egosync.companion.connection.PairingHealth.CredentialMissing ->
            "检测到本机配对密钥缺失，请重新扫码"
        com.egosync.companion.connection.PairingHealth.CredentialInvalid ->
            "本机配对密钥已失效，请重新扫码"
        com.egosync.companion.connection.PairingHealth.PairingRevoked ->
            "桌面已解除与此手机的配对，请重新扫码"
        com.egosync.companion.connection.PairingHealth.TrustMismatch ->
            "桌面身份已变化（可能重装），需重新配对"
    }

    com.egosync.companion.pairing.PairingScreen(
        step = step,
        connectStage = connectStage,
        onStartScan = vm::startScan,
        onScanCompleted = vm::onQrScanned,
        onBack = vm::back,
        onEnterApp = {
            container.completePairing()
            // FR-21：配对成功后未完成引导 → 进空状态引导屏（而非直接落仪表盘）
            val target = if (container.isOnboarded()) Routes.DASHBOARD else Routes.ONBOARDING
            navController.navigate(target) {
                popUpTo(Routes.PAIRING) { inclusive = true }
                launchSingleTop = true
            }
        },
        waitDesktopConfirm = waitDesktopConfirm,
        pairingError = pairingError,
        qrHasRelay = qrHasRelay,
        recoveryHint = recoveryHint,
    )
}

// ── FR-21 空状态引导流路由 ─────────────────────────────────────────────

@Composable
private fun OnboardingRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: OnboardingViewModel = viewModel(
        factory = viewModelFactory { initializer { OnboardingViewModel(container) } }
    )
    val uiState by vm.uiState.collectAsState()
    // T-S2：写操作判据改绑 commandReady（出站通道绑定），不再读连接展示态
    val commandReady by container.connection.commandReady.collectAsState()
    // 完成路径统一出口：清引导栈进主界面（镜像桌面 onComplete → butler 视图）
    val enterMain = {
        navController.navigate(Routes.DASHBOARD) {
            popUpTo(Routes.ONBOARDING) { inclusive = true }
            launchSingleTop = true
        }
    }
    OnboardingScreen(
        uiState = uiState,
        commandReady = commandReady,
        onSendMessage = vm::sendMessage,
        onSkipOnboarding = { vm.skipOnboarding(onComplete = enterMain) },
        onRoleProposalConfirm = { name, icon, color, goal ->
            vm.confirmRoleProposal(name, icon, color, goal, onCompleted = enterMain)
        },
        onRoleProposalSkip = vm::skipRoleProposal,
    )
}

// ── 主界面壳：内容 + 底部四 Tab ─────────────────────────────────────────

@Composable
private fun MainShellRoute(
    navController: NavHostController,
    container: AppModelContainer,
    currentTab: String,
) {
    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        // 修复顶部双重 inset：外层 MainActivity Scaffold 的默认 contentWindowInsets
        // 已把系统栏留白计入 content padding；内层 Scaffold 若再叠加默认值，
        // 四个 Tab 顶部会被推下两次（约 2× 状态栏高度），故此处显式置零。
        contentWindowInsets = WindowInsets(0, 0, 0, 0),
        bottomBar = {
            val backStackEntry by navController.currentBackStackEntryAsState()
            val currentRoute = backStackEntry?.destination?.route
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                // T-S8-A：bottomBar bounds 诊断接线（仅 debug）
                modifier = if (com.egosync.companion.BuildConfig.DEBUG) {
                    Modifier.onGloballyPositioned { ImeDiagnostics.navigationBarBounds = it.boundsInRoot() }
                } else {
                    Modifier
                },
            ) {
                tabs.forEach { tab ->
                    val selected = currentRoute == tab.route
                    NavigationBarItem(
                        selected = selected,
                        onClick = {
                            navController.navigate(tab.route) {
                                popUpTo(Routes.DASHBOARD) { saveState = true }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = {
                            Icon(tab.icon, contentDescription = tab.label)
                        },
                        label = { Text(tab.label) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = MaterialTheme.colorScheme.primary,
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                        ),
                    )
                }
            }
        },
    ) { padding ->
        if (com.egosync.companion.BuildConfig.DEBUG) {
            ImeDiagnostics.innerBottomPadding = padding.calculateBottomPadding()
        }
        Box(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .then(
                    if (com.egosync.companion.BuildConfig.DEBUG) {
                        // T-S8-A：内层 content Box bounds（bottomBar 高度消费后的可用区）
                        Modifier.onGloballyPositioned { ImeDiagnostics.innerContentBounds = it.boundsInRoot() }
                    } else {
                        Modifier
                    },
                )
        ) {
            when (currentTab) {
                Routes.CHAT -> ChatRoute(container)
                Routes.TASKS -> TasksRoute(container)
                Routes.DASHBOARD -> DashboardRoute(navController, container)
                Routes.SETTINGS -> SettingsRoute(navController, container)
            }
        }
    }
}

@Composable
private fun ChatRoute(container: AppModelContainer) {
    val vm: ChatViewModel = viewModel(
        factory = viewModelFactory {
            initializer {
                ChatViewModel(
                    store = container.snapshotStore,
                    commands = container.commandSender,
                    stream = container.streamCoordinator,
                    onError = container::showEvent,
                    outbox = container.chatOutbox,
                    commandReady = container.connection.commandReady,
                    paired = container.connection.paired,
                )
            }
        }
    )
    val uiState by vm.uiState.collectAsState()
    // T-S2/T-S10：写操作判据 commandReady（对话发送已改为常开 + 离线待发箱）
    val commandReady by container.connection.commandReady.collectAsState()
    val chatSnapshotState = container.snapshotStore.state.collectAsState()
    ChatScreen(
        uiState = uiState,
        commandReady = commandReady,
        dataCutoffLabel = truncatedCutoffLabel(
            chatSnapshotState.value.metadata,
            domain = "conversations",
        ),
        onSendMessage = vm::sendMessage,
        onActionCardRespond = vm::respondActionCard,
        onRoleSelected = vm::selectRole,
        onStopStreaming = vm::stopStreaming,
        onDecompositionRespond = vm::respondDecomposition,
        onRoleProposalConfirm = vm::confirmRoleProposal,
        onRoleProposalSkip = vm::skipRoleProposal,
        onNewConversation = vm::newConversation,
        onSelectConversation = vm::selectConversation,
        onDeleteConversation = vm::deleteConversation,
    )
}

@Composable
private fun TasksRoute(container: AppModelContainer) {
    val vm: TasksViewModel = viewModel(
        factory = viewModelFactory {
            initializer {
                TasksViewModel(
                    store = container.snapshotStore,
                    commands = container.commandSender,
                    onError = container::showEvent,
                )
            }
        }
    )
    val uiState by vm.uiState.collectAsState()
    // T-S2：写操作判据改绑 commandReady（出站通道绑定）
    val commandReady by container.connection.commandReady.collectAsState()
    // 快照态订阅派生 roles（评审 P10）：roles-only 的 STATE_DELTA（角色改名/新增角色）
    // 也触发重组，归属筛选 chips 不滞后；组合期直读 store.roles 无此保证
    val snapshotState = container.snapshotStore.state.collectAsState()
    TasksScreen(
        uiState = uiState,
        roles = if (snapshotState.value.loaded) container.snapshotStore.roles else emptyList(),
        commandReady = commandReady,
        onToggleTask = vm::toggleTask,
        onQuadrantFilterSelected = vm::selectQuadrantFilter,
        onToggleBigRocksOnly = vm::toggleBigRocksOnly,
        onToggleOwner = vm::toggleOwner,
        onToggleAllOwners = vm::toggleAllOwners,
        onCreateTask = vm::createTask,
    )
}

@Composable
private fun DashboardRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: DashboardViewModel = viewModel(
        factory = viewModelFactory { initializer { DashboardViewModel(container) } }
    )
    val uiState by vm.uiState.collectAsState()
    val notices by container.notifications.notices.collectAsState()
    val unreadCount = notices.count { !it.read }
    DashboardScreen(
        uiState = uiState,
        unreadNoticeCount = unreadCount,
        onOpenBriefing = { navController.navigate(Routes.BRIEFING) },
        onOpenReview = { navController.navigate(Routes.REVIEW) },
        onOpenNotifications = { navController.navigate(Routes.NOTIFICATIONS) },
        onMetricsScopeSelected = vm::setMetricsScope,
        onActivityWindowSelected = vm::setActivityWindow,
        onOpenMemory = { roleId -> navController.navigate("${Routes.MEMORY}/$roleId") },
    )
}

@Composable
private fun SettingsRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: SettingsViewModel = viewModel(
        key = "settings",
        factory = viewModelFactory { initializer { SettingsViewModel(container) } },
    )
    val uiState by vm.uiState.collectAsState()
    val connectionState by container.connection.state.collectAsState()
    SettingsScreen(
        uiState = uiState,
        connectionState = connectionState,
        onSetTheme = vm::setTheme,
        onToggleNoticeLevel = vm::toggleNoticeLevel,
        onProactivityRoleSelected = vm::selectProactivityRole,
        onProactivityLevelSelected = vm::setProactivityLevel,
        onDebugModeSelected = vm::selectDebugMode,
        onVersionTapped = vm::onVersionTapped,
        onUnpair = {
            vm.unpair()
            navController.navigate(Routes.PAIRING) {
                popUpTo(0) { inclusive = true }
            }
        },
    )
}

@Composable
private fun BriefingRoute(navController: NavHostController, container: AppModelContainer) {
    // 快照取数（T5 收敛：Route 层经容器实例，不再直读全局单例）
    val snapshotState = container.snapshotStore.state.collectAsState()
    BriefingScreen(
        briefing = container.snapshotStore.briefing,
        dataCutoffLabel = truncatedCutoffLabel(snapshotState.value.metadata, domain = "briefings"),
        onBack = { navController.popBackStack() },
    )
}

@Composable
private fun WeeklyReviewRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: WeeklyReviewViewModel = viewModel(
        factory = viewModelFactory { initializer { WeeklyReviewViewModel(container) } },
    )
    val uiState by vm.uiState.collectAsState()
    val snapshotState = container.snapshotStore.state.collectAsState()
    WeeklyReviewScreen(
        uiState = uiState,
        review = container.snapshotStore.weeklyReview,
        roles = container.snapshotStore.roles,
        dataCutoffLabel = truncatedCutoffLabel(snapshotState.value.metadata, domain = "weeklyReviews"),
        onBack = { navController.popBackStack() },
        onEnterPlan = vm::enterPlanPhase,
        onBackToReview = vm::enterReviewPhase,
        onUpdateItem = vm::updateItem,
        onRemoveItem = vm::removeItem,
        onAddItem = vm::addItem,
        onAdoptSuggestion = vm::adoptSuggestion,
        // 确认成功 → pop 返回（等价桌面 savePlan 成功后 onClose 关 Modal）
        onConfirmPlan = { vm.confirmPlan { navController.popBackStack() } },
    )
}

/** 截断元数据 → 明示标签；未截断（或指定域未被截断）返回 null（T7：不惊扰正常态）。 */
private fun truncatedCutoffLabel(
    metadata: com.egosync.companion.sync.SnapshotMetadata?,
    domain: String? = null,
): String? {
    if (metadata == null || !metadata.truncated) return null
    if (domain != null && domain !in metadata.truncatedDomains) return null
    // AC4 防御（评审 P16）：截断但缺 dataCutoffAt 时也不得静默吞掉截断事实，
    // 以缺失数据冒充完整（桌面契约上不应发生，防御性兜底）
    return metadata.dataCutoffAt
        ?.let { com.egosync.companion.sync.SnapshotMapper.formatDataCutoff(it) }
        ?: "部分历史数据因快照过大被截断"
}

@Composable
private fun NotificationCenterRoute(navController: NavHostController, container: AppModelContainer) {
    val notices by container.notifications.notices.collectAsState()
    // T-S2：通知响应等写操作判据改绑 commandReady
    val commandReady by container.connection.commandReady.collectAsState()
    NotificationCenterScreen(
        notices = notices,
        commandReady = commandReady,
        onBack = { navController.popBackStack() },
        onMarkAllRead = container.notifications::markAllRead,
        onRespond = container.notifications::respond,
    )
}

@Composable
private fun MemoryRoute(navController: NavHostController, container: AppModelContainer, roleId: String) {
    val vm: MemoryViewModel = viewModel(
        key = "memory-$roleId",
        factory = viewModelFactory {
            initializer {
                MemoryViewModel(
                    store = container.snapshotStore,
                    roleId = roleId,
                    commands = container.commandSender,
                    onError = container::showEvent,
                )
            }
        },
    )
    val uiState by vm.uiState.collectAsState()
    // T-S2：记忆写操作（选择性遗忘）判据改绑 commandReady
    val commandReady by container.connection.commandReady.collectAsState()
    MemoryScreen(
        uiState = uiState,
        commandReady = commandReady,
        onBack = { navController.popBackStack() },
        onCategorySelected = vm::setCategory,
        onToggleSource = vm::toggleSource,
        onOpenForgetConfirm = vm::openForgetConfirm,
        onCancelForget = vm::cancelForget,
        onConfirmForget = vm::confirmForget,
        onRetry = vm::reload,
        onRetrySources = vm::retrySources,
    )
}
