import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useTasks } from './useTasks';
import { taskService } from '../services/taskService';
import type { Task } from '../types/task';

vi.mock('../services/taskService', () => ({
  taskService: {
    listByRole: vi.fn(),
    listButler: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    reorder: vi.fn(),
    toggleComplete: vi.fn(),
  },
}));

const task: Task = {
  id: 'task-1',
  ownerType: 'role',
  roleId: 'role-1',
  title: '准备季度规划',
  deadline: '2026-06-30',
  quadrant: 'Q1',
  isBigRock: true,
  isCompleted: false,
  completedAt: null,
  sortOrder: 0,
  protectionStatus: 'normal',
  confidence: null,
  manualOverride: false,
  classificationReason: null,
  createdAt: '2026-06-01T00:00:00Z',
  updatedAt: '2026-06-01T00:00:00Z',
  deletedAt: null,
};

describe('useTasks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('按角色加载真实任务并暴露刷新状态', async () => {
    vi.mocked(taskService.listByRole).mockResolvedValue([task]);

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));

    expect(result.current.isLoading).toBe(true);
    await waitFor(() => expect(taskService.listByRole).toHaveBeenCalledWith('role-1'));
    expect(result.current.tasks).toEqual([task]);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('创建、更新、删除后重新加载当前角色任务', async () => {
    vi.mocked(taskService.listByRole).mockResolvedValue([task]);
    vi.mocked(taskService.create).mockResolvedValue(task);
    vi.mocked(taskService.update).mockResolvedValue({ ...task, title: '更新季度规划' });
    vi.mocked(taskService.delete).mockResolvedValue(undefined);

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.createTask({ roleId: 'role-1', title: '准备季度规划', quadrant: 'Q1' });
      await result.current.updateTask('task-1', { title: '更新季度规划' });
      await result.current.deleteTask('task-1');
    });

    expect(taskService.create).toHaveBeenCalledWith({ roleId: 'role-1', title: '准备季度规划', quadrant: 'Q1' });
    expect(taskService.update).toHaveBeenCalledWith('task-1', { title: '更新季度规划' });
    expect(taskService.delete).toHaveBeenCalledWith('task-1');
    expect(taskService.listByRole).toHaveBeenCalledTimes(4);
  });

  it('reorderTasks 乐观重排本地顺序并传入正确 id 顺序', async () => {
    const second: Task = { ...task, id: 'task-2', title: '阅读论文', sortOrder: 1 };
    vi.mocked(taskService.listByRole).mockResolvedValue([task, second]);
    vi.mocked(taskService.reorder).mockResolvedValue(undefined);

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.reorderTasks(['task-2', 'task-1']);
    });

    expect(taskService.reorder).toHaveBeenCalledWith(['task-2', 'task-1']);
    expect(result.current.tasks.map(t => t.id)).toEqual(['task-2', 'task-1']);
    expect(result.current.tasks.map(t => t.sortOrder)).toEqual([0, 1]);
  });

  it('reorderTasks 失败时回滚本地顺序并重新加载', async () => {
    const second: Task = { ...task, id: 'task-2', title: '阅读论文', sortOrder: 1 };
    vi.mocked(taskService.listByRole).mockResolvedValue([task, second]);
    vi.mocked(taskService.reorder).mockRejectedValueOnce(new Error('boom'));

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await expect(result.current.reorderTasks(['task-2', 'task-1'])).rejects.toThrow('boom');
    });

    // 回滚到原顺序，并触发 reloadCurrentRole 重新拉取
    expect(result.current.tasks.map(t => t.id)).toEqual(['task-1', 'task-2']);
    expect(taskService.listByRole).toHaveBeenCalledTimes(2);
  });

  it('toggleComplete 乐观更新完成态并用后端返回值落库', async () => {
    vi.mocked(taskService.listByRole).mockResolvedValue([task]);
    const completed: Task = { ...task, isCompleted: true, completedAt: '2026-06-10T00:00:00Z' };
    vi.mocked(taskService.toggleComplete).mockResolvedValue(completed);

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.toggleComplete('task-1', true);
    });

    expect(taskService.toggleComplete).toHaveBeenCalledWith('task-1', true);
    expect(result.current.tasks[0].isCompleted).toBe(true);
    expect(result.current.tasks[0].completedAt).toBe('2026-06-10T00:00:00Z');
  });

  it('toggleComplete 失败时回滚完成态并重新加载', async () => {
    vi.mocked(taskService.listByRole).mockResolvedValue([task]);
    vi.mocked(taskService.toggleComplete).mockRejectedValueOnce(new Error('boom'));

    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await expect(result.current.toggleComplete('task-1', true)).rejects.toThrow('boom');
    });

    expect(result.current.tasks[0].isCompleted).toBe(false);
    expect(taskService.listByRole).toHaveBeenCalledTimes(2);
  });

  it('scope 内容不变但每次传入新对象引用时不应重复加载（回归：任务一直加载中）', async () => {
    vi.mocked(taskService.listByRole).mockResolvedValue([task]);

    // 模拟调用方每次 render 内联新建 scope 对象（引用每次不同，内容相同）。
    const { result, rerender } = renderHook(() =>
      useTasks({ ownerType: 'role', roleId: 'role-1' }),
    );
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    rerender();
    rerender();
    rerender();

    // 依赖归约为稳定 scopeKey 后，重复 render 不应重新发起加载。
    expect(taskService.listByRole).toHaveBeenCalledTimes(1);
    expect(result.current.isLoading).toBe(false);
  });

  it('butler scope 调用 listButler 加载管家任务', async () => {
    const butlerTask: Task = { ...task, id: 'b-1', ownerType: 'butler', roleId: null };
    vi.mocked(taskService.listButler).mockResolvedValue([butlerTask]);

    const { result } = renderHook(() => useTasks({ ownerType: 'butler' }));

    await waitFor(() => expect(taskService.listButler).toHaveBeenCalledTimes(1));
    expect(taskService.listByRole).not.toHaveBeenCalled();
    expect(result.current.tasks).toEqual([butlerTask]);
  });
});
