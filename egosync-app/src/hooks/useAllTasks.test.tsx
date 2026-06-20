import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAllTasks } from './useAllTasks';
import { taskService } from '../services/taskService';
import type { CrossRoleTask, Task } from '../types/task';

vi.mock('../services/taskService', () => ({
  taskService: {
    listAll: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    toggleComplete: vi.fn(),
    checkProtectionStatus: vi.fn(),
  },
}));

vi.mock('./useTauriEvent', () => ({
  useTauriEvent: vi.fn(),
}));

const crossRoleTask: CrossRoleTask = {
  id: 'task-1',
  ownerType: 'role',
  roleId: 'role-1',
  roleName: '产品',
  roleColor: '#4F46E5',
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

const baseTask: Task = {
  ...crossRoleTask,
};

describe('useAllTasks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(taskService.checkProtectionStatus).mockResolvedValue(0);
  });

  it('加载全量任务并传递筛选条件', async () => {
    vi.mocked(taskService.listAll).mockResolvedValue([crossRoleTask]);

    const { result } = renderHook(() => useAllTasks({ quadrant: 'Q1', isBigRock: true }));

    expect(result.current.isLoading).toBe(true);
    await waitFor(() => expect(taskService.listAll).toHaveBeenCalledWith({ quadrant: 'Q1', isBigRock: true }));
    expect(result.current.tasks).toEqual([crossRoleTask]);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('创建、更新、删除后重新加载全量任务', async () => {
    vi.mocked(taskService.listAll).mockResolvedValue([crossRoleTask]);
    vi.mocked(taskService.create).mockResolvedValue(baseTask);
    vi.mocked(taskService.update).mockResolvedValue({ ...baseTask, title: '更新季度规划' });
    vi.mocked(taskService.delete).mockResolvedValue(undefined);

    const { result } = renderHook(() => useAllTasks());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.createTask({ ownerType: 'role', roleId: 'role-1', title: '准备季度规划' });
      await result.current.updateTask('task-1', { title: '更新季度规划' });
      await result.current.deleteTask('task-1');
    });

    expect(taskService.create).toHaveBeenCalledWith({ ownerType: 'role', roleId: 'role-1', title: '准备季度规划' });
    expect(taskService.update).toHaveBeenCalledWith('task-1', { title: '更新季度规划' });
    expect(taskService.delete).toHaveBeenCalledWith('task-1');
    expect(taskService.listAll).toHaveBeenCalledTimes(4);
  });

  it('toggleComplete 乐观更新后重新加载全量任务', async () => {
    vi.mocked(taskService.listAll).mockResolvedValue([crossRoleTask]);
    vi.mocked(taskService.toggleComplete).mockResolvedValue({ ...baseTask, isCompleted: true });

    const { result } = renderHook(() => useAllTasks());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.toggleComplete('task-1', true);
    });

    expect(taskService.toggleComplete).toHaveBeenCalledWith('task-1', true);
    expect(taskService.listAll).toHaveBeenCalledTimes(2);
  });

  it('更新失败（如大石头超限）时不刷新列表并向上抛出错误', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(taskService.listAll).mockResolvedValue([crossRoleTask]);
    vi.mocked(taskService.update).mockRejectedValue({
      ValidationError: '每个任务清单每周最多 3 个大石头，请先取消一个再标记',
    });

    const { result } = renderHook(() => useAllTasks());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(taskService.listAll).toHaveBeenCalledTimes(1);

    await expect(
      act(async () => {
        await result.current.updateTask('task-1', { isBigRock: true });
      }),
    ).rejects.toMatchObject({ ValidationError: expect.any(String) });

    // 更新失败时不应触发重新加载（refetch 未被调用）
    expect(taskService.listAll).toHaveBeenCalledTimes(1);

    errorSpy.mockRestore();
  });

  it('保护检查失败不影响全量任务列表加载', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.mocked(taskService.checkProtectionStatus).mockRejectedValueOnce(new Error('boom'));
    vi.mocked(taskService.listAll).mockResolvedValue([crossRoleTask]);

    const { result } = renderHook(() => useAllTasks());

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(taskService.checkProtectionStatus).toHaveBeenCalledTimes(1);
    expect(taskService.listAll).toHaveBeenCalledWith({});
    expect(result.current.tasks).toEqual([crossRoleTask]);
    expect(result.current.error).toBeNull();

    warnSpy.mockRestore();
  });
});
