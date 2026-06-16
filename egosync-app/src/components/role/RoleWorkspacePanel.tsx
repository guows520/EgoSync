import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { TasksTab } from './TasksTab';
import { MemoryTab } from './MemoryTab';
import { SettingsTab } from './SettingsTab';
import { memoryService } from '../../services/memoryService';
import { useTasks } from '../../hooks/useTasks';
import type { MemoryCategory } from '../../types/memory';
import type { Role } from '../../types/role';
import type { SourceNavigationTarget } from '../../types/chat';
import type { Task, TaskActions } from '../../types/task';

type WorkspaceTab = 'tasks' | 'memory' | 'settings' | null;

interface RoleWorkspacePanelProps {
  role: Role;
  roles?: Role[];
  currentTab: WorkspaceTab;
  setTab: (tab: WorkspaceTab) => void;
  onOpenTask?: (task: Task | null) => void;
  onTasksApiReady?: (actions: TaskActions) => void;
  onUpdateRole?: (role: Role) => void;
  onArchiveRole?: (id: string) => Promise<void> | void;
  onDeleteRole?: (id: string) => Promise<void> | void;
  activeRoleCount?: number;
  targetMemoryId?: string | null;
  onTargetMemoryHandled?: () => void;
  onSourceMessageClick?: (target: SourceNavigationTarget) => void;
}

export function RoleWorkspacePanel({
  role,
  roles,
  currentTab,
  setTab,
  onOpenTask,
  onTasksApiReady,
  onUpdateRole,
  onArchiveRole,
  onDeleteRole,
  activeRoleCount,
  targetMemoryId,
  onTargetMemoryHandled,
  onSourceMessageClick,
}: RoleWorkspacePanelProps) {
  const [memoryCount, setMemoryCount] = useState<number | null>(null);
  const [memoryCategory, setMemoryCategory] = useState<MemoryCategory | undefined>();
  const [memoryCountReloadKey, setMemoryCountReloadKey] = useState(0);
  const { tasks, isLoading: isLoadingTasks, error: tasksError, createTask, updateTask, deleteTask } = useTasks(role.id);

  useEffect(() => {
    onTasksApiReady?.({
      createTask,
      updateTask,
      deleteTask,
    });
  }, [createTask, deleteTask, onTasksApiReady, updateTask]);

  const effectiveMemoryCategory = targetMemoryId ? undefined : memoryCategory;

  useEffect(() => {
    let cancelled = false;
    setMemoryCount(null);
    memoryService
      .count({ roleId: role.id, category: effectiveMemoryCategory })
      .then(count => {
        if (!cancelled) setMemoryCount(count);
      })
      .catch(e => {
        console.error('加载记忆数量失败:', e);
        if (!cancelled) setMemoryCount(0);
      });

    return () => {
      cancelled = true;
    };
  }, [role.id, effectiveMemoryCategory, memoryCountReloadKey]);

  useEffect(() => {
    if (!targetMemoryId) return;
    setMemoryCategory(undefined);
    setTab('memory');
  }, [setTab, targetMemoryId]);

  const handleMemoryCategoryChange = (next: MemoryCategory | undefined) => {
    if (targetMemoryId) {
      setMemoryCategory(undefined);
      setMemoryCountReloadKey(key => key + 1);
      return;
    }
    setMemoryCategory(next);
  };

  const memoryLabel = memoryCount === null ? '记忆档案' : `记忆档案 (${memoryCount})`;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b border-slate-200/80 px-6 pt-4 bg-white/60 backdrop-blur-md shrink-0">
        <div className="flex gap-8">
          <button
            onClick={() => setTab('tasks')}
            className={cn(
              'pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]',
              currentTab === 'tasks' ? 'border-current' : 'border-transparent text-slate-500 hover:text-slate-800',
            )}
            style={currentTab === 'tasks' ? { color: role.color } : undefined}
          >
            任务清单
          </button>
          <button
            onClick={() => setTab('memory')}
            className={cn(
              'pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]',
              currentTab === 'memory' ? 'border-current' : 'border-transparent text-slate-500 hover:text-slate-800',
            )}
            style={currentTab === 'memory' ? { color: role.color } : undefined}
          >
            {memoryLabel}
          </button>
          <button
            onClick={() => setTab('settings')}
            className={cn(
              'pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]',
              currentTab === 'settings' ? 'border-current' : 'border-transparent text-slate-500 hover:text-slate-800',
            )}
            style={currentTab === 'settings' ? { color: role.color } : undefined}
          >
            设置
          </button>
        </div>
        <button onClick={() => setTab(null)} className="pb-3.5 text-slate-400 hover:text-slate-700 transition-colors">
          <X size={18} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-6 scroll-smooth">
        {currentTab === 'tasks' && (
          <TasksTab
            role={role}
            tasks={tasks}
            isLoading={isLoadingTasks}
            error={tasksError}
            onOpenTask={(task: Task | null) => onOpenTask?.(task)}
            onDeleteTask={deleteTask}
          />
        )}
        {currentTab === 'memory' && (
          <MemoryTab
            roleId={role.id}
            category={effectiveMemoryCategory}
            onCategoryChange={handleMemoryCategoryChange}
            onMemoryDeleted={() => setMemoryCountReloadKey(key => key + 1)}
            targetMemoryId={targetMemoryId}
            onTargetMemoryHandled={onTargetMemoryHandled}
            onSourceMessageClick={onSourceMessageClick}
          />
        )}
        {currentTab === 'settings' && (
          <SettingsTab
            role={role}
            activeRoles={roles ?? []}
            activeRoleCount={activeRoleCount ?? 1}
            onUpdateRole={onUpdateRole}
            onArchiveRole={onArchiveRole}
            onDeleteRole={onDeleteRole}
          />
        )}
      </div>
    </div>
  );
}
