import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useWeeklyReview } from './useWeeklyReview';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeReview(overrides: Partial<any> = {}): any {
  return {
    id: 'rev-1',
    weekStart: '2026-06-23',
    weekEnd: '2026-06-29',
    summary: '本周你在3个维度都有进展。',
    energyTrends: JSON.stringify({
      'r1': { energy: 85, energyUpdatedAt: '2026-06-25T08:00:00Z' },
      'r2': { energy: 60 },
    }),
    bigrockStatus: JSON.stringify([
      { id: 't1', title: '竞品分析', isCompleted: true, completedAt: '2026-06-25T10:00:00Z', roleName: '产品经理' },
      { id: 't2', title: '用户调研', isCompleted: false, completedAt: null, roleName: '产品经理' },
    ]),
    newMemoriesCount: 5,
    createdAt: '2026-06-28T10:00:00Z',
    ...overrides,
  };
}

describe('useWeeklyReview', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('weekStart 为 null 时不加载数据', () => {
    const { result } = renderHook(() => useWeeklyReview(null));
    expect(result.current.review).toBeNull();
    expect(result.current.isLoading).toBe(false);
    expect(result.current.energyTrendsData).toBeNull();
    expect(result.current.bigrockStatusList).toBeNull();
  });

  it('正常加载复盘数据并解析 JSON', async () => {
    mockInvoke.mockResolvedValue(makeReview());

    const { result } = renderHook(() => useWeeklyReview('2026-06-23'));

    await waitFor(() => {
      expect(result.current.review).not.toBeNull();
    });

    expect(result.current.review?.summary).toBe('本周你在3个维度都有进展。');
    expect(result.current.energyTrendsData).not.toBeNull();
    expect(result.current.energyTrendsData!['r1'].energy).toBe(85);
    expect(result.current.energyTrendsData!['r2'].energy).toBe(60);
    expect(result.current.bigrockStatusList).toHaveLength(2);
    expect(result.current.bigrockStatusList![0].title).toBe('竞品分析');
    expect(result.current.bigrockStatusList![0].isCompleted).toBe(true);
    expect(result.current.bigrockStatusList![1].isCompleted).toBe(false);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('后端返回 null 时 review 为 null 且不报错', async () => {
    mockInvoke.mockResolvedValue(null);

    const { result } = renderHook(() => useWeeklyReview('2026-06-23'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.review).toBeNull();
    expect(result.current.energyTrendsData).toBeNull();
    expect(result.current.bigrockStatusList).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('energyTrends JSON 格式非法时降级为 null', async () => {
    mockInvoke.mockResolvedValue(makeReview({ energyTrends: 'not-json' }));

    const { result } = renderHook(() => useWeeklyReview('2026-06-23'));

    await waitFor(() => {
      expect(result.current.review).not.toBeNull();
    });

    expect(result.current.energyTrendsData).toBeNull();
  });

  it('bigrockStatus JSON 格式非法时降级为 null', async () => {
    mockInvoke.mockResolvedValue(makeReview({ bigrockStatus: 'not-json' }));

    const { result } = renderHook(() => useWeeklyReview('2026-06-23'));

    await waitFor(() => {
      expect(result.current.review).not.toBeNull();
    });

    expect(result.current.bigrockStatusList).toBeNull();
  });

  it('invoke 失败时设置 error', async () => {
    mockInvoke.mockRejectedValue(new Error('network error'));

    const { result } = renderHook(() => useWeeklyReview('2026-06-23'));

    await waitFor(() => {
      expect(result.current.error).not.toBeNull();
    });

    expect(result.current.error).toBe('network error');
    expect(result.current.review).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });
});
