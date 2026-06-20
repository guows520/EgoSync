import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ButlerWorkspacePanel } from './ButlerWorkspacePanel';
import { memoryService } from '../../services/memoryService';

vi.mock('../../services/memoryService', () => ({
  memoryService: {
    count: vi.fn(),
  },
}));

vi.mock('./DashboardTab', () => ({
  DashboardTab: () => <div>仪表盘内容</div>,
}));

vi.mock('../role/MemoryTab', () => ({
  MemoryTab: ({ includeRoleMemories, category, targetMemoryId, onCategoryChange, onTargetMemoryHandled, onMemoryDeleted }: any) => (
    <div>
      <div data-testid="include-role-memories">{String(includeRoleMemories)}</div>
      <div data-testid="memory-category">{category ?? 'all'}</div>
      <div data-testid="memory-target">{targetMemoryId ?? 'none'}</div>
      <button type="button" onClick={() => onCategoryChange?.('preference')}>筛选偏好</button>
      <button type="button" onClick={() => onTargetMemoryHandled?.()}>模拟定位完成</button>
      <button type="button" onClick={() => onMemoryDeleted?.()}>模拟删除成功</button>
    </div>
  ),
}));

vi.mock('./ButlerSettingsContent', () => ({
  ButlerSettingsContent: () => <div>管家设置内容</div>,
}));

vi.mock('./TaskOverviewTab', () => ({
  TaskOverviewTab: () => <div>任务总览内容</div>,
}));

vi.mock('../../hooks/useAllTasks', () => ({
  useAllTasks: () => ({
    tasks: [],
    isLoading: false,
    error: null,
    classifyingIds: new Set(),
    refetch: vi.fn(),
    createTask: vi.fn(),
    updateTask: vi.fn(),
    deleteTask: vi.fn(),
    toggleComplete: vi.fn(),
  }),
}));

describe('ButlerWorkspacePanel memory badge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('管家记忆标签显示全量总览记忆数量', async () => {
    vi.mocked(memoryService.count).mockResolvedValue(18);

    render(
      <ButlerWorkspacePanel
        roles={[]}
        currentTab="memory"
        setTab={vi.fn()}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        targetMemoryId={null}
        onTargetMemoryHandled={vi.fn()}
        onSourceMessageClick={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenCalledWith({ roleId: null, includeRoleMemories: true, category: undefined });
    });
    expect(await screen.findByRole('button', { name: '管家记忆 (18)' })).toBeInTheDocument();
  });

  it('切换类别筛选后 badge 随筛选范围联动', async () => {
    vi.mocked(memoryService.count).mockResolvedValueOnce(18).mockResolvedValueOnce(5);

    render(
      <ButlerWorkspacePanel
        roles={[]}
        currentTab="memory"
        setTab={vi.fn()}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        targetMemoryId={null}
        onTargetMemoryHandled={vi.fn()}
        onSourceMessageClick={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
      />
    );

    expect(await screen.findByRole('button', { name: '管家记忆 (18)' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenLastCalledWith({ roleId: null, includeRoleMemories: true, category: 'preference' });
    });
    expect(await screen.findByRole('button', { name: '管家记忆 (5)' })).toBeInTheDocument();
  });

  it('记忆引用跳转打开 memory tab、清空类别并保留全局角色总览范围', async () => {
    vi.mocked(memoryService.count).mockResolvedValueOnce(5).mockResolvedValueOnce(18);
    const setTab = vi.fn();
    const onTargetMemoryHandled = vi.fn();

    render(
      <ButlerWorkspacePanel
        roles={[]}
        currentTab="memory"
        setTab={setTab}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onSourceMessageClick={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        targetMemoryId="memory-2"
        onTargetMemoryHandled={onTargetMemoryHandled}
      />
    );

    expect(await screen.findByRole('button', { name: '管家记忆 (5)' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));
    expect(await screen.findByRole('button', { name: '管家记忆 (18)' })).toBeInTheDocument();
    expect(screen.getByTestId('include-role-memories')).toHaveTextContent('true');
    expect(screen.getByTestId('memory-category')).toHaveTextContent('all');
    expect(screen.getByTestId('memory-target')).toHaveTextContent('memory-2');
    expect(setTab).toHaveBeenCalledWith('memory');

    fireEvent.click(screen.getByRole('button', { name: '模拟定位完成' }));

    expect(onTargetMemoryHandled).toHaveBeenCalledTimes(1);
  });

  it('目标记忆到来时立即以未筛选总览范围传给 MemoryTab，避免旧 category 先误判不可用', async () => {
    vi.mocked(memoryService.count).mockResolvedValueOnce(5).mockResolvedValueOnce(18);
    const setTab = vi.fn();

    const { rerender } = render(
      <ButlerWorkspacePanel
        roles={[]}
        currentTab="memory"
        setTab={setTab}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        targetMemoryId={null}
        onTargetMemoryHandled={vi.fn()}
        onSourceMessageClick={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
      />,
    );

    expect(await screen.findByRole('button', { name: '管家记忆 (5)' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));
    await waitFor(() => {
      expect(screen.getByTestId('memory-category')).toHaveTextContent('preference');
    });

    rerender(
      <ButlerWorkspacePanel
        roles={[]}
        currentTab="memory"
        setTab={setTab}
        archivedRoles={[]}
        onRestoreRole={vi.fn()}
        onViewChange={vi.fn()}
        onUpdateRole={vi.fn()}
        onSourceMessageClick={vi.fn()}
        onOpenTask={vi.fn()}
        onTasksApiReady={vi.fn()}
        targetMemoryId="memory-2"
        onTargetMemoryHandled={vi.fn()}
      />,
    );

    expect(screen.getByTestId('include-role-memories')).toHaveTextContent('true');
    expect(screen.getByTestId('memory-category')).toHaveTextContent('all');
    expect(screen.getByTestId('memory-target')).toHaveTextContent('memory-2');
  });
});
