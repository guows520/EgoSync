import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { SettingsTab } from './SettingsTab';
import { roleService } from '../../services/roleService';
import type { Role } from '../../types/role';

vi.mock('../../services/roleService', () => ({
  roleService: {
    update: vi.fn(),
  },
}));

const baseRole: Role = {
  id: 'role-1',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  goal: '管理产品规划',
  personalityPrompt: '',
  status: 'active',
  energy: 100,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const updatedRole: Role = {
  ...baseRole,
  name: '学习者',
  icon: 'graduation-cap',
  color: '#10B981',
  goal: '保持学习节奏',
  updatedAt: '2026-01-02T00:00:00Z',
};

describe('SettingsTab role CRUD actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('保存后调用 update 并回传更新后的角色', async () => {
    vi.mocked(roleService.update).mockResolvedValue(updatedRole);
    const onUpdateRole = vi.fn();

    render(
      <SettingsTab
        role={baseRole}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.change(screen.getByLabelText('名称'), { target: { value: '学习者' } });
    fireEvent.change(screen.getByLabelText('目标'), { target: { value: '保持学习节奏' } });
    fireEvent.change(screen.getByLabelText('角色个性描述'), { target: { value: '简洁专业，先判断优先级再给建议' } });
    fireEvent.click(screen.getByTitle('学习'));
    fireEvent.click(screen.getByText('翠绿'));
    fireEvent.click(screen.getByRole('button', { name: '保存更改' }));

    await waitFor(() => {
      expect(roleService.update).toHaveBeenCalledWith('role-1', {
        name: '学习者',
        icon: 'graduation-cap',
        color: '#10B981',
        goal: '保持学习节奏',
        personalityPrompt: '简洁专业，先判断优先级再给建议',
      });
    });
    expect(onUpdateRole).toHaveBeenCalledWith(updatedRole);
    expect(await screen.findByText('已保存')).toBeInTheDocument();
  });

  it('永久删除必须输入角色名确认', () => {
    const onDeleteRole = vi.fn();

    render(
      <SettingsTab
        role={baseRole}
        activeRoleCount={2}
        onDeleteRole={onDeleteRole}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /永久删除/ }));
    const confirmButton = screen.getByRole('button', { name: '永久删除' });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText('产品经理'), { target: { value: '错误名称' } });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText('产品经理'), { target: { value: '产品经理' } });
    expect(confirmButton).not.toBeDisabled();
  });

  it('仅剩一个 active 角色时禁用危险操作', () => {
    render(<SettingsTab role={baseRole} activeRoleCount={1} />);

    expect(screen.getByText('至少保留一个角色')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /归档角色/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /永久删除/ })).toBeDisabled();
  });
});