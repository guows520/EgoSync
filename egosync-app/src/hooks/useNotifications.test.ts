import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useNotifications } from './useNotifications';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeNotification(id: string, level: 'whisper' | 'tap' | 'knock', isRead = false) {
  return {
    id,
    roleId: 'role-1',
    level,
    content: `通知${id}`,
    isRead,
    createdAt: '2026-06-20T00:00:00Z',
    roleName: '产品',
    roleIcon: '🎯',
    roleColor: '#6366F1',
  };
}

describe('useNotifications', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('初始加载通知列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'notification_list') {
        return Promise.resolve([makeNotification('n1', 'tap'), makeNotification('n2', 'knock')]);
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNotifications());

    await waitFor(() => {
      expect(result.current.notifications).toHaveLength(2);
    });
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('加载失败时设置错误并清空列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'notification_list') return Promise.reject(new Error('boom'));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNotifications());

    await waitFor(() => {
      expect(result.current.error).toBe('通知加载失败，请稍后再试');
    });
    expect(result.current.notifications).toHaveLength(0);
  });

  it('markAsRead 将对应通知标记为已读', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'notification_list') return Promise.resolve([makeNotification('n1', 'tap')]);
      if (cmd === 'notification_mark_read') return Promise.resolve(makeNotification('n1', 'tap', true));
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNotifications());

    await waitFor(() => {
      expect(result.current.notifications).toHaveLength(1);
    });

    await act(async () => {
      await result.current.markAsRead('n1');
    });

    expect(result.current.notifications[0].isRead).toBe(true);
  });

  it('alertUnreadCount 仅统计未读 tap/knock，whisperUnreadCount 仅统计未读 whisper', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'notification_list') {
        return Promise.resolve([
          makeNotification('n1', 'whisper'),
          makeNotification('n2', 'whisper'),
          makeNotification('n3', 'tap'),
          makeNotification('n4', 'knock'),
          makeNotification('n5', 'knock', true), // 已读不计入
        ]);
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useNotifications());

    await waitFor(() => {
      expect(result.current.notifications).toHaveLength(5);
    });

    expect(result.current.alertUnreadCount).toBe(2); // n3 tap + n4 knock
    expect(result.current.whisperUnreadCount).toBe(2); // n1 + n2
    expect(result.current.unreadCount).toBe(4); // 全部未读
  });
});
