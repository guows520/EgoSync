import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RoleView } from './RoleView';
import type { Role } from '../../types/role';

vi.mock('./RoleHeader', () => ({
  RoleHeader: () => <div>角色头部</div>,
}));

vi.mock('../chat/ChatStream', () => ({
  ChatStream: ({ onMemoryReferenceClick, sourceNavigationTarget, onSourceNavigationHandled }: any) => (
    <div>
      <button type="button" onClick={() => onMemoryReferenceClick?.('memory-2')}>
        点击记忆引用
      </button>
      <div data-testid="role-source-target">{sourceNavigationTarget?.messageId ?? 'none'}</div>
      <button type="button" onClick={() => onSourceNavigationHandled?.()}>模拟来源跳转完成</button>
    </div>
  ),
}));

vi.mock('./RoleWorkspacePanel', () => ({
  RoleWorkspacePanel: ({ currentTab, targetMemoryId, onTargetMemoryHandled, onSourceMessageClick }: any) => (
    <div>
      <div data-testid="role-current-tab">{currentTab}</div>
      <div data-testid="role-target-memory">{targetMemoryId ?? 'none'}</div>
      <button type="button" onClick={() => onTargetMemoryHandled?.()}>模拟定位完成</button>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-role-source', messageId: 'msg-role-source', roleId: 'role-1' })}
      >
        点击来源记录
      </button>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-butler-source', messageId: 'msg-butler-source', roleId: null })}
      >
        点击管家来源记录
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

describe('RoleView memory reference navigation', () => {
  it('点击聊天中的记忆引用后打开记忆侧栏并传递目标 ID，定位完成后清理目标', () => {
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

    expect(screen.queryByTestId('role-current-tab')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '点击记忆引用' }));

    expect(screen.getByTestId('role-current-tab')).toHaveTextContent('memory');
    expect(screen.getByTestId('role-target-memory')).toHaveTextContent('memory-2');

    fireEvent.click(screen.getByRole('button', { name: '模拟定位完成' }));

    expect(screen.getByTestId('role-target-memory')).toHaveTextContent('none');
  });

  it('点击当前角色来源记录后把来源定位目标传给聊天流，跳转完成后清理目标', () => {
    render(
      <RoleView
        role={role}
        onOpenTask={vi.fn()}
        initialTab="memory"
        onTabConsumed={vi.fn()}
        onUpdateRole={vi.fn()}
        onArchiveRole={vi.fn()}
        onDeleteRole={vi.fn()}
        activeRoleCount={1}
      />,
    );

    expect(screen.getByTestId('role-source-target')).toHaveTextContent('none');

    fireEvent.click(screen.getByRole('button', { name: '点击来源记录' }));

    expect(screen.getByTestId('role-source-target')).toHaveTextContent('msg-role-source');

    fireEvent.click(screen.getByRole('button', { name: '模拟来源跳转完成' }));

    expect(screen.getByTestId('role-source-target')).toHaveTextContent('none');
  });

  it('点击指向管家的来源记录时上抛给 App 切回管家视图', () => {
    const onButlerSourceNavigation = vi.fn();

    render(
      <RoleView
        role={role}
        onOpenTask={vi.fn()}
        initialTab="memory"
        onTabConsumed={vi.fn()}
        onUpdateRole={vi.fn()}
        onArchiveRole={vi.fn()}
        onDeleteRole={vi.fn()}
        activeRoleCount={1}
        onButlerSourceNavigation={onButlerSourceNavigation}
      />,
    );

    expect(screen.getByTestId('role-source-target')).toHaveTextContent('none');

    fireEvent.click(screen.getByRole('button', { name: '点击管家来源记录' }));

    expect(onButlerSourceNavigation).toHaveBeenCalledWith({
      conversationId: 'conv-butler-source',
      messageId: 'msg-butler-source',
      roleId: null,
    });
    expect(screen.getByTestId('role-source-target')).toHaveTextContent('none');
  });
});
