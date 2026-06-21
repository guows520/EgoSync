import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { cn } from './lib/utils';
import { Sidebar } from './components/layout/Sidebar';
import { TitleBar } from './components/layout/TitleBar';
import { ButlerView } from './components/butler/ButlerView';
import { RoleView } from './components/role/RoleView';
import { OnboardingView } from './components/onboarding/OnboardingView';
import { GlobalSettingsModal } from './components/settings/GlobalSettingsModal';
import { ArbitrationModal } from './components/modals/ArbitrationModal';
import { WeeklyReviewModal } from './components/modals/WeeklyReviewModal';
import { TaskModal } from './components/modals/TaskModal';
import { AddRoleModal } from './components/modals/AddRoleModal';
import { RoleConfirmModal } from './components/onboarding/RoleConfirmModal';
import type { RoleProposal } from './components/onboarding/RoleConfirmModal';
import { NotificationPanel } from './components/notifications/NotificationPanel';
import { appService } from './services/appService';
import { roleService } from './services/roleService';
import { useTauriEvent } from './hooks/useTauriEvent';
import { normalizeColorHex } from './lib/roleIcons';
import type { Role } from './types/role';
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
  const [isArbOpen, setIsArbOpen] = useState(false);
  const [isReviewOpen, setIsReviewOpen] = useState(false);
  const [taskModalContext, setTaskModalContext] = useState<TaskModalContext | null>(null);
  const [isAddRoleOpen, setIsAddRoleOpen] = useState(false);
  const [roles, setRoles] = useState<Role[]>([]);
  const [archivedRoles, setArchivedRoles] = useState<Role[]>([]);
  const [, setIsLoadingRoles] = useState(true);
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const stored = localStorage.getItem('egosync-theme');
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });
  const [roleInitialTab, setRoleInitialTab] = useState<string | null>(null);
  const [pendingRoleSourceNavigation, setPendingRoleSourceNavigation] = useState<SourceNavigationTarget | null>(null);
  const [pendingButlerSourceNavigation, setPendingButlerSourceNavigation] = useState<SourceNavigationTarget | null>(null);
  const [isNotifOpen, setIsNotifOpen] = useState(false);
  const taskActionsRef = useRef(new Map<string, TaskActions>());

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

  const handleOnboardingComplete = () => {
    refreshAllRoles().catch(() => {});
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
      backgroundColor: `${accent}0F`, // 6% alpha
    } as React.CSSProperties;
  }, [roles, currentView]);

  return (
    <div className={cn("flex flex-col h-screen font-sans transition-colors duration-300 bg-[#F1F3F5] text-slate-900 selection:bg-indigo-100 dark:bg-slate-800 dark:text-slate-100 dark:selection:bg-indigo-900 border border-slate-200 dark:border-slate-700 rounded-lg overflow-hidden", theme === 'dark' && "dark")}>
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
        />

        <div
          className="flex-1 flex flex-col overflow-hidden bg-[#F1F3F5] dark:bg-slate-800"
        >
          <main
            className="flex-1 relative overflow-hidden rounded-tl-2xl border-t border-l border-slate-200/60 dark:border-slate-700/60 bg-[#F8F9FA] dark:bg-slate-900 backdrop-blur-3xl transition-colors duration-300"
            style={mainTint}
          >
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
              onArchiveRole={handleArchiveRole}
              onDeleteRole={handleDeleteRole}
              activeRoleCount={roles.length}
              sourceNavigationTarget={pendingRoleSourceNavigation?.roleId === r.id ? pendingRoleSourceNavigation : null}
              onSourceNavigationHandled={() => setPendingRoleSourceNavigation(null)}
              onButlerSourceNavigation={handleButlerSourceNavigation}
              onRoleSourceNavigation={handleRoleSourceNavigation}
            />
          ))}
          </main>
        </div>
        </div>

      {isSettingsOpen && (
        <GlobalSettingsModal
          onClose={() => setIsSettingsOpen(false)}
          archivedRoles={archivedRoles}
          onRestoreRole={handleRestoreRole}
          onRefreshRoles={refreshAllRoles}
        />
      )}
      {isArbOpen && <ArbitrationModal onClose={() => setIsArbOpen(false)} />}
      {isReviewOpen && <WeeklyReviewModal roles={roles} onClose={() => setIsReviewOpen(false)} />}
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
      {isNotifOpen && <NotificationPanel onClose={() => setIsNotifOpen(false)} />}
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