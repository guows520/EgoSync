import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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

describe('DashboardTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('加载中时显示加载提示', () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}));
    render(<DashboardTab onViewChange={vi.fn()} />);
    expect(screen.getByText('加载中…')).toBeInTheDocument();
  });

  it('加载失败时显示错误信息', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.reject(new Error('boom'));
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('仪表盘数据加载失败，请稍后再试')).toBeInTheDocument();
    });
  });

  it('无角色数据时显示空状态', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('暂无角色数据')).toBeInTheDocument();
    });
  });

  it('渲染角色名称、能量百分比和待办数', async () => {
    const status = makeStatus('r1', { roleName: '产品经理', energy: 80, pendingTasksCount: 5 });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('产品经理')).toBeInTheDocument();
    });
    expect(screen.getByText('80%')).toBeInTheDocument();
    expect(screen.getByText('5 待办')).toBeInTheDocument();
  });

  it('hasUrgent 为 true 时显示需关注标签', async () => {
    const status = makeStatus('r1', { hasUrgent: true });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('需关注')).toBeInTheDocument();
    });
  });

  it('hasUrgent 为 false 时不显示需关注标签', async () => {
    const status = makeStatus('r1', { hasUrgent: false });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.queryByText('需关注')).not.toBeInTheDocument();
    });
  });

  it('点击角色卡片触发 onViewChange 并传入 roleId', async () => {
    const onViewChange = vi.fn();
    const status = makeStatus('r1');
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={onViewChange} />);

    await waitFor(() => {
      expect(screen.getByText('角色r1')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('角色r1'));
    expect(onViewChange).toHaveBeenCalledWith('r1');
  });

  it('lastActiveAt 为 null 时显示暂无活动', async () => {
    const status = makeStatus('r1', { lastActiveAt: null });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('暂无活动')).toBeInTheDocument();
    });
  });
});
