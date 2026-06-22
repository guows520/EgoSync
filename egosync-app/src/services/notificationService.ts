import { invoke } from '@tauri-apps/api/core';
import type { Notification, NotificationWithRole } from '../types/notification';

export const notificationService = {
  list: (): Promise<NotificationWithRole[]> =>
    invoke('notification_list'),

  create: (input: { roleId: string; level: string; content: string }): Promise<Notification> =>
    invoke('notification_create', { input }),

  markRead: (id: string): Promise<Notification> =>
    invoke('notification_mark_read', { id }),

  countUnread: (): Promise<number> =>
    invoke('notification_count_unread'),
};
