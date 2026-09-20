import { useCallback, useEffect, useState } from 'react';
import { notificationService } from '../services/notificationService';
import { useEngineEvent } from './useEngineEvent';
import type { TransportReconnectedPayload } from '@/transport';
import type { NotificationNewPayload, NotificationWithRole } from '../types/notification';

const NOTIFICATION_LOAD_ERROR = '通知加载失败，请稍后再试';
const NOTIFICATION_MARK_READ_ERROR = '标记已读失败，请稍后再试';

export function useNotifications() {
  const [notifications, setNotifications] = useState<NotificationWithRole[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    void (async () => {
      try {
        const items = await notificationService.list();
        if (!cancelled) setNotifications(items ?? []);
      } catch (e) {
        console.error('加载通知失败:', e);
        if (!cancelled) {
          setNotifications([]);
          setError(NOTIFICATION_LOAD_ERROR);
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Story 16.2：重连恢复——优先直接消费白名单重放结果集
  // （notification_list 无参在白名单内，避免重复查询）；重放缺失/失败
  // （结果集内为错误对象）时回退主动重拉。写入类零重放（transport 契约）。
  useEngineEvent<TransportReconnectedPayload>(
    'transport:reconnected',
    useCallback((payload: TransportReconnectedPayload) => {
      const replayed = payload?.results?.['notification_list'];
      if (Array.isArray(replayed)) {
        setNotifications(replayed as NotificationWithRole[]);
        // 评审修复：初载失败的错误横幅须随成功消费清除（不与新鲜数据同屏）
        setError(null);
        return;
      }
      void notificationService.list()
        .then(items => setNotifications(items ?? []))
        .catch(e => {
          console.error('重连后重拉通知失败:', e);
          setError(NOTIFICATION_LOAD_ERROR);
        });
    }, []),
    []
  );

  useEngineEvent<NotificationNewPayload>(
    'notification:new',
    useCallback((payload: NotificationNewPayload) => {
      setNotifications(prev => {
        if (prev.some(n => n.id === payload.id)) return prev;
        const newNotification: NotificationWithRole = {
          id: payload.id,
          roleId: payload.roleId,
          level: payload.level,
          content: payload.content,
          isRead: false,
          createdAt: payload.createdAt,
          roleName: payload.roleName,
          roleIcon: payload.roleIcon,
          roleColor: payload.roleColor,
        };
        return [newNotification, ...prev];
      });
    }, []),
    []
  );

  const markAsRead = useCallback(async (id: string) => {
    try {
      await notificationService.markRead(id);
      setNotifications(prev =>
        prev.map(n => (n.id === id ? { ...n, isRead: true } : n))
      );
    } catch (e) {
      console.error(NOTIFICATION_MARK_READ_ERROR, e);
      setError(NOTIFICATION_MARK_READ_ERROR);
    }
  }, []);

  const unreadCount = notifications.filter(n => !n.isRead).length;
  // UX-DR20: 铃铛红点仅用于「轻触/敲门」未读；耳语不打断（侧边栏绿点）。
  const alertUnreadCount = notifications.filter(
    n => !n.isRead && (n.level === 'tap' || n.level === 'knock')
  ).length;
  const whisperUnreadCount = notifications.filter(
    n => !n.isRead && n.level === 'whisper'
  ).length;

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    notifications,
    unreadCount,
    alertUnreadCount,
    whisperUnreadCount,
    isLoading,
    error,
    markAsRead,
    clearError,
  };
}
