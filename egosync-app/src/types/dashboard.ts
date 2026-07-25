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

export type DashboardMetricsScope =
  | { type: 'all' }
  | { type: 'butler' }
  | { type: 'role'; roleId: string };

export interface DashboardMetricsQuery {
  scope: DashboardMetricsScope;
  startAt: string | null;
  endAt: string | null;
}

export interface DashboardMetrics {
  taskCount: number;
  memoryCount: number;
  conversationCount: number;
  pendingTaskCount: number;
  generatedAt: string;
}
