import { invoke } from '@tauri-apps/api/core';
import type { DashboardMetrics, DashboardMetricsQuery, DashboardStatus } from '../types/dashboard';

export const dashboardService = {
  getStatus: () => invoke<DashboardStatus[]>('dashboard_get_status'),
  getMetrics: (query: DashboardMetricsQuery) =>
    invoke<DashboardMetrics>('dashboard_get_metrics', { query }),
};
