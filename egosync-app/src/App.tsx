import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { cn } from './lib/utils';
import { Sidebar } from './components/layout/Sidebar';
import { TitleBar } from './components/layout/TitleBar';
import { ButlerView } from './components/butler/ButlerView';
import { RoleView } from './components/role/RoleView';
import { OnboardingView } from './components/onboarding/OnboardingView';
import { GlobalSettingsModal } from './components/settings/GlobalSettingsModal';
import { WeeklyReviewModal } from './components/modals/WeeklyReviewModal';
import { TaskModal } from './components/modals/TaskModal';
import { AddRoleModal } from './components/modals/AddRoleModal';
import { RoleConfirmModal } from './components/onboarding/RoleConfirmModal';
import type { RoleProposal } from './components/onboarding/RoleConfirmModal';
import { NotificationPanel } from './components/notifications/NotificationPanel';
import { appService } from './services/appService';
import { roleService } from './services/roleService';
import { useTauriEvent } from './hooks/useTauriEvent';
import { useNotifications } from './hooks/useNotifications';
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
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isReviewOpen, setIsReviewOpen] = useState(false);
  const [reviewInitialPhase, setReviewInitialPhase] = useState<'review' | 'plan'>('review');
  const [taskModalContext, setTaskModalContext] = useState<TaskModalContext | null>(null);
  const [isAddRoleOpen, setIsAddRoleOpen] = useState(false);
  const [roles, setRoles] = useState<Role[]>([]);
  const [archivedRoles, setArchivedRoles] = useState<Role[]>([]);
  const [isLoadingRoles, setIsLoadingRoles] = useState(true);
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
  useTauriEvent<NotificationNewPayload>(
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
  useTauriEvent<Q2ReminderPayload>(
    'q2:reminder',
    useCallback((_payload: Q2ReminderPayload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.1: 晨间简报生成事件到达时，递增 refreshTrigger 触发管家对话刷新
  useTauriEvent<BriefingGeneratedPayload>(
    'briefing:generated',
    useCallback((_payload: BriefingGeneratedPayload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.3: 大石头规划提醒事件到达时，打开 WeeklyReviewModal 规划阶段
  useTauriEvent<BigrockReminderPayload>(
    'bigrock:reminder',
    useCallback((_payload: BigrockReminderPayload) => {
      setReviewInitialPhase('plan');
      setIsReviewOpen(true);
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.6: 大石头保护提醒事件到达时，递增 refreshTrigger 触发管家对话刷新
  useTauriEvent<{ taskId: string; taskTitle: string; message: string; notificationId: string }>(
    'bigrock:protection',
    useCallback((_payload) => {
      setButlerChatRefreshTrigger(t => t + 1);
    }, []),
    []
  );

  // Story 6.4: 周复盘生成事件到达时，递增 refreshTrigger 触发管家对话刷新
  useTauriEvent<ReviewGeneratedPayload>(
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

  useTauriEvent<{ ownerId: string; skillId: string }>(
    'skill-registry-updated',
    useCallback(() => {
      void refreshRoles().catch(error => {
        console.error('Skill 注册表更新后刷新角色失败:', error);
      });
    }, [refreshRoles]),
    [refreshRoles]
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
  useTauriEvent<RoleProposedPayload>('role:proposed', useCallback((payload: RoleProposedPayload) => {
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

  useEffect(() => {
    appService.isFirstLaunch().then(isFirst => {
      if (isFirst) {
        setCurrentView('onboard');
        setIsLoadingRoles(false);
      } else {
        refreshAllRoles().catch(() => {}).finally(() => {
          setIsLoadingRoles(false);
        });
      }
    }).catch(() => {
      setIsLoadingRoles(false);
    });
  }, [refreshAllRoles]);

  // 加载完成后移除 splash 并显示窗口
  useEffect(() => {
    if (isLoadingRoles) return;
    const splash = document.getElementById('egosync-splash');
    if (splash) {
      splash.classList.add('hidden');
      setTimeout(() => splash.remove(), 400);
    }
    getCurrentWindow().show().catch(() => {});
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

  return (
    <div className={cn("flex flex-col h-screen font-sans transition-colors duration-300 bg-[#F1F3F5] text-slate-900 selection:bg-indigo-100 dark:bg-slate-800 dark:text-slate-100 dark:selection:bg-indigo-900 relative rounded-lg overflow-hidden", theme === 'dark' && "dark")}>
        <TitleBar />
        <div className="flex-1 flex overflow-hidden relative">
        <Sidebar
          roles={roles}
          currentView={currentView}
          isSettingsOpen={isSettingsOpen}
          onViewChange={setCurrentView}
          onOpenSettings={() => setIsSettingsOpen(true)}
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
          {isLoadingRoles ? null : (
          <>
          {currentView === 'onboard' && <OnboardingView onComplete={handleOnboardingComplete} onOpenSettings={() => setIsSettingsOpen(true)} />}
          {currentView === 'butler' && (
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
            />
          )}
          {roles.map(r => r.id === currentView && (
            <RoleView
              key={r.id}
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
            />
          ))}
          </>
          )}
          </main>
        </div>
        </div>

      {isSettingsOpen && (
        <GlobalSettingsModal
          onClose={() => setIsSettingsOpen(false)}
          onDataDestroyed={handleDataDestroyed}
          onDataImported={handleDataImported}
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