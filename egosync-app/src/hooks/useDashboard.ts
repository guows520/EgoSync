import { useEffect, useState } from 'react';
import { dashboardService } from '../services/dashboardService';
import type { DashboardStatus } from '../types/dashboard';

const DASHBOARD_LOAD_ERROR = '仪表盘数据加载失败，请稍后再试';

export function useDashboard() {
  const [statuses, setStatuses] = useState<DashboardStatus[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    void (async () => {
      try {
        const items = await dashboardService.getStatus();
        if (!cancelled) setStatuses(items ?? []);
      } catch (e) {
        console.error('加载仪表盘状态失败:', e);
        if (!cancelled) {
          setStatuses([]);
          setError(DASHBOARD_LOAD_ERROR);
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const clearError = () => setError(null);

  return { statuses, isLoading, error, clearError };
}
