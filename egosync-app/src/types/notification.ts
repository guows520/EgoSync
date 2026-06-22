export interface Notification {
  id: string;
  roleId: string;
  level: 'whisper' | 'tap' | 'knock';
  content: string;
  isRead: boolean;
  createdAt: string;
}

export interface NotificationWithRole extends Notification {
  roleName: string;
  roleIcon: string;
  roleColor: string;
}

export interface NotificationNewPayload {
  id: string;
  level: 'whisper' | 'tap' | 'knock';
  content: string;
  roleId: string;
  roleName: string;
  roleIcon: string;
  roleColor: string;
  createdAt: string;
}
