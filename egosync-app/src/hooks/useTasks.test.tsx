import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useTasks } from './useTasks';
import { taskService } from '../services/taskService';
import type { Task } from '../types/task';

vi.mock('../services/taskService', () => ({
  taskService: {
    listByRole: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
  },
}));

const task: Task = {
  id: 'task-1',
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

    const { result } = renderHook(() => useTasks('role-1'));

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

    const { result } = renderHook(() => useTasks('role-1'));
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
});
