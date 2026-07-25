import { useEffect, useRef, useState } from 'react';
import { dashboardService } from '../services/dashboardService';
import type { DashboardMetrics, DashboardMetricsScope, DashboardStatus } from '../types/dashboard';

const DASHBOARD_LOAD_ERROR = '仪表盘数据加载失败，请稍后再试';
const METRICS_LOAD_ERROR = '活动统计加载失败，请稍后再试';

export function useDashboard() {
  const [statuses, setStatuses] = useState<DashboardStatus[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null);
  const [metricsLoading, setMetricsLoading] = useState(false);
  const [metricsError, setMetricsError] = useState<string | null>(null);
  const [scope, setScope] = useState<DashboardMetricsScope>({ type: 'all' });
  const [timeRange, setTimeRange] = useState<{ startAt: string | null; endAt: string | null }>({
    startAt: null,
    endAt: null,
  });

  const requestIdRef = useRef(0);
  // 角色列表是否已完成首次加载，用于判断“失效角色回退”是否适用
  const statusesLoadedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    void (async () => {
      try {
        const items = await dashboardService.getStatus();
        if (!cancelled) {
          setStatuses(items ?? []);
          statusesLoadedRef.current = true;
        }
      } catch (e) {
        console.error('加载仪表盘状态失败:', e);
        if (!cancelled) {
          setStatuses([]);
          setError(DASHBOARD_LOAD_ERROR);
          statusesLoadedRef.current = true;
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    // AC-5 归档角色安全回退：角色列表加载完成后，若 scope 指向的角色不在活跃列表中，回退到 all
    if (
      statusesLoadedRef.current &&
      scope.type === 'role' &&
      !statuses.some((s) => s.roleId === scope.roleId)
    ) {
      setScope({ type: 'all' });
      return; // 不发送失效角色 ID 的请求
    }

    const currentRequestId = ++requestIdRef.current;
    let cancelled = false;
    setMetricsLoading(true);
    setMetricsError(null);

    void (async () => {
      try {
        const result = await dashboardService.getMetrics({ scope, ...timeRange });
        if (!cancelled && currentRequestId === requestIdRef.current) {
          setMetrics(result);
        }
      } catch (e) {
        console.error('加载活动统计失败:', e);
        if (!cancelled && currentRequestId === requestIdRef.current) {
          // AC-13 失败保留旧数据：无论是否已有 metrics，都设置错误；UI 负责区分首次 vs 刷新
          setMetricsError(METRICS_LOAD_ERROR);
        }
      } finally {
        if (!cancelled && currentRequestId === requestIdRef.current) {
          setMetricsLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [scope, timeRange, statuses]);

  const clearError = () => setError(null);

  return {
    statuses,
    isLoading,
    error,
    clearError,
    metrics,
    metricsLoading,
    metricsError,
    scope,
    setScope,
    timeRange,
    setTimeRange,
  };
}
