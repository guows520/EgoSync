import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ButlerView } from './ButlerView';

vi.mock('../chat/ChatStream', () => ({
  ChatStream: ({ onMemoryReferenceClick, sourceNavigationTarget, onSourceNavigationHandled }: any) => (
    <div>
      <button type="button" onClick={() => onMemoryReferenceClick?.('memory-1')}>
        点击管家记忆引用
      </button>
      <div data-testid="butler-source-target">{sourceNavigationTarget?.messageId ?? 'none'}</div>
      <button type="button" onClick={() => onSourceNavigationHandled?.()}>模拟管家来源跳转完成</button>
    </div>
  ),
}));

vi.mock('./ButlerWorkspacePanel', () => ({
  ButlerWorkspacePanel: ({ currentTab, targetMemoryId, onTargetMemoryHandled, onSourceMessageClick }: any) => (
    <div>
      <div data-testid="butler-current-tab">{currentTab}</div>
      <div data-testid="butler-target-memory">{targetMemoryId ?? 'none'}</div>
      <button type="button" onClick={() => onTargetMemoryHandled?.()}>模拟定位完成</button>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-butler-source', messageId: 'msg-butler-source', roleId: null })}
      >
        点击管家来源记录
      </button>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-role-source', messageId: 'msg-role-source', roleId: 'role-1' })}
      >
        点击角色来源记录
      </button>
    </div>
  ),
}));

describe('ButlerView memory reference navigation', () => {
  it('显示数字分身管家文案并为顶部页签提供深色模式样式', () => {
    render(
      <ButlerView
        roles={[]}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onRoleSourceNavigation={vi.fn()}
        sourceNavigationTarget={null}
        onSourceNavigationHandled={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        knockNotifications={[]}
        onDismissKnock={vi.fn()}
        chatRefreshTrigger={0}
      />,
    );

    expect(screen.getByText('数字分身管家')).toBeInTheDocument();
    const dashboardButton = screen.getByRole('button', { name: /仪表盘/ });
    expect(dashboardButton).toHaveClass('dark:text-slate-400', 'dark:hover:bg-slate-700/60');
    fireEvent.click(dashboardButton);
    expect(dashboardButton).toHaveClass('dark:bg-slate-700', 'dark:text-indigo-300');
  });

  it('点击聊天中的记忆引用后打开记忆侧栏并传递目标 ID，定位完成后清理目标', () => {
    render(
      <ButlerView
        roles={[]}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onRoleSourceNavigation={vi.fn()}
        sourceNavigationTarget={null}
        onSourceNavigationHandled={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        knockNotifications={[]}
        onDismissKnock={vi.fn()}
        chatRefreshTrigger={0}
      />,
    );

    expect(screen.queryByTestId('butler-current-tab')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '点击管家记忆引用' }));

    expect(screen.getByTestId('butler-current-tab')).toHaveTextContent('memory');
    expect(screen.getByTestId('butler-target-memory')).toHaveTextContent('memory-1');

    fireEvent.click(screen.getByRole('button', { name: '模拟定位完成' }));

    expect(screen.getByTestId('butler-target-memory')).toHaveTextContent('none');
  });

  it('点击管家来源记录后把来源定位目标传给管家聊天流', () => {
    render(
      <ButlerView
        roles={[]}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onRoleSourceNavigation={vi.fn()}
        sourceNavigationTarget={null}
        onSourceNavigationHandled={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        knockNotifications={[]}
        onDismissKnock={vi.fn()}
        chatRefreshTrigger={0}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '点击管家记忆引用' }));
    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('none');

    fireEvent.click(screen.getByRole('button', { name: '点击管家来源记录' }));

    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('msg-butler-source');

    fireEvent.click(screen.getByRole('button', { name: '模拟管家来源跳转完成' }));

    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('none');
  });

  it('点击角色来源记录后请求切换到对应角色视图', () => {
    const onRoleSourceNavigation = vi.fn();

    render(
      <ButlerView
        roles={[]}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onRoleSourceNavigation={onRoleSourceNavigation}
        sourceNavigationTarget={null}
        onSourceNavigationHandled={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        knockNotifications={[]}
        onDismissKnock={vi.fn()}
        chatRefreshTrigger={0}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '点击管家记忆引用' }));
    fireEvent.click(screen.getByRole('button', { name: '点击角色来源记录' }));

    expect(onRoleSourceNavigation).toHaveBeenCalledWith({
      conversationId: 'conv-role-source',
      messageId: 'msg-role-source',
      roleId: 'role-1',
    });
    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('none');
  });
});
