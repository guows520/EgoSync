import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { DashboardTab } from './DashboardTab';
import type { DashboardStatus } from '../../types/dashboard';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../lib/roleIcons', () => ({
  getRoleIconComponent: () => () => null,
  normalizeColorHex: (c: string) => c || '#4F46E5',
}));

import { invoke } from '@tauri-apps/api/core';
const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeStatus(id: string, overrides: Partial<DashboardStatus> = {}): DashboardStatus {
  return {
    roleId: id,
    roleName: `角色${id}`,
    roleIcon: 'target',
    roleColor: '#4F46E5',
    energy: 70,
    pendingTasksCount: 2,
    lastActiveAt: '2026-06-23T10:00:00Z',
    hasUrgent: false,
    ...overrides,
  };
}

describe('DashboardTab 无障碍', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('能量进度条应有 role="progressbar" 和 aria 属性', async () => {
    const status = makeStatus('r1', { energy: 75 });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve({ taskCount: 0, memoryCount: 0, conversationCount: 0, pendingTaskCount: 0, generatedAt: '2026-07-24T10:00:00Z' });
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getAllByText('角色r1').length).toBeGreaterThan(0);
    });

    const progressbar = screen.getByRole('progressbar');
    expect(progressbar).toHaveAttribute('aria-valuenow', '75');
    expect(progressbar).toHaveAttribute('aria-valuemin', '0');
    expect(progressbar).toHaveAttribute('aria-valuemax', '100');
    expect(progressbar).toHaveAttribute('aria-label');
  });
});
