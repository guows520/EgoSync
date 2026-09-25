import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RoleHeader } from './RoleHeader';
import type { Role } from '../../types/role';

// Story 16.4 D2 收口（2026-09-25 人类指令「移动端不能归档/删除角色 这个要修复」）：
// 桌面侧栏右键菜单的归档/删除在 <768px 随侧栏退场消失，本套件钉移动端
// 「⋯」菜单新增的归档/删除入口与确认弹窗语义（删除需输入角色名、错误归一化、
// 与桌面 Sidebar ConfirmDialog 同语义）。新建文件——冻结块「禁改既有断言、
// 新测试一律新建文件」。
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

const openMoreMenu = () => {
  fireEvent.click(screen.getByTestId('role-more-menu'));
};

describe('RoleHeader 移动端「⋯」菜单归档/删除入口（Story 16.4 D2 收口）', () => {
  it('传入处理器时菜单出现归档/删除两项（与切换角色/新建角色同菜单）', () => {
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onArchiveRole={vi.fn()}
        onDeleteRole={vi.fn()}
      />,
    );
    openMoreMenu();

    expect(screen.getByTestId('role-menu-switch')).toBeInTheDocument();
    expect(screen.getByTestId('role-menu-add')).toBeInTheDocument();
    expect(screen.getByTestId('role-menu-archive')).toBeInTheDocument();
    expect(screen.getByTestId('role-menu-delete')).toBeInTheDocument();
  });

  it('未传入处理器时不渲染归档/删除（桌面/无权限面零新增）', () => {
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
      />,
    );
    openMoreMenu();

    expect(screen.queryByTestId('role-menu-archive')).not.toBeInTheDocument();
    expect(screen.queryByTestId('role-menu-delete')).not.toBeInTheDocument();
  });

  it('归档：菜单→确认弹窗→确认后回调 onArchiveRole(role.id) 且弹窗关闭', async () => {
    const onArchiveRole = vi.fn().mockResolvedValue(undefined);
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onArchiveRole={onArchiveRole}
      />,
    );
    openMoreMenu();
    fireEvent.click(screen.getByTestId('role-menu-archive'));

    expect(screen.getByRole('dialog', { name: '确认归档' })).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('role-confirm-archive'));

    await waitFor(() => expect(onArchiveRole).toHaveBeenCalledWith('role-1'));
    await waitFor(() => expect(screen.queryByRole('dialog', { name: '确认归档' })).not.toBeInTheDocument());
  });

  it('删除：未输入角色名时确认禁用，输入正确角色名后回调 onDeleteRole(role.id)', async () => {
    const onDeleteRole = vi.fn().mockResolvedValue(undefined);
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onDeleteRole={onDeleteRole}
      />,
    );
    openMoreMenu();
    fireEvent.click(screen.getByTestId('role-menu-delete'));

    const confirmBtn = screen.getByTestId('role-confirm-delete') as HTMLButtonElement;
    expect(confirmBtn.disabled).toBe(true);

    fireEvent.change(screen.getByTestId('role-delete-confirm-input'), { target: { value: '产品经理' } });
    expect(confirmBtn.disabled).toBe(false);

    fireEvent.click(confirmBtn);
    await waitFor(() => expect(onDeleteRole).toHaveBeenCalledWith('role-1'));
  });

  it('删除：输入错误的角色名确认仍禁用（防误删第二道闸）', () => {
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onDeleteRole={vi.fn()}
      />,
    );
    openMoreMenu();
    fireEvent.click(screen.getByTestId('role-menu-delete'));
    fireEvent.change(screen.getByTestId('role-delete-confirm-input'), { target: { value: '产品' } });

    expect((screen.getByTestId('role-confirm-delete') as HTMLButtonElement).disabled).toBe(true);
  });

  it('取消关闭弹窗且不调用任何处理器', () => {
    const onArchiveRole = vi.fn();
    const onDeleteRole = vi.fn();
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onArchiveRole={onArchiveRole}
        onDeleteRole={onDeleteRole}
      />,
    );
    openMoreMenu();
    fireEvent.click(screen.getByTestId('role-menu-archive'));
    fireEvent.click(screen.getByTestId('role-confirm-cancel'));

    expect(screen.queryByRole('dialog', { name: '确认归档' })).not.toBeInTheDocument();
    expect(onArchiveRole).not.toHaveBeenCalled();
    expect(onDeleteRole).not.toHaveBeenCalled();
  });

  it('执行失败：错误文案呈现且弹窗保持打开（含「至少保留一个角色」归一化）', async () => {
    const onArchiveRole = vi.fn().mockRejectedValue(new Error('至少保留一个角色'));
    render(
      <RoleHeader
        role={baseRole}
        openTab={null}
        onToggleTab={vi.fn()}
        onSwitchRole={vi.fn()}
        onAddRole={vi.fn()}
        onArchiveRole={onArchiveRole}
      />,
    );
    openMoreMenu();
    fireEvent.click(screen.getByTestId('role-menu-archive'));
    fireEvent.click(screen.getByTestId('role-confirm-archive'));

    await waitFor(() => expect(screen.getByText('至少保留一个角色')).toBeInTheDocument());
    expect(screen.getByRole('dialog', { name: '确认归档' })).toBeInTheDocument();
    expect(onArchiveRole).toHaveBeenCalledTimes(1);
  });
});
