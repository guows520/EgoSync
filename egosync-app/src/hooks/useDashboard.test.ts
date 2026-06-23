import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useDashboard } from './useDashboard';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeStatus(id: string, overrides: Partial<any> = {}): any {
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

describe('useDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('初始加载仪表盘状态列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') {
        return Promise.resolve([makeStatus('r1'), makeStatus('r2')]);
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.statuses).toHaveLength(2);
    });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('加载失败时设置错误并清空列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.reject(new Error('boom'));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.error).toBe('仪表盘数据加载失败，请稍后再试');
    });
    expect(result.current.statuses).toHaveLength(0);
  });

  it('后端返回 null 时列表为空且无错误', async () => {
    mockInvoke.mockImplementation(() => Promise.resolve(null));

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
    expect(result.current.statuses).toHaveLength(0);
    expect(result.current.error).toBeNull();
  });
});
