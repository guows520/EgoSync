import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauriHost } from '@/transport';
import { cn } from './lib/utils';
import { Sidebar } from './components/layout/Sidebar';
import { BottomTabBar, type BottomTab } from './components/layout/BottomTabBar';
import { RoleListPanel } from './components/layout/RoleListPanel';
import { TitleBar } from './components/layout/TitleBar';
import { ButlerView } from './components/butler/ButlerView';
import { RoleView } from './components/role/RoleView';
import { OnboardingView } from './components/onboarding/OnboardingView';
import { GlobalSettingsModal } from './components/settings/GlobalSettingsModal';
import { MobileSettingsView } from './components/settings/MobileSettingsView';
import { WeeklyReviewModal } from './components/modals/WeeklyReviewModal';
import { TaskModal } from './components/modals/TaskModal';
import { AddRoleModal } from './components/modals/AddRoleModal';
import { RoleConfirmModal } from './components/onboarding/RoleConfirmModal';
import type { RoleProposal } from './components/onboarding/RoleConfirmModal';
import { NotificationPanel } from './components/notifications/NotificationPanel';
import { appService } from './services/appService';
import { roleService } from './services/roleService';
import { useEngineEvent } from './hooks/useEngineEvent';
import { useNotifications } from './hooks/useNotifications';
import type { TransportReconnectedPayload } from '@/transport';
import { normalizeColorHex } from './lib/roleIcons';
import { playNotificationSound } from './lib/notificationSound';
import type { Role } from './types/role';
import type { NotificationNewPayload } from './types/notification';
import type { Q2ReminderPayload } from './types/q2Reminder';
import type { BriefingGeneratedPayload } from './types/briefing';
import type { BigrockReminderPayload } from './types/bigrockReminder';
import type { ReviewGeneratedPayload } from './types/review';
import type { SourceNavigationTarget } from './types/chat';
import type { CreateTaskInput, Task, TaskActions, UpdateTaskInput } from './types/task';
import type { TaskScope } from './hooks/useTasks';

const BUTLER_ACCENT = '#6366F1';

interface TaskModalContext {
  scope: TaskScope;
  task: Task | null;
}

export default function App() {
  const [currentView, setCurrentView] = useState('butler');
  // Story 16.4：移动端「角色」tab 两级——mobileRoleRoot=true 呈现角色
  // 列表根（第一级）；点列表行切角色并进详情（false）。桌面不可达
  // （BottomTabBar md:hidden）；跨断点互斥靠 max-md:hidden/md:hidden
  // 类对（详情在根呈现时 max-md:hidden 让位，桌面宽度下无效）。
  const [mobileRoleRoot, setMobileRoleRoot] = useState(false);
  // Story 16.4：移动设置 tab 打开桌面同款 GlobalSettingsModal 的落点 tab
  const [settingsInitialTab, setSettingsInitialTab] = useState<'llm' | 'mcp' | 'scheduler' | 'data'>('llm');
  // Story 16.4：断点回桌面时收敛移动专属状态（BREAKPOINT_EDGE 矩阵行——
  // 「互斥切换无残留」）：currentView='settings' 与 mobileRoleRoot 均由移动
  // tab 产生，放大到 ≥768px 必须还原子（设置面残留/列表根残留均属布局残渣）
  useEffect(() => {
    const desktop = window.matchMedia('(min-width: 768px)');
    const converge = () => {
      if (!desktop.matches) return;
      setCurrentView(v => (v === 'settings' ? 'butler' : v));
      setMobileRoleRoot(false);
    };
    converge();
    // 旧 Safari 回退（vite target safari13）：addEventListener 缺席时用
    // legacy addListener/removeListener——iOS 13 上直接调用前者会抛
    // TypeError 打断 effect（白屏风险）
    if (typeof desktop.addEventListener === 'function') {
      desktop.addEventListener('change', converge);
      return () => desktop.removeEventListener('change', converge);
    }
    const legacy = desktop as MediaQueryList & {
      addListener?: (cb: () => void) => void;
      removeListener?: (cb: () => void) => void;
    };
    legacy.addListener?.(converge);
    return () => legacy.removeListener?.(converge);
  }, []);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isReviewOpen, setIsReviewOpen] = useState(false);
  const [reviewInitialPhase, setReviewInitialPhase] = useState<'review' | 'plan'>('review');
  const [taskModalContext, setTaskModalContext] = useState<TaskModalContext | null>(null);
  const [isAddRoleOpen, setIsAddRoleOpen] = useState(false);
  const [roles, setRoles] = useState<Role[]>([]);
  const [archivedRoles, setArchivedRoles] = useState<Role[]>([]);
  const [isLoadingRoles, setIsLoadingRoles] = useState(true);
  // 2026-09-22（onboarding 劫持案）：首启门失败态与重试计数——失败时
  // 显式报错给出重试入口，禁止静默落管家视图（详见首启门 effect 注释）。
  const [launchGateError, setLaunchGateError] = useState(false);
  const [gateRetryCount, setGateRetryCount] = useState(0);
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const stored = localStorage.getItem('egosync-theme');
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });
  const [roleInitialTab, setRoleInitialTab] = useState<string | null>(null);
  const [pendingRoleSourceNavigation, setPendingRoleSourceNavigation] = useState<SourceNavigationTarget | null>(null);
  const [pendingButlerSourceNavigation, setPendingButlerSourceNavigation] = useState<SourceNavigationTarget | null>(null);
  const [isNotifOpen, setIsNotifOpen] = useState(false);
  const { alertUnreadCount, whisperUnreadCount, notifications, isLoading: isNotifLoading, markAsRead } = useNotifications();
  const taskActionsRef = useRef(new Map<string, TaskActions>());

  const knockNotifications = useMemo(
    () => notifications.filter(n => n.level === 'knock' && !n.isRead),
    [notifications]
  );

  const handleDismissKnock = useCallback((id: string) => {
    markAsRead(id);
  }, [markAsRead]);

  // Story 4.5: 敲门通知到达时，若用户开启声音设置则播放提示音（默认关闭）
  useEngineEvent<NotificationNewPayload>(
    'notification:new',
    useCallback((payload: NotificationNewPayload) => {
      if (payload.level !== 'knock') return;
      void (async () => {
        try {
          const enabled = await appService.getSetting('notification.knock_sound');
          if (enabled === 'true') playNotificationSound();
        } catch (e) {
          console.error('读取敲门声音设置失败:', e);
        }
      })();
    }, []),
    []
  );

  // Story 4.6: Q2 保护提醒事件到达时，递增 refreshTrigger 触发管家对话刷新
  const [butlerChatRefreshTrigger, setButlerChatRefreshTrigger] = useState(0);
  useEngineEvent<Q2ReminderPayload>(
    'q2:reminder',
    useCallback((_payload: Q2ReminderPayload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.1: 晨间简报生成事件到达时，递增 refreshTrigger 触发管家对话刷新
  useEngineEvent<BriefingGeneratedPayload>(
    'briefing:generated',
    useCallback((_payload: BriefingGeneratedPayload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.3: 大石头规划提醒事件到达时，打开 WeeklyReviewModal 规划阶段
  useEngineEvent<BigrockReminderPayload>(
    'bigrock:reminder',
    useCallback((_payload: BigrockReminderPayload) => {
      setReviewInitialPhase('plan');
      setIsReviewOpen(true);
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.6: 大石头保护提醒事件到达时，递增 refreshTrigger 触发管家对话刷新
  useEngineEvent<{ taskId: string; taskTitle: string; message: string; notificationId: string }>(
    'bigrock:protection',
    useCallback((_payload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.4: 周复盘生成事件到达时，递增 refreshTrigger 触发管家对话刷新
  useEngineEvent<ReviewGeneratedPayload>(
    'review:generated',
    useCallback((_payload: ReviewGeneratedPayload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 2.5: 管家涌现角色提议（非 onboarding 模式）
  const [butlerProposal, setButlerProposal] = useState<RoleProposal | null>(null);
  const [isButlerProposalOpen, setIsButlerProposalOpen] = useState(false);
  const [isButlerProposalBusy, setIsButlerProposalBusy] = useState(false);

  const refreshRoles = useCallback(async () => {
    const realRoles = await roleService.list();
    setRoles(realRoles);
    return realRoles;
  }, []);

  const refreshArchivedRoles = useCallback(async () => {
    const archived = await roleService.listArchived();
    setArchivedRoles(archived);
    return archived;
  }, []);

  const refreshAllRoles = useCallback(async () => {
    const [active, archived] = await Promise.all([refreshRoles(), refreshArchivedRoles()]);
    return { active, archived };
  }, [refreshArchivedRoles, refreshRoles]);

  useEngineEvent<{ ownerId: string; skillId: string }>(
    'skill-registry-updated',
    useCallback(() => {
      void refreshRoles().catch(error => {
        console.error('Skill 注册表更新后刷新角色失败:', error);
      });
    }, [refreshRoles]),
    [refreshRoles]
  );

  // Story 16.2：重连恢复——role_list / role_list_archived 均在白名单内，
  // 直接消费重放结果集（避免重复查询）；任一缺失/失败回退 refreshAllRoles。
  // 侧边栏/角色视图重连后反映最新态；写入类零重放（transport 契约）。
  useEngineEvent<TransportReconnectedPayload>(
    'transport:reconnected',
    useCallback((payload: TransportReconnectedPayload) => {
      const replayedActive = payload?.results?.['role_list'];
      const replayedArchived = payload?.results?.['role_list_archived'];
      if (Array.isArray(replayedActive) && Array.isArray(replayedArchived)) {
        setRoles(replayedActive as Role[]);
        setArchivedRoles(replayedArchived as Role[]);
        return;
      }
      void refreshAllRoles().catch(error => {
        console.error('重连后刷新角色列表失败:', error);
      });
    }, [refreshAllRoles]),
    [refreshAllRoles]
  );

  // Story 2.5 AC-6: 管家模式下 role:proposed 事件监听
  // onboarding 模式下跳过（OnboardingView 自己处理）
  interface RoleProposedPayload {
    conversationId: string;
    name: string;
    icon: string | null;
    color: string | null;
    goal: string | null;
  }
  useEngineEvent<RoleProposedPayload>('role:proposed', useCallback((payload: RoleProposedPayload) => {
    if (currentView === 'onboard') return; // OnboardingView 自己处理
    console.info('[App] 收到管家角色提议:', payload.name);
    setButlerProposal({
      name: payload.name,
      icon: payload.icon,
      color: payload.color,
      goal: payload.goal,
    });
    setIsButlerProposalOpen(true);
  }, [currentView]), [currentView]);

  const handleButlerProposalConfirm = useCallback(
    async (values: { name: string; icon: string; color: string; goal: string }) => {
      if (isButlerProposalBusy) return;
      setIsButlerProposalBusy(true);
      try {
        const role = await roleService.create({
          name: values.name,
          icon: values.icon,
          color: values.color,
          goal: values.goal || undefined,
        });
        console.info('[App] 涌现角色创建成功:', role.name, role.id);
        await refreshRoles();
        setIsButlerProposalOpen(false);
        setButlerProposal(null);
      } catch (e) {
        console.error('[App] 涌现角色创建失败:', e);
      } finally {
        setIsButlerProposalBusy(false);
      }
    },
    [isButlerProposalBusy, refreshRoles],
  );

  const handleButlerProposalCancel = useCallback(() => {
    setIsButlerProposalOpen(false);
    setButlerProposal(null);
  }, []);

  // 2026-09-22（onboarding 劫持案修复）：首启门失败不再静默落管家视图。
  // 原实现 `.catch(() => setIsLoadingRoles(false))` 在 app_is_first_launch
  // 瞬时失败时让 currentView 保持默认 'butler' 直接可用——但
  // onboarding_completed 标记并未落，用户在「未完成引导」状态下正常使用
  // 后，下次页面加载即被 OnboardingView 劫持（历史对话入口被引导页顶掉，
  // web UAT 实证）。改为对齐 ChatStream.initializeConversation 的自愈模式
  // （e2e 取证结论：启动窗口瞬时 REST 失败需有限重试）：3 次尝试、1.5s
  // 退避、代际守卫（重试/重挂载取消旧尝试）；终态失败显式报错给出重试
  // 入口——禁止静默放行。
  const launchGateGenerationRef = useRef(0);
  useEffect(() => {
    const generation = ++launchGateGenerationRef.current;
    let cancelled = false;

    const runGate = async (attempt: number): Promise<void> => {
      try {
        const isFirst = await appService.isFirstLaunch();
        if (cancelled || launchGateGenerationRef.current !== generation) return;
        setLaunchGateError(false);
        if (isFirst) {
          setCurrentView('onboard');
          setIsLoadingRoles(false);
        } else {
          refreshAllRoles().catch(() => {}).finally(() => {
            if (!cancelled && launchGateGenerationRef.current === generation) {
              setIsLoadingRoles(false);
            }
          });
        }
      } catch {
        if (cancelled || launchGateGenerationRef.current !== generation) return;
        if (attempt < 2) {
          setTimeout(() => { void runGate(attempt + 1); }, 1500);
        } else {
          console.error('首启判定连续失败（3 次尝试均失败），进入显式错误态');
          setLaunchGateError(true);
          setIsLoadingRoles(false);
        }
      }
    };

    void runGate(0);
    return () => { cancelled = true; };
  }, [refreshAllRoles, gateRetryCount]);

  const handleLaunchGateRetry = useCallback(() => {
    setIsLoadingRoles(true);
    setLaunchGateError(false);
    setGateRetryCount(c => c + 1);
  }, []);

  // 加载完成后移除 splash 并显示窗口
  useEffect(() => {
    if (isLoadingRoles) return;
    const splash = document.getElementById('egosync-splash');
    if (splash) {
      splash.classList.add('hidden');
      setTimeout(() => splash.remove(), 400);
    }
    // 浏览器宿主：无窗口 API（getCurrentWindow 会抛错），show() 仅 Tauri 分支执行
    if (isTauriHost()) {
      getCurrentWindow().show().catch(() => {});
    }
  }, [isLoadingRoles]);

  const handleOnboardingComplete = () => {
    refreshAllRoles().catch(() => {});
    setCurrentView('butler');
  };

  const handleDataDestroyed = async () => {
    setIsSettingsOpen(false);
    await refreshAllRoles();
    setCurrentView('onboard');
  };

  const handleDataImported = async () => {
    setIsSettingsOpen(false);
    await refreshAllRoles();
    setCurrentView('butler');
  };

  const handleArchiveRole = async (id: string) => {
    await roleService.archive(id);
    await refreshAllRoles();
    if (currentView === id) setCurrentView('butler');
  };

  const handleRestoreRole = async (id: string) => {
    await roleService.restore(id);
    await refreshAllRoles();
  };

  const handleDeleteRole = async (id: string) => {
    await roleService.delete(id);
    await refreshAllRoles();
    if (currentView === id) setCurrentView('butler');
  };

  const handleUpdateRole = (updated: Role) => {
    setRoles(prev => prev.map(role => role.id === updated.id ? updated : role));
  };

  const handleSourceNavigation = (target: SourceNavigationTarget) => {
    if (!target.roleId) {
      setPendingRoleSourceNavigation(null);
      setPendingButlerSourceNavigation(target);
      setCurrentView('butler');
      return;
    }
    setPendingButlerSourceNavigation(null);
    setPendingRoleSourceNavigation(target);
    setCurrentView(target.roleId);
  };

  const handleRoleSourceNavigation = (target: SourceNavigationTarget) => {
    handleSourceNavigation(target);
  };

  const handleOpenTask = useCallback((scope: TaskScope, task: Task | null = null) => {
    setTaskModalContext({ scope, task });
  }, []);

  const handleTasksApiReady = useCallback((key: string, actions: TaskActions) => {
    taskActionsRef.current.set(key, actions);
  }, []);

  const handleSaveTask = useCallback(async (input: CreateTaskInput | UpdateTaskInput) => {
    if (!taskModalContext) return;
    const key = taskModalContext.scope.ownerType === 'role'
      ? `role:${taskModalContext.scope.roleId}`
      : 'butler';
    const actions = taskActionsRef.current.get(key);
    if (!actions) throw new Error('当前任务列表尚未准备好');

    if (taskModalContext.task) {
      await actions.updateTask(taskModalContext.task.id, input as UpdateTaskInput);
    } else {
      await actions.createTask(input as CreateTaskInput);
    }
  }, [taskActionsRef, taskModalContext]);

  const handleButlerSourceNavigation = (target: SourceNavigationTarget) => {
    handleSourceNavigation(target);
  };

  const toggleTheme = () => {
    setTheme(t => {
      const next = t === 'light' ? 'dark' : 'light';
      localStorage.setItem('egosync-theme', next);
      document.documentElement.classList.toggle('dark', next === 'dark');
      return next;
    });
  };

  // AC-1 / AC-3: 主区色温由当前视图驱动。butler/onboard 走默认靛蓝；
  // 进入角色时切到 role.color，并叠 6% alpha 作为背景 tint。
  // 子元素只要挂 transition-colors，CSS 变量切换就会自动 300ms 过渡。
  const mainTint = useMemo(() => {
    const activeRole = roles.find(r => r.id === currentView);
    if (!activeRole) {
      return { '--role-accent': BUTLER_ACCENT } as React.CSSProperties;
    }
    const accent = normalizeColorHex(activeRole.color);
    return {
      '--role-accent': accent,
    } as React.CSSProperties;
  }, [roles, currentView]);

  // Story 16.4：移动设置入口——打开桌面同款 GlobalSettingsModal 并落指定
  // tab（同一批组件不重写；通知=调度时间 tab 内的敲门通知声音设置）
  const handleOpenSettingsTab = (tab: 'llm' | 'mcp' | 'scheduler' | 'data') => {
    setSettingsInitialTab(tab);
    setIsSettingsOpen(true);
  };

  // Story 16.4：底部 tab 激活态——角色 tab 在「角色详情」与「角色列表根」
  // 两态均激活（标准 tab 行为：再点一次回根）
  const isRoleView = roles.some(r => r.id === currentView);
  const bottomActiveTab: BottomTab = currentView === 'settings'
    ? 'settings'
    : (mobileRoleRoot || isRoleView) ? 'roles' : 'butler';

  return (
    // Story 16.2：dvh 优先（iOS Safari 地址栏抖动缓解）、100vh 回退
    // （supports 门控变体与基础 h-screen 在 twMerge 中不冲突，双声明共存）；
    // 左右安全区内边距（评审修复）：viewport-fit=cover 下 iOS 横屏刘海
    // 不压侧栏与内容（非刘海/桌面环境 env()=0 零影响）。
    <div className={cn("flex flex-col h-screen supports-[height:100dvh]:h-dvh font-sans transition-colors duration-300 bg-[#F1F3F5] text-slate-900 selection:bg-indigo-100 dark:bg-slate-800 dark:text-slate-100 dark:selection:bg-indigo-900 relative rounded-lg overflow-hidden pl-[max(0px,env(safe-area-inset-left))] pr-[max(0px,env(safe-area-inset-right))]", theme === 'dark' && "dark")}>
        <TitleBar />
        <div className="flex-1 flex overflow-hidden relative">
        <Sidebar
          roles={roles}
          currentView={currentView}
          isSettingsOpen={isSettingsOpen}
          onViewChange={setCurrentView}
          onOpenSettings={() => { setSettingsInitialTab('llm'); setIsSettingsOpen(true); }}
          onCloseSettings={() => setIsSettingsOpen(false)}
          onAddRole={() => setIsAddRoleOpen(true)}
          onArchiveRole={handleArchiveRole}
          onDeleteRole={handleDeleteRole}
          theme={theme}
          onToggleTheme={toggleTheme}
          onEditRole={(id: string) => { setCurrentView(id); setRoleInitialTab('settings'); setIsSettingsOpen(false); }}
          isNotifOpen={isNotifOpen}
          onToggleNotif={() => setIsNotifOpen(v => !v)}
          unreadCount={alertUnreadCount}
          whisperUnread={whisperUnreadCount}
        />

        <div
          className="flex-1 flex flex-col overflow-hidden bg-[#F1F3F5] dark:bg-slate-800"
        >
          <main
            className="flex-1 relative overflow-hidden rounded-tl-2xl border-t border-l border-slate-200/60 dark:border-slate-700/60 bg-[#F8F9FA] dark:bg-slate-900 backdrop-blur-3xl transition-colors duration-300"
            style={mainTint}
          >
          {launchGateError ? (
            <div className="h-full flex items-center justify-center p-8" data-testid="launch-gate-error">
              <div className="max-w-md text-center space-y-4">
                <div className="text-slate-800 dark:text-slate-100 font-semibold text-lg">启动检查失败</div>
                <p className="text-slate-500 dark:text-slate-400 text-[14px] leading-relaxed">
                  无法确认是否为首次使用（已连续尝试 3 次）。<br />
                  请检查服务连接后重试。
                </p>
                <button
                  type="button"
                  onClick={handleLaunchGateRetry}
                  className="px-5 py-2.5 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity"
                >
                  重试
                </button>
              </div>
            </div>
          ) : isLoadingRoles ? null : (
          <>
          {/* Story 16.4：移动端「角色」tab 第一级（列表根）——md:hidden
              桌面不可见；详情视图同时在树下挂载（根呈现时 max-md:hidden），
              跨断点缩放互不留空白。 */}
          {mobileRoleRoot && (
            <div className="md:hidden h-full">
              <RoleListPanel
                roles={roles}
                currentView={currentView}
                onSelectButler={() => { setMobileRoleRoot(false); setCurrentView('butler'); }}
                onSelectRole={(id: string) => { setMobileRoleRoot(false); setCurrentView(id); }}
                onAddRole={() => setIsAddRoleOpen(true)}
              />
            </div>
          )}
          {currentView === 'onboard' && <OnboardingView onComplete={handleOnboardingComplete} onOpenSettings={() => { setSettingsInitialTab('llm'); setIsSettingsOpen(true); }} />}
          {currentView === 'butler' && (
            <div className={cn('h-full', mobileRoleRoot && 'max-md:hidden')}>
              <ButlerView
                roles={roles}
                onViewChange={setCurrentView}
                archivedRoles={archivedRoles}
                onRestoreRole={handleRestoreRole}
                onUpdateRole={handleUpdateRole}
                onRoleSourceNavigation={handleRoleSourceNavigation}
                sourceNavigationTarget={pendingButlerSourceNavigation}
                onSourceNavigationHandled={() => setPendingButlerSourceNavigation(null)}
                onOpenTask={handleOpenTask}
                onTasksApiReady={handleTasksApiReady}
                knockNotifications={knockNotifications}
                onDismissKnock={handleDismissKnock}
                chatRefreshTrigger={butlerChatRefreshTrigger}
                isNotifOpen={isNotifOpen}
                onToggleNotif={() => setIsNotifOpen(v => !v)}
                unreadCount={alertUnreadCount}
                whisperUnread={whisperUnreadCount}
              />
            </div>
          )}
          {roles.map(r => r.id === currentView && (
            <div key={r.id} className={cn('h-full', mobileRoleRoot && 'max-md:hidden')}>
              <RoleView
                role={r}
                roles={roles}
                onOpenTask={handleOpenTask}
                onTasksApiReady={handleTasksApiReady}
                initialTab={roleInitialTab}
                onTabConsumed={() => setRoleInitialTab(null)}
                onUpdateRole={handleUpdateRole}
                sourceNavigationTarget={pendingRoleSourceNavigation?.roleId === r.id ? pendingRoleSourceNavigation : null}
                onSourceNavigationHandled={() => setPendingRoleSourceNavigation(null)}
                onButlerSourceNavigation={handleButlerSourceNavigation}
                onRoleSourceNavigation={handleRoleSourceNavigation}
                onSwitchRole={() => setMobileRoleRoot(true)}
                onAddRole={() => setIsAddRoleOpen(true)}
              />
            </div>
          ))}
          {/* Story 16.4：移动端「设置」tab——桌面 GlobalSettingsModal 同款
              全量的移动入口（7 项：模型服务/MCP/调度/数据/通知/主题/登出） */}
          {currentView === 'settings' && (
            <MobileSettingsView
              onOpenSettings={handleOpenSettingsTab}
              theme={theme}
              onToggleTheme={toggleTheme}
            />
          )}
          </>
          )}
          </main>
        </div>
        </div>

        {/* Story 16.4：底部 tab 导航（<768px 侧栏退场替代）。onboarding
            聚焦期不呈现（首启流跑完进管家即出现）；launchGateError 主区
            锁死错误屏与 isLoadingRoles 空窗期同样不呈现——避免可见但不
            可点的死控件（与主区内容的呈现条件对齐）。 */}
        {currentView !== 'onboard' && !launchGateError && !isLoadingRoles && (
          <BottomTabBar
            activeTab={bottomActiveTab}
            onButlerTab={() => { setMobileRoleRoot(false); setCurrentView('butler'); }}
            onRolesTab={() => {
              setMobileRoleRoot(true);
              // 从设置面切角色 tab：先落回管家上下文（设置面不挂条件让位
              // 类——列表根与设置面同显会叠屏）
              if (currentView === 'settings') setCurrentView('butler');
            }}
            onSettingsTab={() => { setMobileRoleRoot(false); setCurrentView('settings'); }}
          />
        )}

      {isSettingsOpen && (
        <GlobalSettingsModal
          onClose={() => setIsSettingsOpen(false)}
          onDataDestroyed={handleDataDestroyed}
          onDataImported={handleDataImported}
          initialTab={settingsInitialTab}
        />
      )}
      {isReviewOpen && <WeeklyReviewModal roles={roles} onClose={() => { setIsReviewOpen(false); setReviewInitialPhase('review'); }} initialPhase={reviewInitialPhase} />}
      {taskModalContext && (
        <TaskModal
          scope={taskModalContext.scope}
          roles={roles}
          task={taskModalContext.task}
          onClose={() => setTaskModalContext(null)}
          onSave={handleSaveTask}
        />
      )}
      {isAddRoleOpen && <AddRoleModal onClose={() => setIsAddRoleOpen(false)} onAdd={(role: Role) => { setRoles(prev => [...prev, role]); setIsAddRoleOpen(false); }} />}
      {isNotifOpen && <NotificationPanel onClose={() => setIsNotifOpen(false)} notifications={notifications} isLoading={isNotifLoading} markAsRead={markAsRead} />}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 z-50 rounded-lg ring-1 ring-inset ring-[#D5D9DE] dark:ring-white/12"
      />
      <RoleConfirmModal
        open={isButlerProposalOpen}
        proposal={butlerProposal}
        onConfirm={handleButlerProposalConfirm}
        onCancel={handleButlerProposalCancel}
        busy={isButlerProposalBusy}
      />
    </div>
  );
}