export interface Q2ReminderPayload {
  taskId: string;
  taskTitle: string;
  roleId: string | null;
  roleName: string | null;
  message: string;
  notificationId: string;
}
