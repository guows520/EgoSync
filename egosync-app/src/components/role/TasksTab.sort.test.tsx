// Story 16.4：TasksTab 移动端排序模式（触屏拖拽降级）测试。
//
// 覆盖：
// - 象限分组头「排序」开关（md:hidden）切换排序模式；
// - 排序模式开：拖拽把手隐藏（dnd-kit disabled——useSortable({disabled})），
//   每卡渲染 ↑/↓ 按钮（≥44px 类名在组件测试层由 ChatInput 同款范式覆盖，
//   此处断言行为：首任务 ↑ 禁用、末任务 ↓ 禁用）；
// - ↑/↓ 复用 handleDragEnd 同款重排落库：onReorderTasks 收到全量序
//   （未完成按新序拼接 + 已完成尾随），与拖拽路径逐字节同语义；
// - 筛选视图（象限 chip）下排序入口不渲染（与拖拽守卫一致）。

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TasksTab } from './TasksTab';
import type { Task } from '../../types/task';

function makeTask(id: string, sortOrder: number, overrides: Partial<Task> = {}): Task {
  return {
    id,
    ownerType: 'role',
    roleId: 'role-1',
    title: `任务 ${id}`,
    deadline: null,
    quadrant: 'Q1',
    isBigRock: false,
    isCompleted: false,
    completedAt: null,
    sortOrder,
    protectionStatus: 'normal',
    confidence: null,
    manualOverride: false,
    classificationReason: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    deletedAt: null,
    ...overrides,
  };
}

const taskA = makeTask('task-a', 0);
const taskB = makeTask('task-b', 1);
const completed = makeTask('task-c', 2, { isCompleted: true, completedAt: '2026-06-02T00:00:00Z' });

const role = { color: '#4F46E5' };

function setup(tasks: Task[] = [taskA, taskB, completed]) {
  const onReorderTasks = vi.fn();
  render(
    <TasksTab
      role={role}
      tasks={tasks}
      isLoading={false}
      error={null}
      onOpenTask={vi.fn()}
      onDeleteTask={vi.fn()}
      onReorderTasks={onReorderTasks}
      onToggleComplete={vi.fn()}
    />,
  );
  return { onReorderTasks };
}

describe('TasksTab 排序模式（Story 16.4）', () => {
  it('排序开关默认关：拖拽把手在场、↑/↓ 按钮缺席', () => {
    setup();

    expect(screen.getByRole('button', { name: '拖动排序 任务 task-a' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '上移 任务 task-a' })).not.toBeInTheDocument();
  });

  it('点「排序」开模式：拖拽把手隐藏、↑/↓ 按钮登场（禁用拖拽）', () => {
    setup();

    fireEvent.click(screen.getByTestId('sort-mode-Q1'));

    // useSortable({ disabled: true }) ⇒ 拖拽把手不渲染
    expect(screen.queryByRole('button', { name: '拖动排序 任务 task-a' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '上移 任务 task-a' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '下移 任务 task-a' })).toBeInTheDocument();
    // 首任务 ↑ 禁用（边界）
    expect(screen.getByRole('button', { name: '上移 任务 task-a' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '下移 任务 task-a' })).toBeEnabled();
  });

  it('↑/↓ 调序复用 handleDragEnd 同款落库：全量序（新序未完成 + 已完成尾随）', () => {
    const { onReorderTasks } = setup();

    fireEvent.click(screen.getByTestId('sort-mode-Q1'));
    fireEvent.click(screen.getByRole('button', { name: '下移 任务 task-a' }));

    expect(onReorderTasks).toHaveBeenCalledTimes(1);
    expect(onReorderTasks).toHaveBeenCalledWith(['task-b', 'task-a', 'task-c']);
  });

  it('上移末位任务：序反转回原位', () => {
    const { onReorderTasks } = setup();

    fireEvent.click(screen.getByTestId('sort-mode-Q1'));
    fireEvent.click(screen.getByRole('button', { name: '上移 任务 task-b' }));

    expect(onReorderTasks).toHaveBeenCalledWith(['task-b', 'task-a', 'task-c']);
  });

  it('末任务 ↓ 禁用（边界不产生空重排）', () => {
    setup();

    fireEvent.click(screen.getByTestId('sort-mode-Q1'));

    expect(screen.getByRole('button', { name: '下移 任务 task-b' })).toBeDisabled();
  });

  it('筛选视图（象限 chip）下排序入口不渲染（与拖拽守卫一致）', () => {
    setup();

    fireEvent.click(screen.getByRole('button', { name: 'Q1' }));

    expect(screen.queryByTestId('sort-mode-Q1')).not.toBeInTheDocument();
  });

  it('排序模式可再点关闭（回到拖拽把手）', () => {
    setup();

    const toggle = screen.getByTestId('sort-mode-Q1');
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-pressed', 'false');

    expect(screen.getByRole('button', { name: '拖动排序 任务 task-a' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '上移 任务 task-a' })).not.toBeInTheDocument();
  });
});
