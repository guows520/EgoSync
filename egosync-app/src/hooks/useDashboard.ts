import { useCallback, useEffect, useRef, useState } from 'react';
import { dashboardService } from '../services/dashboardService';
import { useEngineEvent } from './useEngineEvent';
import type { TransportReconnectedPayload } from '@/transport';
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

  // Story 16.2：重连恢复——dashboard_get_status（无参）在白名单内，直接
  // 消费重放结果集；statuses 引用更新会触发 metrics 依赖 effect 重拉
  // （dashboard_get_metrics 带 scope/时间范围参数，不在白名单——以重放
  // 为触发器定向重查）。重放缺失/失败回退主动重拉状态。
  useEngineEvent<TransportReconnectedPayload>(
    'transport:reconnected',
    useCallback((payload: TransportReconnectedPayload) => {
      const replayed = payload?.results?.['dashboard_get_status'];
      if (Array.isArray(replayed)) {
        setStatuses(replayed as DashboardStatus[]);
        statusesLoadedRef.current = true;
        // 评审修复：初载失败的错误横幅须随成功消费清除
        setError(null);
        return;
      }
      void dashboardService.getStatus()
        .then(items => {
          setStatuses(items ?? []);
          statusesLoadedRef.current = true;
          setError(null);
        })
        .catch(e => {
          console.error('重连后重拉仪表盘状态失败:', e);
          // 评审修复：与初载路径一致——重连后重拉失败不静默（NFR-C7）
          setError(DASHBOARD_LOAD_ERROR);
        });
    }, []),
    []
  );

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
