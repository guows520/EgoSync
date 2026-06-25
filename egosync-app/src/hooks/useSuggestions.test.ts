import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useSuggestions } from './useSuggestions';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeSuggestion(id: string) {
  return {
    id,
    roleId: 'role-1',
    title: `建议${id}`,
    content: '内容',
    priority: 'medium',
    status: 'pending',
    rejectionReason: null,
    convertedTaskId: null,
    conversationId: 'conv-1',
    createdAt: '2026-06-20T00:00:00Z',
    roleName: '产品',
    roleIcon: '🎯',
    roleColor: '#6366F1',
  };
}

describe('useSuggestions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('初始加载 pending 建议列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1'), makeSuggestion('s2')]);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(2);
    });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('确认建议成功但不立即移除（移除由动画结束驱动）', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1'), makeSuggestion('s2')]);
      if (cmd === 'suggestion_confirm') return Promise.resolve({ ...makeSuggestion('s1'), status: 'confirmed' });
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(2);
    });

    await act(async () => {
      await result.current.confirmSuggestion('s1');
    });

    expect(result.current.suggestions).toHaveLength(2);
  });

  it('拒绝建议成功但不立即移除（移除由动画结束驱动）', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1'), makeSuggestion('s2')]);
      if (cmd === 'suggestion_reject') return Promise.resolve({ ...makeSuggestion('s1'), status: 'rejected' });
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(2);
    });

    await act(async () => {
      await result.current.rejectSuggestion('s1', 'irrelevant');
    });

    expect(result.current.suggestions).toHaveLength(2);
  });

  it('removeSuggestion 从列表移除指定建议', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1'), makeSuggestion('s2')]);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(2);
    });

    await act(async () => {
      result.current.removeSuggestion('s1');
    });

    expect(result.current.suggestions).toHaveLength(1);
    expect(result.current.suggestions[0].id).toBe('s2');
  });

  it('确认失败时保留建议并设置 error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1')]);
      if (cmd === 'suggestion_confirm') return Promise.reject(new Error('confirm fail'));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(1);
    });

    await act(async () => {
      await expect(result.current.confirmSuggestion('s1')).rejects.toThrow('confirm fail');
    });

    expect(result.current.suggestions).toHaveLength(1);
    expect(result.current.error).not.toBeNull();
  });

  it('加载失败时设置 error 状态', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.reject(new Error('db error'));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.error).not.toBeNull();
    });
    expect(result.current.suggestions).toHaveLength(0);
    expect(result.current.isLoading).toBe(false);
  });

  it('refetch 触发重新加载', async () => {
    let callCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') {
        callCount++;
        return Promise.resolve([makeSuggestion(`s${callCount}`)]);
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions('conv-1'));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(1);
    });
    expect(result.current.suggestions[0].id).toBe('s1');

    await act(async () => {
      result.current.refetch();
    });

    await waitFor(() => {
      expect(result.current.suggestions[0].id).toBe('s2');
    });
  });

  it('conversationId 为 null 时不加载建议', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'suggestion_list_pending') return Promise.resolve([makeSuggestion('s1')]);
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useSuggestions(null));

    await waitFor(() => {
      expect(result.current.suggestions).toHaveLength(0);
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('suggestion_list_pending', expect.anything());
  });
});
