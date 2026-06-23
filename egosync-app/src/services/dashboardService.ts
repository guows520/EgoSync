import { invoke } from '@tauri-apps/api/core';
import type { DashboardStatus } from '../types/dashboard';

export const dashboardService = {
  getStatus: () => invoke<DashboardStatus[]>('dashboard_get_status'),
};
