import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RoleView } from './RoleView';
import type { Role } from '../../types/role';

// Story 16.4 评审轮 G11：16.2 既有测试 RoleView.test.tsx 的钉孔（h-[42%]）
// 是 16.4 之前小屏堆叠比例的载体；58/42 flex 修复后该钉孔由组件侧的 inert
// 遗留垫片保持有效（见 RoleView.tsx 注释）。本文件新建，钉修复后的真实布局
// 类——冻结块「禁改既有断言、新测试一律新建文件」两不违。
vi.mock('./RoleHeader', () => ({
  RoleHeader: ({ onToggleTab }: any) => (
    <div>
      角色头部
      <button type="button" onClick={() => onToggleTab('tasks')}>任务</button>
    </div>
  ),
}));

vi.mock('../chat/ChatStream', () => ({
  ChatStream: () => <div>聊天区</div>,
}));

vi.mock('./RoleWorkspacePanel', () => ({
  RoleWorkspacePanel: ({ currentTab }: any) => <div data-testid="role-current-tab">{currentTab}</div>,
}));

const role: Role = {
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

const renderRoleView = () =>
  render(
    <RoleView
      role={role}
      onOpenTask={vi.fn()}
      initialTab={null}
      onTabConsumed={vi.fn()}
      onUpdateRole={vi.fn()}
      onArchiveRole={vi.fn()}
      onDeleteRole={vi.fn()}
      activeRoleCount={1}
    />,
  );

describe('RoleView 小屏 58/42 堆叠比例（Story 16.4 flex 修复钉孔）', () => {
  it('工作区开启时以 max-md:flex-[42] 取 42% 定高比例（附带 inert h-[42%] 垫片保既有钉孔）', () => {
    const { container } = renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));

    const workspacePane = container.querySelector('[aria-hidden="false"]');
    // 真实生效类：flex 列子项 flex-basis 优先于 height，对定高容器免疫百分比退化
    expect(workspacePane).toHaveClass('max-md:flex-[42]');
    // inert 遗留垫片：保留仅为 16.2 既有测试 h-[42%] 钉孔继续有效（冻结边界）
    expect(workspacePane).toHaveClass('h-[42%]');
    // 桌面钉孔不动（NFR-C4）
    expect(workspacePane).toHaveClass('md:h-auto');
    expect(workspacePane).toHaveClass('md:w-[35%]');
  });

  it('对话区以 max-md:flex-[58] 取 58% 比例，桌面 65% 双栏钉孔不动', () => {
    const { container } = renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));

    // 对话区 = 工作区 pane 的前一个兄弟（RoleView 渲染序：ChatStream pane → workspace pane）
    const workspacePane = container.querySelector('[aria-hidden="false"]');
    const chatPane = workspacePane?.previousElementSibling;
    expect(chatPane).toHaveClass('max-md:flex-[58]');
    expect(chatPane).toHaveClass('md:h-auto');
    expect(chatPane).toHaveClass('md:w-[65%]');
  });
});
