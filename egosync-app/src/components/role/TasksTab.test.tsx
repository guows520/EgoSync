import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TasksTab } from './TasksTab';
import type { Task } from '../../types/task';

const tasks: Task[] = [
  {
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
  },
  {
    id: 'task-2',
    ownerType: 'role',
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
    manualOverride: false,
    classificationReason: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    deletedAt: null,
  },
];

const completedTask: Task = {
  id: 'task-3',
  ownerType: 'role',
  roleId: 'role-1',
  title: '已完成的任务',
  deadline: null,
  quadrant: 'Q1',
  isBigRock: false,
  isCompleted: true,
  completedAt: '2026-06-02T00:00:00Z',
  sortOrder: 2,
  protectionStatus: 'normal',
  confidence: null,
  manualOverride: false,
  classificationReason: null,
  createdAt: '2026-06-01T00:00:00Z',
  updatedAt: '2026-06-02T00:00:00Z',
  deletedAt: null,
};

const role = { color: '#4F46E5' };

const noop = () => {};

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
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
    expect(screen.getByText('整理会议纪要')).toBeInTheDocument();
    expect(screen.getByText('2026-06-30')).toBeInTheDocument();
    expect(screen.getByText('大石头')).toBeInTheDocument();
    expect(screen.queryByText('暂无任务')).not.toBeInTheDocument();
  });

  it('classifyingIds 中的任务显示「智能分类中…」徽章，其它任务不显示', () => {
    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        classifyingIds={new Set(['task-2'])}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    const badges = screen.getAllByLabelText('正在智能分类');
    expect(badges).toHaveLength(1);
    expect(screen.getByText('智能分类中…')).toBeInTheDocument();
  });

  it('每个未完成任务卡片渲染拖拽手柄', () => {
    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: '拖动排序 准备季度规划' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '拖动排序 整理会议纪要' })).toBeInTheDocument();
  });

  it('点击完成圆圈触发 onToggleComplete 并传入取反状态', () => {
    const onToggleComplete = vi.fn();

    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={onToggleComplete}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '完成 准备季度规划' }));
    expect(onToggleComplete).toHaveBeenCalledWith('task-1', true);
  });

  it('点击拖拽手柄不会触发卡片编辑', () => {
    const onOpenTask = vi.fn();

    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={onOpenTask}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '拖动排序 准备季度规划' }));
    expect(onOpenTask).not.toHaveBeenCalled();
  });

  it('点击任务卡片空白区域不会触发编辑，仅编辑按钮触发', () => {
    const onOpenTask = vi.fn();

    const { container } = render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={onOpenTask}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    // 点击卡片容器（非按钮区域）不应触发编辑
    const card = container.querySelector('[class*="rounded-xl"]');
    if (card) fireEvent.click(card);
    expect(onOpenTask).not.toHaveBeenCalled();

    // 仅编辑按钮触发
    fireEvent.click(screen.getByRole('button', { name: '编辑 准备季度规划' }));
    expect(onOpenTask).toHaveBeenCalledWith(tasks[0]);
  });

  it('已完成任务折叠展示、展开后灰显并提供撤销完成入口', () => {
    const onToggleComplete = vi.fn();
    render(
      <TasksTab
        role={role}
        tasks={[completedTask, tasks[0]]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={onToggleComplete}
      />,
    );

    // 未完成任务始终可见
    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
    // 已完成任务默认折叠，仅显示摘要
    const expandBtn = screen.getByRole('button', { name: /已完成 \(1\)/ });
    expect(expandBtn).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('已完成的任务')).not.toBeInTheDocument();

    // 展开后显示已完成卡片 + 撤销入口
    fireEvent.click(expandBtn);
    expect(expandBtn).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('已完成的任务')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '撤销完成 已完成的任务' }));
    expect(onToggleComplete).toHaveBeenCalledWith('task-3', false);
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
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '新增任务' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 准备季度规划' }));
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
      <TasksTab role={role} tasks={[]} isLoading error={null} onOpenTask={vi.fn()} onDeleteTask={vi.fn()} onReorderTasks={noop} onToggleComplete={noop} />,
    );
    expect(screen.getByText('任务加载中...')).toBeInTheDocument();

    rerender(<TasksTab role={role} tasks={[]} isLoading={false} error="任务暂时加载失败，请稍后再试" onOpenTask={vi.fn()} onDeleteTask={vi.fn()} onReorderTasks={noop} onToggleComplete={noop} />);
    expect(screen.getByText('任务暂时加载失败，请稍后再试')).toBeInTheDocument();

    rerender(<TasksTab role={role} tasks={[]} isLoading={false} error={null} onOpenTask={vi.fn()} onDeleteTask={vi.fn()} onReorderTasks={noop} onToggleComplete={noop} />);
    expect(screen.getByText('还没有任务，先添加一个小目标吧')).toBeInTheDocument();
  });

  it('低置信度的自动分类任务不再显示「不确定」徽章', () => {
    const uncertainTask: Task = {
      ...tasks[0],
      id: 'task-uncertain',
      title: '不确定的任务',
      manualOverride: false,
      confidence: 0.4,
      classificationReason: 'LLM 暂时不可用，先放入 Q2',
    };

    render(
      <TasksTab
        role={role}
        tasks={[uncertainTask]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.queryByText('? 不确定')).not.toBeInTheDocument();
  });

  it('toggleComplete 失败时显示内联中文错误文案', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const onToggleComplete = vi.fn().mockRejectedValue(new Error('boom'));

    render(
      <TasksTab
        role={role}
        tasks={tasks}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={onToggleComplete}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '完成 准备季度规划' }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('任务状态暂时切换失败，请稍后再试');
    });

    consoleSpy.mockRestore();
  });

  it('大石头任务在同象限内排在非大石头任务之前', () => {
    const q2Tasks: Task[] = [
      {
        id: 'br-non-rock',
        ownerType: 'role',
        roleId: 'role-1',
        title: '非大石头任务',
        deadline: null,
        quadrant: 'Q2',
        isBigRock: false,
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
      },
      {
        id: 'br-rock',
        ownerType: 'role',
        roleId: 'role-1',
        title: '大石头任务',
        deadline: null,
        quadrant: 'Q2',
        isBigRock: true,
        isCompleted: false,
        completedAt: null,
        sortOrder: 1,
        protectionStatus: 'normal',
        confidence: null,
        manualOverride: false,
        classificationReason: null,
        createdAt: '2026-06-01T00:00:00Z',
        updatedAt: '2026-06-01T00:00:00Z',
        deletedAt: null,
      },
    ];

    render(
      <TasksTab
        role={role}
        tasks={q2Tasks}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    const editButtons = screen.getAllByRole('button', { name: /编辑/ });
    expect(editButtons[0]).toHaveAccessibleName('编辑 大石头任务');
    expect(editButtons[1]).toHaveAccessibleName('编辑 非大石头任务');
  });
});
