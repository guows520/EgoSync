import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { RoleView } from './RoleView';
import type { Role } from '../../types/role';

// Story 16.4 布局修复（2026-09-26 人类指令）：小屏 tab 从 58/42 堆叠改为
// 全屏 drill-down——对话区 max-md:hidden、工作区 max-md:flex-1；再点一次
// 当前 tab 按钮关闭回对话（既有 toggle 语义）。
// 16.2 旧钉孔（h-[42%]/max-md:flex-[42]）已在 RoleView.test.tsx 与本文按
// 新布局改写（owner 授权解冻）；桌面 md:w-[65%]/md:w-[35%] 双栏钉孔不动。
vi.mock('./RoleHeader', () => ({
  RoleHeader: ({ onToggleTab }: any) => (
    <div>
      角色头部
      <button type="button" onClick={() => onToggleTab('tasks')}>任务</button>
    </div>
  ),
}));

vi.mock('../chat/ChatStream', () => ({
  ChatStream: ({ sourceNavigationTarget }: any) => (
    <div>
      聊天区
      <div data-testid="role-source-target">{sourceNavigationTarget?.messageId ?? 'none'}</div>
    </div>
  ),
}));

vi.mock('./RoleWorkspacePanel', () => ({
  RoleWorkspacePanel: ({ currentTab, onSourceMessageClick }: any) => (
    <div>
      <div data-testid="role-current-tab">{currentTab}</div>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-role-source', messageId: 'msg-role-source', roleId: 'role-1' })}
      >
        点击角色来源记录
      </button>
    </div>
  ),
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

/** 主区行容器：`flex-1 flex flex-col md:flex-row`（chat pane 与 workspace pane 的父） */
const mainRow = (container: HTMLElement) =>
  container.querySelector('.flex-1.flex-col.md\\:flex-row') as HTMLElement;

/** 打桩 matchMedia：仅 max-width: 767px 查询返回 matches=true（jsdom 恒 false） */
const stubMobileViewport = () => {
  vi.stubGlobal(
    'matchMedia',
    (query: string) => ({
      matches: query.includes('max-width: 767px'),
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }),
  );
};

describe('RoleView 小屏 tab 全屏化（Story 16.4 布局修复钉孔）', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });
  it('tab 打开时对话区 max-md:hidden（只显示工作区，桌面 65% 双栏钉孔不动）', () => {
    const { container } = renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));

    const chatPane = mainRow(container).children[0];
    expect(chatPane).toHaveClass('max-md:hidden');
    // 桌面双栏钉孔不动（NFR-C4）
    expect(chatPane).toHaveClass('md:h-auto');
    expect(chatPane).toHaveClass('md:w-[65%]');
  });

  it('tab 打开时工作区 max-md:flex-1 全屏（原 h-[42%] 堆叠垫片已移除）', () => {
    const { container } = renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));

    const workspacePane = mainRow(container).children[1];
    // 小屏全屏：flex-1 占满主区（替代原 max-md:flex-[42] 的 42% 定高比例）
    expect(workspacePane).toHaveClass('max-md:flex-1');
    expect(workspacePane).not.toHaveClass('h-[42%]');
    // 桌面 35% 双栏钉孔不动
    expect(workspacePane).toHaveClass('md:h-auto');
    expect(workspacePane).toHaveClass('md:w-[35%]');
    expect(screen.getByTestId('role-current-tab')).toHaveTextContent('tasks');
  });

  it('再点一次当前 tab 按钮关闭工作区、对话区回全屏（既有 toggle 语义）', () => {
    const { container } = renderRoleView();

    const taskButton = screen.getByRole('button', { name: '任务' });
    fireEvent.click(taskButton);
    expect(mainRow(container).children).toHaveLength(2);

    fireEvent.click(taskButton);

    const workspacePane = mainRow(container).children[1];
    expect(workspacePane).toHaveAttribute('aria-hidden', 'true');
    expect(workspacePane).toHaveClass('w-0');
    const chatPane = mainRow(container).children[0];
    expect(chatPane).toHaveClass('w-full', 'h-full');
  });

  it('移动视口下来源消息跳转先关 tab 回对话，再传定位目标（对话区 hidden 时跳转可见）', () => {
    stubMobileViewport();
    const { container } = renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));
    fireEvent.click(screen.getByRole('button', { name: '点击角色来源记录' }));

    // 工作区已卸载（tab 关闭）——对话区回到全屏，定位目标随之可见
    expect(screen.queryByTestId('role-current-tab')).not.toBeInTheDocument();
    expect(mainRow(container).children).toHaveLength(2);
    expect(mainRow(container).children[1]).toHaveClass('w-0');
    expect(screen.getByTestId('role-source-target')).toHaveTextContent('msg-role-source');
  });

  it('桌面视口下来源消息跳转保持工作区打开（既有双栏行为零变化）', () => {
    // 不打桩：jsdom matchMedia 恒 matches=false ⇒ isMobileViewport()=false
    renderRoleView();

    fireEvent.click(screen.getByRole('button', { name: '任务' }));
    fireEvent.click(screen.getByRole('button', { name: '点击角色来源记录' }));

    expect(screen.getByTestId('role-current-tab')).toHaveTextContent('tasks');
    expect(screen.getByTestId('role-source-target')).toHaveTextContent('msg-role-source');
  });
});
