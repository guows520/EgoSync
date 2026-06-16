import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TasksTab } from './TasksTab';
import type { Task } from '../../types/task';

const tasks: Task[] = [
  {
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
  },
  {
    id: 'task-2',
    roleId: 'role-1',
    title: '整理会议纪要',
    deadline: null,
    quadrant: 'Q2',
    isBigRock: false,
    isCompleted: false,
    completedAt: null,
    sortOrder: 1,
    protectionStatus: 'normal',
    confidence: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    deletedAt: null,
  },
];

const role = { color: '#4F46E5' };

describe('TasksTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('展示传入的真实任务并保留大石头与截止时间标记', () => {
    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
      />,
    );

    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
    expect(screen.getByText('整理会议纪要')).toBeInTheDocument();
    expect(screen.getByText('2026-06-30')).toBeInTheDocument();
    expect(screen.getByText('大石头')).toBeInTheDocument();
    expect(screen.queryByText('暂无任务')).not.toBeInTheDocument();
  });

  it('新增、点击卡片编辑、删除确认通过回调交给上层数据链路', async () => {
    const onOpenTask = vi.fn();
    const onDeleteTask = vi.fn();

    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={onOpenTask}
        onDeleteTask={onDeleteTask}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '新增任务' }));
    fireEvent.click(screen.getByRole('button', { name: '打开编辑 准备季度规划' }));
    fireEvent.click(screen.getByRole('button', { name: '删除 准备季度规划' }));

    expect(onOpenTask).toHaveBeenNthCalledWith(1, null);
    expect(onOpenTask).toHaveBeenNthCalledWith(2, tasks[0]);
    expect(onDeleteTask).not.toHaveBeenCalled();
    expect(screen.getByRole('heading', { name: '确认删除任务？' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));
    await waitFor(() => {
      expect(onDeleteTask).toHaveBeenCalledWith('task-1');
    });
  });

  it('加载、错误、空态使用真实数据状态', () => {
    const { rerender } = render(
      <TasksTab role={role} tasks={[]} isLoading error={null} onOpenTask={vi.fn()} onDeleteTask={vi.fn()} />,
    );
    expect(screen.getByText('任务加载中...')).toBeInTheDocument();

    rerender(<TasksTab role={role} tasks={[]} isLoading={false} error="任务暂时加载失败，请稍后再试" onOpenTask={vi.fn()} onDeleteTask={vi.fn()} />);
    expect(screen.getByText('任务暂时加载失败，请稍后再试')).toBeInTheDocument();

    rerender(<TasksTab role={role} tasks={[]} isLoading={false} error={null} onOpenTask={vi.fn()} onDeleteTask={vi.fn()} />);
    expect(screen.getByText('还没有任务，先添加一个小目标吧')).toBeInTheDocument();
  });
});
