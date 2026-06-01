import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { RoleWorkspacePanel } from './RoleWorkspacePanel';
import { memoryService } from '../../services/memoryService';
import type { Role } from '../../types/role';

vi.mock('../../services/memoryService', () => ({
  memoryService: {
    count: vi.fn(),
  },
}));

vi.mock('./TasksTab', () => ({
  TasksTab: () => <div>任务清单内容</div>,
}));

vi.mock('./MemoryTab', () => ({
  MemoryTab: ({ onCategoryChange, onMemoryDeleted }: any) => (
    <div>
      <button type="button" onClick={() => onCategoryChange?.('preference')}>筛选偏好</button>
      <button type="button" onClick={() => onMemoryDeleted?.()}>模拟删除成功</button>
    </div>
  ),
}));

vi.mock('./SettingsTab', () => ({
  SettingsTab: () => <div>设置内容</div>,
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

describe('RoleWorkspacePanel memory badge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('记忆档案标签显示当前角色记忆数量', async () => {
    vi.mocked(memoryService.count).mockResolvedValue(12);

    render(<RoleWorkspacePanel role={role} currentTab="memory" setTab={vi.fn()} />);

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenCalledWith({ roleId: 'role-1', category: undefined });
    });
    expect(await screen.findByRole('button', { name: '记忆档案 (12)' })).toBeInTheDocument();
  });

  it('切换类别筛选后 badge 随筛选范围联动', async () => {
    vi.mocked(memoryService.count).mockResolvedValueOnce(12).mockResolvedValueOnce(3);

    render(<RoleWorkspacePanel role={role} currentTab="memory" setTab={vi.fn()} />);

    expect(await screen.findByRole('button', { name: '记忆档案 (12)' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenLastCalledWith({ roleId: 'role-1', category: 'preference' });
    });
    expect(await screen.findByRole('button', { name: '记忆档案 (3)' })).toBeInTheDocument();
  });

  it('删除成功回调后按当前角色与类别重新刷新 badge', async () => {
    vi.mocked(memoryService.count)
      .mockResolvedValueOnce(12)
      .mockResolvedValueOnce(3)
      .mockResolvedValueOnce(2);

    render(<RoleWorkspacePanel role={role} currentTab="memory" setTab={vi.fn()} />);

    expect(await screen.findByRole('button', { name: '记忆档案 (12)' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '筛选偏好' }));
    expect(await screen.findByRole('button', { name: '记忆档案 (3)' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '模拟删除成功' }));

    await waitFor(() => {
      expect(memoryService.count).toHaveBeenLastCalledWith({ roleId: 'role-1', category: 'preference' });
    });
    expect(await screen.findByRole('button', { name: '记忆档案 (2)' })).toBeInTheDocument();
  });
});
