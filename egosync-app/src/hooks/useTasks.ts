import { useCallback, useEffect, useState } from 'react';
import { taskService } from '../services/taskService';
import type { CreateTaskInput, Task, UpdateTaskInput } from '../types/task';

const TASK_LOAD_ERROR = '任务暂时加载失败，请稍后再试';

export function useTasks(roleId: string | null) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  const refetch = useCallback(() => {
    setReloadKey(key => key + 1);
  }, []);

  useEffect(() => {
    if (!roleId) {
      setTasks([]);
      setIsLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    taskService
      .listByRole(roleId)
      .then(items => {
        if (!cancelled) setTasks(items);
      })
      .catch(e => {
        console.error('加载任务失败:', e);
        if (!cancelled) {
          setTasks([]);
          setError(TASK_LOAD_ERROR);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [roleId, reloadKey]);

  const reloadCurrentRole = useCallback(async () => {
    if (!roleId) return;
    setIsLoading(true);
    setError(null);
    try {
      const items = await taskService.listByRole(roleId);
      setTasks(items);
    } catch (e) {
      console.error('加载任务失败:', e);
      setTasks([]);
      setError(TASK_LOAD_ERROR);
    } finally {
      setIsLoading(false);
    }
  }, [roleId]);

  const createTask = useCallback(async (input: CreateTaskInput) => {
    await taskService.create(input);
    await reloadCurrentRole();
  }, [reloadCurrentRole]);

  const updateTask = useCallback(async (id: string, input: UpdateTaskInput) => {
    await taskService.update(id, input);
    await reloadCurrentRole();
  }, [reloadCurrentRole]);

  const deleteTask = useCallback(async (id: string) => {
    await taskService.delete(id);
    await reloadCurrentRole();
  }, [reloadCurrentRole]);

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
      await reloadCurrentRole();
      throw e;
    }
  }, [reloadCurrentRole]);

  const toggleComplete = useCallback(async (id: string, isCompleted: boolean) => {
    let snapshot: Task[] = [];
    setTasks(prev => {
      snapshot = prev;
      return prev.map(task =>
        task.id === id
          ? { ...task, isCompleted, completedAt: isCompleted ? new Date().toISOString() : null }
          : task,
      );
    });
    try {
      const updated = await taskService.toggleComplete(id, isCompleted);
      setTasks(prev => prev.map(task => (task.id === id ? updated : task)));
    } catch (e) {
      console.error('切换任务完成状态失败:', e);
      setTasks(snapshot);
      await reloadCurrentRole();
      throw e;
    }
  }, [reloadCurrentRole]);

  return {
    tasks,
    isLoading,
    error,
    refetch,
    createTask,
    updateTask,
    deleteTask,
    reorderTasks,
    toggleComplete,
  };
}
