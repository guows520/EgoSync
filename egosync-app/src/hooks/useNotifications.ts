import { useCallback, useEffect, useState } from 'react';
import { notificationService } from '../services/notificationService';
import { useTauriEvent } from './useTauriEvent';
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

  useTauriEvent<NotificationNewPayload>(
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
