import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { llmConfigService } from '../../services/llmConfigService';
import { roleService } from '../../services/roleService';
import type { Role } from '../../types/role';

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    setDefault: vi.fn(),
    testConnection: vi.fn(),
  },
}));

vi.mock('../../services/roleService', () => ({
  roleService: {
    listArchived: vi.fn(),
    restore: vi.fn(),
  },
}));

const archivedRole: Role = {
  id: 'role-archived',
  name: '学习者',
  icon: 'book-open',
  color: '#10B981',
  goal: '保持学习节奏',
  personalityPrompt: '',
  status: 'archived',
  energy: 70,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: '2026-01-02T00:00:00Z',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
};

describe('GlobalSettingsModal archived roles', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(llmConfigService.list).mockResolvedValue([]);
    vi.mocked(roleService.listArchived).mockResolvedValue([archivedRole]);
  });

  it('打开设置时加载归档角色并可恢复', async () => {
    const onRestoreRole = vi.fn().mockResolvedValue(undefined);

    render(
      <GlobalSettingsModal
        onClose={vi.fn()}
        onRestoreRole={onRestoreRole}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: '数据与主权' }));

    expect(await screen.findByText('学习者')).toBeInTheDocument();
    expect(screen.getByText('保持学习节奏')).toBeInTheDocument();
    expect(screen.getByText(/归档时间：/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /恢复/ }));

    await waitFor(() => {
      expect(onRestoreRole).toHaveBeenCalledWith('role-archived');
      expect(roleService.listArchived).toHaveBeenCalledTimes(2);
    });
  });

  it('没有归档角色时显示空态', async () => {
    vi.mocked(roleService.listArchived).mockResolvedValue([]);

    render(<GlobalSettingsModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: '数据与主权' }));

    expect(await screen.findByText('暂无归档角色')).toBeInTheDocument();
  });
});