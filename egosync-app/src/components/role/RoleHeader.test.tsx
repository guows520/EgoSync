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

  it('选中和未选中的页签均包含深色模式样式', () => {
    render(<RoleHeader role={baseRole} openTab="tasks" onToggleTab={vi.fn()} />);

    expect(screen.getByRole('button', { name: /任务/ })).toHaveClass('dark:bg-slate-700');
    expect(screen.getByRole('button', { name: /记忆/ })).toHaveClass('dark:text-slate-400', 'dark:hover:bg-slate-700/60');
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

  /// Story 16.4 布局修复（2026-09-26 人类指令）：小屏头部右侧控件顺序钉死——
  /// 「⋯」更多菜单在「设置」按钮右边（原渲染在 tab 组左侧）。
  it('「⋯」更多菜单渲染在「设置」按钮右边', () => {
    const { container } = render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
      />,
    );
    const cluster = container.querySelector('.ml-auto') as HTMLElement;
    const settingsButton = screen.getByRole('button', { name: /设置/ });
    const moreButton = screen.getByTestId('role-more-menu');
    expect(cluster.contains(settingsButton)).toBe(true);
    expect(cluster.contains(moreButton)).toBe(true);
    // DOM 序钉死：settings 在前、more 紧随其后 ⇒ 视觉上「⋯」在设置按钮右边
    expect(
      settingsButton.compareDocumentPosition(moreButton) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  /// 移动端去重（2026-09-26 人类指令，spec-web-mobile-tab-dedup）：面板自带
  /// tab 条小屏退场后，关闭 X 上移头部——仅 tab 打开时渲染；onClick 复用
  /// toggle 语义（onToggleTab(openTab) ⇒ 父组件再点当前 tab 即关闭），
  /// 不新增 props；md:hidden 保证桌面无此控件、零变化。
  it('tab 打开时头部渲染关闭钮，点击以当前 tab 回调 onToggleTab', () => {
    const onToggleTab = vi.fn();
    render(<RoleHeader role={baseRole} openTab="tasks" onToggleTab={onToggleTab} />);

    const closeButton = screen.getByTestId('role-close-tab');
    expect(closeButton).toHaveAttribute('aria-label', '关闭');
    expect(closeButton).toHaveClass('md:hidden');
    // DOM 序钉死：X 紧随「设置」tab（关闭入口与 tab 组相邻、在「⋯」左侧）
    const settingsButton = screen.getByRole('button', { name: /设置/ });
    expect(
      settingsButton.compareDocumentPosition(closeButton) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    fireEvent.click(closeButton);
    expect(onToggleTab).toHaveBeenCalledWith('tasks');
  });

  it('tab 未打开时头部不渲染关闭钮', () => {
    render(<RoleHeader role={baseRole} openTab={null} onToggleTab={vi.fn()} />);
    expect(screen.queryByTestId('role-close-tab')).not.toBeInTheDocument();
  });
});
