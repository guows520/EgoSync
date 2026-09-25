// Story 16.4：Sidebar 移动退场测试——桌面侧栏在 <768px 经 max-md:hidden
// 退场（BottomTabBar md:hidden 成对互斥）；桌面渲染零变化（既有 Sidebar
// 行为/属性不动，仅新增一个响应式类）。

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Sidebar } from './Sidebar';
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

describe('Sidebar 移动端退场（Story 16.4 max-md:hidden）', () => {
  it('侧栏根元素带 max-md:hidden（<768px 退场，桌面 w-16 原值不变）', () => {
    render(
      <Sidebar
        roles={[baseRole]}
        currentView="chat"
        onViewChange={vi.fn()}
        isSettingsOpen={false}
        onOpenSettings={vi.fn()}
        onCloseSettings={vi.fn()}
        onAddRole={vi.fn()}
        onArchiveRole={vi.fn()}
        onDeleteRole={vi.fn()}
        theme="light"
        onToggleTheme={vi.fn()}
        onEditRole={vi.fn()}
        isNotifOpen={false}
        onToggleNotif={vi.fn()}
      />,
    );

    const nav = screen.getByRole('navigation', { name: '角色导航' });
    expect(nav).toHaveClass('max-md:hidden');
    // 桌面原值保持（NFR-C4：既有桌面值零改动）
    expect(nav).toHaveClass('w-16');
  });
});
