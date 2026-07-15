import { useCallback, useEffect, useState } from 'react';
import { taskService } from '../services/taskService';
import type { AllTasksFilter, CreateTaskInput, CrossRoleTask, Task, UpdateTaskInput } from '../types/task';
import { useTauriEvent } from './useTauriEvent';

const TASK_LOAD_ERROR = '任务暂时加载失败，请稍后再试';
const TASK_CLASSIFIED_EVENT = 'task:classified';
const TASK_TOOL_ACTION_EVENT = 'task:tool-action';

function filterKey(filter: AllTasksFilter) {
  return `${filter.quadrant ?? 'all'}:${filter.isBigRock === undefined ? 'all' : String(filter.isBigRock)}`;
}

function matchesFilter(task: CrossRoleTask, filter: AllTasksFilter) {
  if (filter.quadrant && task.quadrant !== filter.quadrant) return false;
  if (filter.isBigRock !== undefined && task.isBigRock !== filter.isBigRock) return false;
  return true;
}

function toCrossRoleTask(task: Task): CrossRoleTask {
  return {
    ...task,
    roleName: null,
    roleColor: null,
  };
}

export function useAllTasks(filter: AllTasksFilter = {}) {
  const stableFilterKey = filterKey(filter);
  const [tasks, setTasks] = useState<CrossRoleTask[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [classifyingIds, setClassifyingIds] = useState<Set<string>>(() => new Set());

  useTauriEvent<Task>(
    TASK_CLASSIFIED_EVENT,
    classified => {
      const crossRoleTask = toCrossRoleTask(classified);
      setTasks(prev => {
        if (!matchesFilter(crossRoleTask, filter)) return prev.filter(task => task.id !== classified.id);
        return prev.map(task => (task.id === classified.id ? { ...crossRoleTask, roleName: task.roleName, roleColor: task.roleColor } : task));
      });
      setClassifyingIds(prev => {
        if (!prev.has(classified.id)) return prev;
        const next = new Set(prev);
        next.delete(classified.id);
        return next;
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [stableFilterKey],
  );

  // 监听任务操作工具事件（complete_task / delete_task），刷新任务列表
  useTauriEvent<{ action: string }>(
    TASK_TOOL_ACTION_EVENT,
    () => {
      setReloadKey(key => key + 1);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const refetch = useCallback(() => {
    setReloadKey(key => key + 1);
  }, []);

  const reloadCurrent = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const items = await taskService.listAll(filter);
      setTasks(items);
    } catch (e) {
      console.error('加载全量任务失败:', e);
      setTasks([]);
      setError(TASK_LOAD_ERROR);
    } finally {
      setIsLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stableFilterKey]);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    const load = async () => {
      try {
        await taskService.checkProtectionStatus();
      } catch (e) {
        console.warn('Q2 保护状态检查失败（忽略，不影响任务加载）:', e);
      }
      try {
        const items = await taskService.listAll(filter);
        if (!cancelled) setTasks(items);
      } catch (e) {
        console.error('加载全量任务失败:', e);
        if (!cancelled) {
          setTasks([]);
          setError(TASK_LOAD_ERROR);
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };
    void load();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stableFilterKey, reloadKey]);

  const createTask = useCallback(async (input: CreateTaskInput) => {
    const created = await taskService.create(input);
    if (input.quadrant === undefined) {
      setClassifyingIds(prev => {
        const next = new Set(prev);
        next.add(created.id);
        return next;
      });
    }
    await reloadCurrent();
  }, [reloadCurrent]);

  const updateTask = useCallback(async (id: string, input: UpdateTaskInput) => {
    await taskService.update(id, input);
    await reloadCurrent();
  }, [reloadCurrent]);

  const deleteTask = useCallback(async (id: string) => {
    await taskService.delete(id);
    await reloadCurrent();
  }, [reloadCurrent]);

  const toggleComplete = useCallback(async (id: string, isCompleted: boolean) => {
    let snapshot: CrossRoleTask[] = [];
    setTasks(prev => {
      snapshot = prev;
      return prev.map(task =>
        task.id === id
          ? {
              ...task,
              isCompleted,
              completedAt: isCompleted ? new Date().toISOString() : null,
              isBigRock: isCompleted ? false : task.isBigRock,
            }
          : task,
      );
    });
    try {
      await taskService.toggleComplete(id, isCompleted);
      await reloadCurrent();
    } catch (e) {
      console.error('切换任务完成状态失败:', e);
      setTasks(snapshot);
      await reloadCurrent();
      throw e;
    }
  }, [reloadCurrent]);

  return {
    tasks,
    isLoading,
    error,
    classifyingIds,
    refetch,
    createTask,
    updateTask,
    deleteTask,
    toggleComplete,
  };
}
