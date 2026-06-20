import { useState, useEffect } from 'react';
import { cn } from '../../lib/utils';
import { RoleHeader, type RoleViewTab } from './RoleHeader';
import { RoleWorkspacePanel } from './RoleWorkspacePanel';
import { ChatStream } from '../chat/ChatStream';
import type { Role } from '../../types/role';
import type { SourceNavigationTarget } from '../../types/chat';
import type { Task, TaskActions } from '../../types/task';
import type { TaskScope } from '../../hooks/useTasks';

interface RoleViewProps {
  role: Role;
  roles?: Role[];
  onOpenTask: (scope: TaskScope, task?: Task | null) => void;
  onTasksApiReady?: (key: string, actions: TaskActions) => void;
  initialTab: string | null;
  onTabConsumed: () => void;
  onUpdateRole: (role: Role) => void;
  onArchiveRole: (id: string) => Promise<void> | void;
  onDeleteRole: (id: string) => Promise<void> | void;
  activeRoleCount: number;
  sourceNavigationTarget?: SourceNavigationTarget | null;
  onSourceNavigationHandled?: () => void;
  onButlerSourceNavigation?: (target: SourceNavigationTarget) => void;
  onRoleSourceNavigation?: (target: SourceNavigationTarget) => void;
}

export function RoleView({
  role,
  roles,
  onOpenTask,
  onTasksApiReady,
  initialTab,
  onTabConsumed,
  onUpdateRole,
  onArchiveRole,
  onDeleteRole,
  activeRoleCount,
  sourceNavigationTarget: externalSourceNavigationTarget,
  onSourceNavigationHandled,
  onButlerSourceNavigation,
  onRoleSourceNavigation,
}: RoleViewProps) {
  const [openTab, setOpenTab] = useState<RoleViewTab>(
    (initialTab as RoleViewTab) ?? null,
  );
  const [targetMemoryId, setTargetMemoryId] = useState<string | null>(null);
  const [sourceNavigationTarget, setSourceNavigationTarget] = useState<SourceNavigationTarget | null>(null);

  useEffect(() => {
    if (initialTab && openTab !== initialTab) {
      setOpenTab(initialTab as RoleViewTab);
      onTabConsumed?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialTab]);

  useEffect(() => {
    if (!externalSourceNavigationTarget) return;
    setSourceNavigationTarget(externalSourceNavigationTarget);
  }, [externalSourceNavigationTarget]);

  const toggleTab = (tab: Exclude<RoleViewTab, null>) => {
    setOpenTab(prev => (prev === tab ? null : tab));
  };

  const handleMemoryReferenceClick = (memoryId: string) => {
    setTargetMemoryId(memoryId);
    setOpenTab('memory');
  };

  const handleSourceMessageClick = (target: SourceNavigationTarget) => {
    if (target.roleId === role.id) {
      setSourceNavigationTarget(target);
      return;
    }
    if (target.roleId === null) {
      onButlerSourceNavigation?.(target);
      return;
    }
    onRoleSourceNavigation?.(target);
  };

  return (
    <div className="h-full flex flex-col transition-colors duration-300 animate-in fade-in duration-300">
      <RoleHeader role={role} openTab={openTab} onToggleTab={toggleTab} />

      <div className="flex-1 flex overflow-hidden">
        <div
          className={cn(
            'flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out',
            openTab ? 'w-[60%] border-r border-slate-200/60 dark:border-slate-700/60' : 'w-full',
          )}
        >
          <ChatStream
            role={role}
            onMemoryReferenceClick={handleMemoryReferenceClick}
            sourceNavigationTarget={sourceNavigationTarget}
            onSourceNavigationHandled={() => {
              setSourceNavigationTarget(null);
              onSourceNavigationHandled?.();
            }}
          />
        </div>

        <div
          className={cn(
            'bg-slate-50/60 dark:bg-slate-800/60 flex flex-col backdrop-blur-sm border-l border-white/40 dark:border-slate-700/40 shadow-[-8px_0_24px_rgba(0,0,0,0.02)] overflow-hidden transition-all duration-500 ease-in-out',
            openTab ? 'w-[40%] opacity-100' : 'w-0 opacity-0 border-l-0 shadow-none pointer-events-none',
          )}
          aria-hidden={!openTab}
        >
          {openTab && (
            <RoleWorkspacePanel
              role={role}
              roles={roles}
              currentTab={openTab}
              setTab={setOpenTab}
              onOpenTask={(task: Task | null) => onOpenTask({ ownerType: 'role', roleId: role.id }, task)}
              onTasksApiReady={(actions: TaskActions) => onTasksApiReady?.(`role:${role.id}`, actions)}
              onUpdateRole={onUpdateRole}
              onArchiveRole={onArchiveRole}
              onDeleteRole={onDeleteRole}
              activeRoleCount={activeRoleCount}
              targetMemoryId={targetMemoryId}
              onTargetMemoryHandled={() => setTargetMemoryId(null)}
              onSourceMessageClick={handleSourceMessageClick}
            />
          )}
        </div>
      </div>
    </div>
  );
}