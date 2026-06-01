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
  MemoryTab: ({ onCategoryChange }: any) => (
    <button type="button" onClick={() => onCategoryChange?.('preference')}>筛选偏好</button>
  ),
}));

vi.mock('./ButlerSettingsContent', () => ({
  ButlerSettingsContent: () => <div>管家设置内容</div>,
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
      />
    );

    expect(await screen.findByRole('button', { name: '管家记忆 (18)' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenLastCalledWith({ roleId: null, includeRoleMemories: true, category: 'preference' });
    });
    expect(await screen.findByRole('button', { name: '管家记忆 (5)' })).toBeInTheDocument();
  });
});
