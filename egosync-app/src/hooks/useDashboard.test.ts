import { renderHook, waitFor, act } from '@testing-library/react';
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

function makeMetrics(overrides: Partial<any> = {}): any {
  return {
    taskCount: 10,
    memoryCount: 5,
    conversationCount: 3,
    pendingTaskCount: 2,
    generatedAt: '2026-07-24T10:00:00Z',
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
      if (cmd === 'dashboard_get_metrics') {
        return Promise.resolve(makeMetrics());
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
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
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

  it('初始加载活动统计指标', async () => {
    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        expect(args.query.scope).toEqual({ type: 'all' });
        return Promise.resolve(makeMetrics({ taskCount: 15, memoryCount: 8 }));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.metrics).not.toBeNull();
    });
    expect(result.current.metrics?.taskCount).toBe(15);
    expect(result.current.metrics?.memoryCount).toBe(8);
    expect(result.current.metricsLoading).toBe(false);
    expect(result.current.metricsError).toBeNull();
  });

  it('切换 scope 后重新加载指标', async () => {
    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        if (args.query.scope.type === 'butler') {
          return Promise.resolve(makeMetrics({ taskCount: 3 }));
        }
        return Promise.resolve(makeMetrics({ taskCount: 15 }));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(15);
    });

    act(() => {
      result.current.setScope({ type: 'butler' });
    });

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(3);
    });
  });

  it('指标加载失败时设置 metricsError', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.reject(new Error('metrics boom'));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.metricsError).toBe('活动统计加载失败，请稍后再试');
    });
    expect(result.current.metrics).toBeNull();
    expect(result.current.metricsLoading).toBe(false);
  });

  it('设置时间范围后重新加载指标', async () => {
    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        if (args.query.startAt) {
          return Promise.resolve(makeMetrics({ taskCount: 5 }));
        }
        return Promise.resolve(makeMetrics({ taskCount: 15 }));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(15);
    });

    act(() => {
      result.current.setTimeRange({ startAt: '2026-06-01T00:00:00Z', endAt: null });
    });

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(5);
    });
  });

  it('AC-13 快速切换 scope 只接受最新结果', async () => {
    let resolveFirst: (val: any) => void = () => {};
    let resolveSecond: (val: any) => void = () => {};

    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        if (args.query.scope.type === 'butler') {
          return new Promise((resolve) => { resolveSecond = resolve; });
        }
        return new Promise((resolve) => { resolveFirst = resolve; });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    // 等待首次 all 请求发出（未 resolve）
    await waitFor(() => {
      expect(result.current.metricsLoading).toBe(true);
    });

    // 快速切换到 butler
    act(() => {
      result.current.setScope({ type: 'butler' });
    });

    // 旧请求（all）后完成 — 不应覆盖
    act(() => {
      resolveFirst(makeMetrics({ taskCount: 999 }));
    });

    // 新请求（butler）完成 — 应被接受
    act(() => {
      resolveSecond(makeMetrics({ taskCount: 42 }));
    });

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(42);
    });
    expect(result.current.metrics?.taskCount).not.toBe(999);
  });

  it('AC-13 刷新失败时保留上一次成功数据', async () => {
    let callCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve(makeMetrics({ taskCount: 20 }));
        }
        return Promise.reject(new Error('refresh boom'));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    await waitFor(() => {
      expect(result.current.metrics?.taskCount).toBe(20);
    });

    // 触发刷新（切换 scope）
    act(() => {
      result.current.setScope({ type: 'butler' });
    });

    await waitFor(() => {
      expect(result.current.metricsError).toBe('活动统计加载失败，请稍后再试');
    });
    // 旧数据应保留
    expect(result.current.metrics?.taskCount).toBe(20);
  });

  it('AC-5 归档角色回退到 all，不提交失效角色 ID', async () => {
    const metricsCalls: any[] = [];
    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') {
        metricsCalls.push(args.query.scope);
        return Promise.resolve(makeMetrics());
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboard());

    // 等待 statuses 加载完成
    await waitFor(() => {
      expect(result.current.statuses).toHaveLength(1);
    });

    // 设置 scope 为不存在的角色
    act(() => {
      result.current.setScope({ type: 'role', roleId: 'ghost' });
    });

    // 应自动回退到 all
    await waitFor(() => {
      expect(result.current.scope).toEqual({ type: 'all' });
    });

    // 不应有 role ghost 的请求被提交
    const hasGhostRequest = metricsCalls.some(
      (s) => s.type === 'role' && s.roleId === 'ghost'
    );
    expect(hasGhostRequest).toBe(false);
  });
});
