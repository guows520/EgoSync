import { fireEvent, render, screen, within } from '@testing-library/react';
import type { ComponentProps } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { TaskOverviewTab } from './TaskOverviewTab';
import type { CrossRoleTask } from '../../types/task';
import type { Role } from '../../types/role';

const roles: Role[] = [
  {
    id: 'role-1',
    name: '产品',
    icon: 'target',
    color: '#4F46E5',
    goal: '',
    personalityPrompt: '',
    status: 'active',
    energy: 100,
    skillsConfig: '{}',
    proactivityLevel: 'moderate',
    archivedAt: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
  },
  {
    id: 'role-2',
    name: '学习',
    icon: 'book',
    color: '#0EA5E9',
    goal: '',
    personalityPrompt: '',
    status: 'active',
    energy: 100,
    skillsConfig: '{}',
    proactivityLevel: 'moderate',
    archivedAt: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
  },
];

function task(overrides: Partial<CrossRoleTask> = {}): CrossRoleTask {
  return {
    id: 'task-1',
    ownerType: 'role',
    roleId: 'role-1',
    roleName: '产品',
    roleColor: '#4F46E5',
    title: '默认任务',
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
    ...overrides,
  };
}

function renderOverview(overrides: Partial<ComponentProps<typeof TaskOverviewTab>> = {}) {
  const props: ComponentProps<typeof TaskOverviewTab> = {
    roles,
    tasks: [
      task({ id: 'role-q1', title: '角色紧急任务', quadrant: 'Q1', roleId: 'role-1', roleName: '产品' }),
      task({ id: 'butler-q2', title: '管家规划任务', ownerType: 'butler', roleId: null, roleName: null, roleColor: null, quadrant: 'Q2' }),
      task({ id: 'role-rock', title: '学习大石头', quadrant: 'Q2', roleId: 'role-2', roleName: '学习', roleColor: '#0EA5E9', isBigRock: true }),
      task({ id: 'done-q2', title: '已完成任务', quadrant: 'Q2', isCompleted: true, completedAt: '2026-06-02T00:00:00Z' }),
    ],
    isLoading: false,
    error: null,
    classifyingIds: new Set(),
    quadrantFilter: 'all',
    onQuadrantFilterChange: vi.fn(),
    showBigRocksOnly: false,
    onToggleBigRocksOnly: vi.fn(),
    onOpenTask: vi.fn(),
    onDeleteTask: vi.fn(),
    onToggleComplete: vi.fn(),
    ...overrides,
  };

  render(<TaskOverviewTab {...props} />);
  return props;
}

describe('TaskOverviewTab', () => {
  it('按象限展示所有角色和管家任务，并显示归属标签', () => {
    renderOverview();

    expect(screen.getByText('角色紧急任务')).toBeInTheDocument();
    expect(screen.getByText('管家规划任务')).toBeInTheDocument();
    expect(screen.getByText('学习大石头')).toBeInTheDocument();
    expect(screen.getAllByText('产品').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('管家').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('学习').length).toBeGreaterThanOrEqual(1);
  });

  it('默认折叠已完成任务，可展开查看', () => {
    renderOverview();

    expect(screen.queryByText('已完成任务')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /已完成 \(1\)/ }));
    expect(screen.getByText('已完成任务')).toBeInTheDocument();
  });

  it('归属筛选使用“全部”入口并支持全选/全不选与多选过滤', () => {
    renderOverview();

    const ownerFilter = screen.getByRole('group', { name: '按归属筛选' });
    const allOwnersButton = within(ownerFilter).getByRole('button', { name: '全部' });
    expect(ownerFilter).toBeInTheDocument();
    expect(ownerFilter).toHaveClass('thin-horizontal-scrollbar');
    expect(allOwnersButton).toHaveAttribute('aria-pressed', 'true');

    fireEvent.click(allOwnersButton);
    expect(allOwnersButton).toHaveAttribute('aria-pressed', 'false');
    expect(screen.queryByText('角色紧急任务')).not.toBeInTheDocument();
    expect(screen.queryByText('学习大石头')).not.toBeInTheDocument();
    expect(screen.queryByText('管家规划任务')).not.toBeInTheDocument();
    expect(screen.getByText('当前筛选无匹配任务')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '产品' }));

    expect(screen.getByText('角色紧急任务')).toBeInTheDocument();
    expect(screen.queryByText('学习大石头')).not.toBeInTheDocument();
    expect(screen.queryByText('管家规划任务')).not.toBeInTheDocument();
  });

  it('象限 chip 与只看大石头通过回调上抛（服务端过滤）', () => {
    const onQuadrantFilterChange = vi.fn();
    const onToggleBigRocksOnly = vi.fn();
    renderOverview({ onQuadrantFilterChange, onToggleBigRocksOnly });

    fireEvent.click(screen.getByRole('button', { name: 'Q1' }));
    expect(onQuadrantFilterChange).toHaveBeenCalledWith('Q1');

    fireEvent.click(screen.getByRole('button', { name: '只看大石头' }));
    expect(onToggleBigRocksOnly).toHaveBeenCalled();
  });

  it('新建任务从管家 scope 打开，编辑任务也走管家全量任务 API scope', () => {
    const onOpenTask = vi.fn();
    renderOverview({ onOpenTask });

    fireEvent.click(screen.getByRole('button', { name: '新增任务' }));
    expect(onOpenTask).toHaveBeenCalledWith({ ownerType: 'butler' }, null);

    fireEvent.click(screen.getByRole('button', { name: '编辑 角色紧急任务' }));
    expect(onOpenTask).toHaveBeenCalledWith({ ownerType: 'butler' }, expect.objectContaining({ id: 'role-q1' }));
  });

  it('切换完成状态与删除任务会调用对应回调', () => {
    const onToggleComplete = vi.fn();
    const onDeleteTask = vi.fn();
    renderOverview({ onToggleComplete, onDeleteTask });

    fireEvent.click(screen.getByRole('button', { name: '完成 角色紧急任务' }));
    expect(onToggleComplete).toHaveBeenCalledWith('role-q1', true);

    fireEvent.click(screen.getByRole('button', { name: '删除 角色紧急任务' }));
    const dialog = screen.getByText('确认删除任务？').closest('div')!;
    fireEvent.click(within(dialog).getByRole('button', { name: '确认删除' }));
    expect(onDeleteTask).toHaveBeenCalledWith('role-q1');
  });
});
