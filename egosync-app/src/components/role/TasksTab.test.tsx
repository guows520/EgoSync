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
    deadline: '2026-06-30T10:00:00Z',
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
    expect(screen.getByText('06-30 10:00')).toBeInTheDocument();
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

  it('顶部使用标题新增任务与筛选卡片的上下结构', () => {
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

    expect(screen.getByRole('heading', { name: '任务清单' })).toBeInTheDocument();
    expect(screen.getByText('管理当前角色的任务')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '新增任务' })).toHaveClass('shrink-0');
    expect(screen.getByRole('group', { name: '按象限筛选' })).toBeInTheDocument();
  });

  it('顶部象限筛选只显示选中的象限，点全部恢复全量展示', () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'Q2' }));
    expect(screen.queryByText('准备季度规划')).not.toBeInTheDocument();
    expect(screen.getByText('整理会议纪要')).toBeInTheDocument();
    expect(screen.queryByText('Q1 · 重要且紧急')).not.toBeInTheDocument();
    expect(screen.getByText('Q2 · 重要不紧急')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '拖动排序 整理会议纪要' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '全部' }));
    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '拖动排序 准备季度规划' })).toBeInTheDocument();
  });

  it('只看大石头筛选可与象限筛选叠加', () => {
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

    fireEvent.click(screen.getByRole('button', { name: '只看大石头' }));
    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
    expect(screen.queryByText('整理会议纪要')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '拖动排序 准备季度规划' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Q2' }));
    expect(screen.queryByText('准备季度规划')).not.toBeInTheDocument();
    expect(screen.getByText('当前筛选无匹配任务')).toBeInTheDocument();
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

  it('protectionStatus 为 at_risk 的 Q2 任务渲染「被挤压」预警，normal 任务不渲染', () => {
    const atRiskTask: Task = {
      ...tasks[1],
      id: 'task-at-risk',
      title: '被持续挤压的重要任务',
      quadrant: 'Q2',
      protectionStatus: 'at_risk',
    };
    const normalTask: Task = {
      ...tasks[1],
      id: 'task-normal',
      title: '正常的 Q2 任务',
      quadrant: 'Q2',
      protectionStatus: 'normal',
    };

    render(
      <TasksTab
        role={role}
        tasks={[atRiskTask, normalTask]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    const badges = screen.getAllByLabelText('重要任务被持续挤压，建议尽快处理');
    expect(badges).toHaveLength(1);
    expect(screen.getByText('被挤压')).toBeInTheDocument();

    // AC3: at_risk 卡片渲染琥珀左竖线（border-l-4 border-l-amber-400），normal 卡片不渲染
    const atRiskCard = screen.getByText('被持续挤压的重要任务').closest('[class*="rounded-xl"]');
    const normalCard = screen.getByText('正常的 Q2 任务').closest('[class*="rounded-xl"]');
    expect(atRiskCard?.className).toContain('border-l-amber-400');
    expect(normalCard?.className).not.toContain('border-l-amber-400');
  });

  it('AC1：四象限标题按象限配色（Q1 红 / Q2 蓝 / Q3 灰 / Q4 淡灰）', () => {
    const oneEach: Task[] = (['Q1', 'Q2', 'Q3', 'Q4'] as const).map((quadrant, i) => ({
      ...tasks[0],
      id: `q-${quadrant}`,
      title: `${quadrant} 任务`,
      quadrant,
      isBigRock: false,
      deadline: null,
      sortOrder: i,
    }));

    render(
      <TasksTab
        role={role}
        tasks={oneEach}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.getByText('Q1 · 重要且紧急').className).toContain('text-slate-900');
    expect(screen.getByText('Q2 · 重要不紧急').className).toContain('text-slate-900');
    expect(screen.getByText('Q3 · 紧急不重要').className).toContain('text-slate-900');
    expect(screen.getByText('Q4 · 不重要不紧急').className).toContain('text-slate-900');
  });

  it('AC2：每组标题右侧显示任务数 badge（含已完成）', () => {
    // Q1 含 1 未完成 + 1 已完成 = 2；Q2 含 1 未完成 = 1
    render(
      <TasksTab
        role={role}
        tasks={[tasks[0], tasks[1], completedTask]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    const q1Header = screen.getByText('Q1 · 重要且紧急').closest('div');
    expect(q1Header).toHaveTextContent('2');
    const q2Header = screen.getByText('Q2 · 重要不紧急').closest('div');
    expect(q2Header).toHaveTextContent('1');
  });

  it('AC3：空象限显示鼓励文案且无任务卡片，非空象限不显示鼓励文案', () => {
    // 仅 1 条 Q1 任务 → Q2/Q3/Q4 为空
    render(
      <TasksTab
        role={role}
        tasks={[tasks[0]]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.getByText('暂无重要规划，别忘了为长远目标留出时间')).toBeInTheDocument();
    expect(screen.getByText('没有需要应付的杂事，很清爽')).toBeInTheDocument();
    expect(screen.getByText('没有可有可无的任务，注意力很集中')).toBeInTheDocument();
    // Q1 非空，不显示鼓励文案
    expect(screen.queryByText('没有紧急任务，太棒了！')).not.toBeInTheDocument();
    expect(screen.getByText('Q2 · 重要不紧急')).toBeInTheDocument();
  });

  it('AC3 边界：完全无任务时仅显示全局空态，不显示任何象限鼓励文案', () => {
    render(
      <TasksTab role={role} tasks={[]} isLoading={false} error={null} onOpenTask={vi.fn()} onDeleteTask={vi.fn()} onReorderTasks={noop} onToggleComplete={noop} />,
    );

    expect(screen.getByText('还没有任务，先添加一个小目标吧')).toBeInTheDocument();
    expect(screen.queryByText('没有紧急任务，太棒了！')).not.toBeInTheDocument();
    expect(screen.queryByText('暂无重要规划，别忘了为长远目标留出时间')).not.toBeInTheDocument();
    expect(screen.queryByText('没有需要应付的杂事，很清爽')).not.toBeInTheDocument();
    expect(screen.queryByText('没有可有可无的任务，注意力很集中')).not.toBeInTheDocument();
  });

  it('象限组标题为静态展示，不再作为折叠按钮', () => {
    render(
      <TasksTab
        role={role}
        tasks={[tasks[0]]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    expect(screen.getByText('Q1 · 重要且紧急')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Q1 · 重要且紧急/ })).not.toBeInTheDocument();
    expect(screen.getByText('准备季度规划')).toBeInTheDocument();
  });

  it('AC3：空象限标题右侧显示 (0) 计数 badge', () => {
    // 仅 1 条 Q1 任务 → Q2/Q3/Q4 为空
    render(
      <TasksTab
        role={role}
        tasks={[tasks[0]]}
        isLoading={false}
        error={null}
        onOpenTask={vi.fn()}
        onDeleteTask={vi.fn()}
        onReorderTasks={vi.fn()}
        onToggleComplete={vi.fn()}
      />,
    );

    const q2Header = screen.getByText('Q2 · 重要不紧急').closest('div');
    expect(q2Header).toHaveTextContent('0');
  });
});
