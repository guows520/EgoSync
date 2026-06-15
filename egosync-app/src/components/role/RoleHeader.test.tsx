import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RoleHeader } from './RoleHeader';
import type { Role } from '../../types/role';

const baseRole: Role = {
  id: 'role-1',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  goal: '打磨产品节奏',
  personalityPrompt: '',
  status: 'active',
  energy: 72,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

describe('RoleHeader', () => {
  /// AC-1: 角色视图头部必须把后端 Role 的 name / energy 直接呈现给用户。
  /// 如果这里渲染错了，就出现"侧边栏图标显示真实角色，但进入视图后还是 mock"
  /// 的撕裂感，破坏 Story 1.9 已建立的"角色是真实的"信任。
  it('渲染角色名与能量百分比', () => {
    render(<RoleHeader role={baseRole} openTab={null} onToggleTab={vi.fn()} />);
    expect(screen.getByText('产品经理')).toBeInTheDocument();
    expect(screen.getByText('打磨产品节奏')).toBeInTheDocument();
    expect(screen.getByText('72% 能量')).toBeInTheDocument();
  });

  /// AC-5: tab 切换是用户控制工作台的唯一入口。点击必须把 tab key 透传给
  /// 父组件 RoleView，否则用户点了"任务"什么都不会发生 —— 严重退化。
  it('点击 tab 按钮回调 onToggleTab', () => {
    const onToggleTab = vi.fn();
    render(<RoleHeader role={baseRole} openTab={null} onToggleTab={onToggleTab} />);

    fireEvent.click(screen.getByRole('button', { name: /任务/ }));
    fireEvent.click(screen.getByRole('button', { name: /记忆/ }));
    fireEvent.click(screen.getByRole('button', { name: /设置/ }));

    expect(onToggleTab).toHaveBeenNthCalledWith(1, 'tasks');
    expect(onToggleTab).toHaveBeenNthCalledWith(2, 'memory');
    expect(onToggleTab).toHaveBeenNthCalledWith(3, 'settings');
  });

  /// AC-5: active tab 着色必须用 role.color (hex) 注入 inline style，
  /// 而不是依赖已废弃的 `role.text` Tailwind class —— 否则真实数据角色
  /// 永远拿不到品牌色，全部退回灰色，违反 Epic 1 的 mock 收敛纪律。
  it('active tab 用 role.color 作为前景色', () => {
    render(<RoleHeader role={baseRole} openTab="tasks" onToggleTab={vi.fn()} />);
    const taskButton = screen.getByRole('button', { name: /任务/ });
    // jsdom 会把 #4F46E5 标准化为 rgb(79, 70, 229)，断言等价值。
    expect(taskButton.style.color).toBe('rgb(79, 70, 229)');
  });
});