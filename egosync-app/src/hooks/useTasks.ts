import { useCallback, useEffect, useState } from 'react';
import { taskService } from '../services/taskService';
import type { CreateTaskInput, Task, UpdateTaskInput } from '../types/task';
import { useTauriEvent } from './useTauriEvent';

const TASK_LOAD_ERROR = '任务暂时加载失败，请稍后再试';

/** 后端任务自动分类完成（或降级）时推送的事件名，需与 commands/task.rs 的 TASK_CLASSIFIED_EVENT 保持一致。 */
const TASK_CLASSIFIED_EVENT = 'task:classified';
const TASK_TOOL_ACTION_EVENT = 'task:tool-action';

export type TaskScope =
  | { ownerType: 'role'; roleId: string }
  | { ownerType: 'butler' };

export function useTasks(scope: TaskScope | null) {
  // scope 为对象，调用方每次 render 新建引用。若直接作为 effect/callback 依赖会因
  // 引用每次变化导致无限重载，故归约成稳定的原始字符串 key 作为依赖（等价于值比较）。
  const scopeKey = scope
    ? scope.ownerType === 'role'
      ? `role:${scope.roleId}`
      : 'butler'
    : null;

  const [tasks, setTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  // 新建任务后后端在后台异步分类，这些任务 id 在收到 task:classified 事件前显示「分类中」过渡态。
  const [classifyingIds, setClassifyingIds] = useState<Set<string>>(() => new Set());

  // 监听后端推送的分类完成事件：用最新任务替换卡片并清除「分类中」标记。
  useTauriEvent<Task>(
    TASK_CLASSIFIED_EVENT,
    classified => {
      if (!scope) return;
      if (scope.ownerType === 'role' && classified.roleId !== scope.roleId) return;
      if (scope.ownerType === 'butler' && classified.ownerType !== 'butler') return;
      setTasks(prev => prev.map(task => (task.id === classified.id ? classified : task)));
      setClassifyingIds(prev => {
        if (!prev.has(classified.id)) return prev;
        const next = new Set(prev);
        next.delete(classified.id);
        return next;
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [scopeKey],
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

  useEffect(() => {
    if (!scope) {
      setTasks([]);
      setIsLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    const load = async () => {
      // Story 3.5: 打开任务面板时先触发一次 Q2 保护检查，使 at_risk 状态即时反映。
      // 容错：失败仅 console.warn，绝不阻塞后续列表加载。
      try {
        await taskService.checkProtectionStatus();
      } catch (e) {
        console.warn('Q2 保护状态检查失败（忽略，不影响任务加载）:', e);
      }
      try {
        const items = scope.ownerType === 'role'
          ? await taskService.listByRole(scope.roleId)
          : await taskService.listButler();
        if (!cancelled) setTasks(items);
      } catch (e) {
        console.error('加载任务失败:', e);
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
    // 依赖 scopeKey（稳定原始值）而非 scope（每次新建的对象引用），避免无限重载。
    // scopeKey 唯一决定 scope 内容，故闭包内读取的 scope 不存在 stale 风险。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scopeKey, reloadKey]);

  const reloadCurrent = useCallback(async () => {
    if (!scope) return;
    setIsLoading(true);
    setError(null);
    try {
      const items = scope.ownerType === 'role'
        ? await taskService.listByRole(scope.roleId)
        : await taskService.listButler();
      setTasks(items);
    } catch (e) {
      console.error('加载任务失败:', e);
      setTasks([]);
      setError(TASK_LOAD_ERROR);
    } finally {
      setIsLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scopeKey]);

  const createTask = useCallback(async (input: CreateTaskInput) => {
    const created = await taskService.create(input);
    // 未显式指定 quadrant 时，后端在后台异步分类；先标记「分类中」，收到 task:classified 事件后清除。
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

  const reorderTasks = useCallback(async (taskIds: string[]) => {
    let snapshot: Task[] = [];
    setTasks(prev => {
      snapshot = prev;
      const byId = new Map(prev.map(task => [task.id, task]));
      const reordered = taskIds
        .map((id, index) => {
          const task = byId.get(id);
          return task ? { ...task, sortOrder: index } : null;
        })
        .filter((task): task is Task => task !== null);
      const missing = prev.filter(task => !taskIds.includes(task.id));
      return [...reordered, ...missing];
    });
    try {
      await taskService.reorder(taskIds);
    } catch (e) {
      console.error('任务排序失败:', e);
      setTasks(snapshot);
      await reloadCurrent();
      throw e;
    }
  }, [reloadCurrent]);

  const toggleComplete = useCallback(async (id: string, isCompleted: boolean) => {
    let snapshot: Task[] = [];
    setTasks(prev => {
      snapshot = prev;
      return prev.map(task =>
        task.id === id
          ? {
              ...task,
              isCompleted,
              completedAt: isCompleted ? new Date().toISOString() : null,
              // 方案 D：完成时后端会自动撤销大石头标记，前端乐观同步让「大石头」标签即时消失。
              isBigRock: isCompleted ? false : task.isBigRock,
            }
          : task,
      );
    });
    try {
      const updated = await taskService.toggleComplete(id, isCompleted);
      setTasks(prev => prev.map(task => (task.id === id ? updated : task)));
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
    reorderTasks,
    toggleComplete,
  };
}
