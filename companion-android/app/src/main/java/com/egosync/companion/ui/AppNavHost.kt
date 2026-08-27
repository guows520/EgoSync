package com.egosync.companion.ui

import androidx.compose.material3.Icon
import com.egosync.companion.ui.icons.LucideIcons
import androidx.compose.ui.graphics.vector.ImageVector

import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
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
import com.egosync.companion.connection.ConnectionStatusBanner
import com.egosync.companion.ui.briefing.BriefingScreen
import com.egosync.companion.ui.chat.ChatScreen
import com.egosync.companion.ui.chat.ChatViewModel
import com.egosync.companion.ui.components.DegradedOverlayHost
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
import com.egosync.companion.ui.theme.rememberReducedMotion

private data class TabSpec(
    val route: String,
    val label: String,
    val icon: ImageVector,
)

private val tabs = listOf(
    TabSpec("chat", "管家", LucideIcons.MessageSquare),
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
    val connectionState by container.connection.state.collectAsState()
    // reduced-motion（系统「移除动画」开启）：页面转场瞬时，不做淡入淡出
    val reducedMotion = rememberReducedMotion()

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
        composable(Routes.BRIEFING) { BriefingRoute(navController) }
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

        // 降级态遮罩全局覆盖（含二级页）：离线时任何页面都明示数据截止与引擎禁用
        if (connectionState is com.egosync.companion.connection.ConnectionState.Offline) {
            DegradedOverlayHost(
                state = connectionState,
                quickNotes = container.quickNotes,
            )
        }
    }
}

// ── 配对流路由 ─────────────────────────────────────────────────────────

@Composable
private fun PairingRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: com.egosync.companion.pairing.PairingViewModel = viewModel(
        factory = viewModelFactory {
            initializer { com.egosync.companion.pairing.PairingViewModel(container.connection) }
        }
    )
    val step by vm.step.collectAsState()
    val connectStage by vm.connectStage.collectAsState()

    com.egosync.companion.pairing.PairingScreen(
        step = step,
        connectStage = connectStage,
        onStartScan = vm::startScan,
        onScanCompleted = vm::onScanCompleted,
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
    )
}

// ── FR-21 空状态引导流路由 ─────────────────────────────────────────────

@Composable
private fun OnboardingRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: OnboardingViewModel = viewModel(
        factory = viewModelFactory { initializer { OnboardingViewModel(container) } }
    )
    val uiState by vm.uiState.collectAsState()
    val connectionState by container.connection.state.collectAsState()
    // 完成路径统一出口：清引导栈进主界面（镜像桌面 onComplete → butler 视图）
    val enterMain = {
        navController.navigate(Routes.DASHBOARD) {
            popUpTo(Routes.ONBOARDING) { inclusive = true }
            launchSingleTop = true
        }
    }
    OnboardingScreen(
        uiState = uiState,
        engineAvailable = connectionState.engineAvailable,
        onSendMessage = vm::sendMessage,
        onSkipOnboarding = { vm.skipOnboarding(onComplete = enterMain) },
        onRoleProposalConfirm = { name, icon, color, goal ->
            vm.confirmRoleProposal(name, icon, color, goal, onCompleted = enterMain)
        },
        onRoleProposalSkip = vm::skipRoleProposal,
    )
}

// ── 主界面壳：连接横幅 + 内容 + 底部四 Tab ──────────────────────────────

@Composable
private fun MainShellRoute(
    navController: NavHostController,
    container: AppModelContainer,
    currentTab: String,
) {
    val connectionState by container.connection.state.collectAsState()

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = { ConnectionStatusBanner(state = connectionState) },
        bottomBar = {
            val backStackEntry by navController.currentBackStackEntryAsState()
            val currentRoute = backStackEntry?.destination?.route
            NavigationBar(containerColor = MaterialTheme.colorScheme.surface) {
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
        Box(Modifier.fillMaxSize().padding(padding)) {
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
        factory = viewModelFactory { initializer { ChatViewModel(container) } }
    )
    val uiState by vm.uiState.collectAsState()
    val connectionState by container.connection.state.collectAsState()
    ChatScreen(
        uiState = uiState,
        engineAvailable = connectionState.engineAvailable,
        onSendMessage = vm::sendMessage,
        onActionCardRespond = vm::respondActionCard,
        onRoleSelected = vm::selectRole,
        onStopStreaming = vm::stopStreaming,
        onDecompositionRespond = vm::respondDecomposition,
        onRoleProposalConfirm = vm::confirmRoleProposal,
        onRoleProposalSkip = vm::skipRoleProposal,
    )
}

@Composable
private fun TasksRoute(container: AppModelContainer) {
    val vm: TasksViewModel = viewModel(
        factory = viewModelFactory { initializer { TasksViewModel(container) } }
    )
    val uiState by vm.uiState.collectAsState()
    val connectionState by container.connection.state.collectAsState()
    TasksScreen(
        uiState = uiState,
        engineAvailable = connectionState.engineAvailable,
        onToggleTask = vm::toggleTask,
        onQuadrantFilterSelected = vm::selectQuadrantFilter,
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
private fun BriefingRoute(navController: NavHostController) {
    BriefingScreen(onBack = { navController.popBackStack() })
}

@Composable
private fun WeeklyReviewRoute(navController: NavHostController, container: AppModelContainer) {
    val vm: WeeklyReviewViewModel = viewModel(
        factory = viewModelFactory { initializer { WeeklyReviewViewModel(container) } },
    )
    val uiState by vm.uiState.collectAsState()
    WeeklyReviewScreen(
        uiState = uiState,
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

@Composable
private fun NotificationCenterRoute(navController: NavHostController, container: AppModelContainer) {
    val notices by container.notifications.notices.collectAsState()
    val connectionState by container.connection.state.collectAsState()
    NotificationCenterScreen(
        notices = notices,
        engineAvailable = connectionState.engineAvailable,
        onBack = { navController.popBackStack() },
        onMarkAllRead = container.notifications::markAllRead,
        onRespond = container.notifications::respond,
    )
}

@Composable
private fun MemoryRoute(navController: NavHostController, container: AppModelContainer, roleId: String) {
    val vm: MemoryViewModel = viewModel(
        key = "memory-$roleId",
        factory = viewModelFactory { initializer { MemoryViewModel(container, roleId) } },
    )
    val uiState by vm.uiState.collectAsState()
    MemoryScreen(
        uiState = uiState,
        onBack = { navController.popBackStack() },
        onCategorySelected = vm::setCategory,
        onToggleSource = vm::toggleSource,
        onOpenForgetConfirm = vm::openForgetConfirm,
        onCancelForget = vm::cancelForget,
        onConfirmForget = vm::confirmForget,
    )
}
