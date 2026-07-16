import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useTaskDecompositions } from './useTaskDecompositions';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;
const proposal = {
  id: 'proposal-1',
  roleId: 'role-1',
  sourceConversationId: 'conv-1',
  taskSummary: '安排家长会',
  items: [{ title: '准备材料', deadline: null }, { title: '联系老师', deadline: null }],
  status: 'pending',
  createdAt: '2026-07-16T10:00:00Z',
  resolvedAt: null,
  roleName: '家庭',
  roleIcon: 'home',
  roleColor: '#6366F1',
};

describe('useTaskDecompositions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('仅加载来源管家会话的 pending 提案', async () => {
    mockInvoke.mockResolvedValueOnce([proposal]);
    const { result } = renderHook(() => useTaskDecompositions('conv-1'));

    await waitFor(() => expect(result.current.proposals).toHaveLength(1));
    expect(mockInvoke).toHaveBeenCalledWith('task_decomposition_list_pending', { conversationId: 'conv-1' });
  });

  it('接受拆分成功后从当前会话移除卡片', async () => {
    mockInvoke
      .mockResolvedValueOnce([proposal])
      .mockResolvedValueOnce({ ...proposal, status: 'accepted' });
    const { result } = renderHook(() => useTaskDecompositions('conv-1'));
    await waitFor(() => expect(result.current.proposals).toHaveLength(1));

    await act(async () => {
      await result.current.acceptProposal('proposal-1');
    });

    expect(mockInvoke).toHaveBeenCalledWith('task_decomposition_accept', { id: 'proposal-1' });
    expect(result.current.proposals).toHaveLength(0);
  });

  it('操作失败时保留 pending 提案供重试', async () => {
    mockInvoke
      .mockResolvedValueOnce([proposal])
      .mockRejectedValueOnce(new Error('failed'));
    const { result } = renderHook(() => useTaskDecompositions('conv-1'));
    await waitFor(() => expect(result.current.proposals).toHaveLength(1));

    await act(async () => {
      await expect(result.current.keepSingleProposal('proposal-1')).rejects.toThrow('failed');
    });

    expect(result.current.proposals).toHaveLength(1);
    expect(result.current.error).not.toBeNull();
  });
});
