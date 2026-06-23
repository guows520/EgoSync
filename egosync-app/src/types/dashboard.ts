export interface DashboardStatus {
  roleId: string;
  roleName: string;
  roleIcon: string;
  roleColor: string;
  energy: number;
  pendingTasksCount: number;
  lastActiveAt: string | null;
  hasUrgent: boolean;
}
