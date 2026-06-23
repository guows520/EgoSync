import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { DashboardTab } from './DashboardTab';
import { MemoryTab } from '../role/MemoryTab';
import { ButlerSettingsContent } from './ButlerSettingsContent';
import { TaskOverviewTab } from './TaskOverviewTab';
import { memoryService } from '../../services/memoryService';
import { useAllTasks } from '../../hooks/useAllTasks';
import type { MemoryCategory } from '../../types/memory';
import type { AllTasksFilter, Task, TaskActions, TaskQuadrant } from '../../types/task';
import type { TaskScope } from '../../hooks/useTasks';

export function ButlerWorkspacePanel({
  roles,
  currentTab,
  setTab,
  archivedRoles,
  onRestoreRole,
  onUpdateRole,
  onViewChange,
  targetMemoryId,
  onTargetMemoryHandled,
  onSourceMessageClick,
  onOpenTask,
  onTasksApiReady,
}: {
  roles: any[];
  currentTab: 'dashboard' | 'tasks' | 'memory' | 'settings' | null;
  setTab: (tab: 'dashboard' | 'tasks' | 'memory' | 'settings' | null) => void;
  archivedRoles: any[];
  onRestoreRole: (id: string) => Promise<void> | void;
  onUpdateRole: (role: any) => void;
  onViewChange: (view: string) => void;
  targetMemoryId: string | null;
  onTargetMemoryHandled: () => void;
  onSourceMessageClick: (target: any) => void;
  onOpenTask: (scope: TaskScope, task?: Task | null) => void;
  onTasksApiReady: (key: string, actions: TaskActions) => void;
}) {
  const [memoryCount, setMemoryCount] = useState<number | null>(null);
  const [memoryCategory, setMemoryCategory] = useState<MemoryCategory | undefined>();
  const [memoryCountReloadKey, setMemoryCountReloadKey] = useState(0);

  const [quadrantFilter, setQuadrantFilter] = useState<TaskQuadrant | 'all'>('all');
  const [showBigRocksOnly, setShowBigRocksOnly] = useState(false);
  const allTasksFilter: AllTasksFilter = {
    ...(quadrantFilter === 'all' ? {} : { quadrant: quadrantFilter }),
    ...(showBigRocksOnly ? { isBigRock: true } : {}),
  };
  const { tasks, isLoading: isLoadingTasks, error: tasksError, classifyingIds, createTask, updateTask, deleteTask, toggleComplete } = useAllTasks(allTasksFilter);

  useEffect(() => {
    onTasksApiReady?.('butler', {
      createTask,
      updateTask,
      deleteTask,
      reorderTasks: async () => {},
      toggleComplete,
    });
  }, [createTask, deleteTask, onTasksApiReady, updateTask, toggleComplete]);

  const effectiveMemoryCategory = targetMemoryId ? undefined : memoryCategory;

  useEffect(() => {
    let cancelled = false;
    setMemoryCount(null);
    memoryService
      .count({ roleId: null, includeRoleMemories: true, category: effectiveMemoryCategory })
      .then(count => {
        if (!cancelled) setMemoryCount(count);
      })
      .catch(e => {
        console.error('加载管家记忆数量失败:', e);
        if (!cancelled) setMemoryCount(0);
      });

    return () => {
      cancelled = true;
    };
  }, [effectiveMemoryCategory, memoryCountReloadKey]);

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

  const roleLabels = Object.fromEntries((roles ?? []).map((role: any) => [role.id, role.name]));
  const memoryLabel = memoryCount === null ? '管家记忆' : `管家记忆 (${memoryCount})`;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b border-slate-200/80 px-6 pt-4 bg-white/60 backdrop-blur-md shrink-0">
        <div className="flex gap-6">
          <button onClick={() => setTab('dashboard')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'dashboard' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            仪表盘
          </button>
          <button onClick={() => setTab('tasks')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'tasks' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            任务概览
          </button>
          <button onClick={() => setTab('memory')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'memory' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            {memoryLabel}
          </button>
          <button onClick={() => setTab('settings')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'settings' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            管家设置
          </button>
        </div>
        <button onClick={() => setTab(null)} className="pb-3.5 text-slate-400 hover:text-slate-700 transition-colors">
          <X size={18} />
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-6 scroll-smooth">
        {currentTab === 'dashboard' && <DashboardTab onViewChange={onViewChange} />}
        {currentTab === 'tasks' && (
          <TaskOverviewTab
            roles={roles}
            tasks={tasks}
            isLoading={isLoadingTasks}
            error={tasksError}
            classifyingIds={classifyingIds}
            quadrantFilter={quadrantFilter}
            onQuadrantFilterChange={setQuadrantFilter}
            showBigRocksOnly={showBigRocksOnly}
            onToggleBigRocksOnly={() => setShowBigRocksOnly(value => !value)}
            onOpenTask={onOpenTask}
            onDeleteTask={deleteTask}
            onToggleComplete={toggleComplete}
          />
        )}
        {currentTab === 'memory' && (
          <MemoryTab
            roleId={null}
            includeRoleMemories
            showOwnerLabel
            roleLabels={roleLabels}
            category={effectiveMemoryCategory}
            onCategoryChange={handleMemoryCategoryChange}
            onMemoryDeleted={() => setMemoryCountReloadKey(key => key + 1)}
            targetMemoryId={targetMemoryId}
            onTargetMemoryHandled={onTargetMemoryHandled}
            onSourceMessageClick={onSourceMessageClick}
          />
        )}
        {currentTab === 'settings' && <ButlerSettingsContent activeRoles={roles} archivedRoles={archivedRoles} onRestoreRole={onRestoreRole} onUpdateRole={onUpdateRole} />}
      </div>
    </div>
  );
}
