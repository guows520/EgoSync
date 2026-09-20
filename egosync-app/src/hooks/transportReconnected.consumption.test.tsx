// Story 16.2：transport:reconnected 视图消费矩阵——真实行为钉。
//
// HttpTransport 重连后重放 31 条白名单命令（payload.results = {命令名: 结果}），
// 各视图 hook 的消费契约：
// - 白名单命中（数组结果）→ 直接消费，不重复查询；
// - 带/参不在白名单或重放缺失 → 回退主动重拉（service 调用）。
// Tauri 分支从不发射本事件（订阅即休眠，桌面零回归）——不在本文件覆盖。
//
// 本文件统一 mock useEngineEvent（按事件名捕获 handler 投递 payload）与
// 各 service 模块，逐 hook 验证「命中消费 / 缺失回退」两分支。

import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./useEngineEvent', () => ({
  useEngineEvent: vi.fn(),
}));

vi.mock('../services/notificationService', () => ({
  notificationService: {
    list: vi.fn(),
    markRead: vi.fn(),
    markAllRead: vi.fn(),
    clearAll: vi.fn(),
  },
}));

vi.mock('../services/taskService', () => ({
  taskService: {
    listByRole: vi.fn(),
    listButler: vi.fn(),
    listAll: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    reorder: vi.fn(),
    toggleComplete: vi.fn(),
    checkProtectionStatus: vi.fn(),
  },
}));

vi.mock('../services/dashboardService', () => ({
  dashboardService: {
    getStatus: vi.fn(),
    getMetrics: vi.fn(),
  },
}));

import { useEngineEvent } from './useEngineEvent';
import { useNotifications } from './useNotifications';
import { useTasks } from './useTasks';
import { useAllTasks } from './useAllTasks';
import { useDashboard } from './useDashboard';
import { notificationService } from '../services/notificationService';
import { taskService } from '../services/taskService';
import { dashboardService } from '../services/dashboardService';
import type { TransportReconnectedPayload } from '@/transport';
import type { CrossRoleTask, Task } from '../types/task';
import type { NotificationWithRole } from '../types/notification';
import type { DashboardStatus } from '../types/dashboard';

/** 按事件名捕获 useEngineEvent 注册的 handler，投递 transport:reconnected。 */
function deliverReconnected(payload: TransportReconnectedPayload) {
  const handler = vi.mocked(useEngineEvent).mock.calls.find(
    call => call[0] === 'transport:reconnected',
  )?.[1] as ((p: TransportReconnectedPayload) => void) | undefined;
  expect(handler).toBeDefined();
  act(() => handler!(payload));
}

function makeNotification(id: string): NotificationWithRole {
  return {
    id,
    roleId: 'role-1',
    level: 'tap',
    content: `通知${id}`,
    isRead: false,
    createdAt: '2026-06-20T00:00:00Z',
    roleName: '产品',
    roleIcon: '🎯',
    roleColor: '#6366F1',
  };
}

function makeTask(id: string, overrides: Partial<Task> = {}): CrossRoleTask {
  return {
    id,
    ownerType: 'butler',
    roleId: null,
    title: `任务${id}`,
    deadline: null,
    quadrant: 'Q1',
    isBigRock: false,
    isCompleted: false,
    completedAt: null,
    sortOrder: 0,
    protectionStatus: 'normal',
    confidence: null,
    manualOverride: false,
    classificationReason: null,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    deletedAt: null,
    roleName: null,
    roleColor: null,
    ...overrides,
  };
}

function makeStatus(roleId: string): DashboardStatus {
  return {
    roleId,
    roleName: `角色${roleId}`,
    roleIcon: 'target',
    roleColor: '#4F46E5',
    energy: 70,
    pendingTasksCount: 2,
    hasUrgent: false,
    lastActiveAt: '2026-06-20T00:00:00Z',
  };
}

describe('transport:reconnected 视图消费（Story 16.2）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(notificationService.list).mockResolvedValue([makeNotification('n-initial')]);
    vi.mocked(taskService.listButler).mockResolvedValue([makeTask('t-initial', { ownerType: 'butler' })]);
    vi.mocked(taskService.listAll).mockResolvedValue([makeTask('t-initial')]);
    vi.mocked(taskService.listByRole).mockResolvedValue([makeTask('t-initial', { ownerType: 'role', roleId: 'role-1' })]);
    vi.mocked(dashboardService.getStatus).mockResolvedValue([makeStatus('role-initial')]);
    vi.mocked(dashboardService.getMetrics).mockResolvedValue({
      taskCount: 1,
      memoryCount: 0,
      conversationCount: 0,
      pendingTaskCount: 0,
      generatedAt: '2026-06-20T00:00:00Z',
    });
  });

  it('useNotifications：白名单命中直接消费（不重复查询）', async () => {
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(result.current.notifications).toHaveLength(1));
    expect(notificationService.list).toHaveBeenCalledTimes(1);

    deliverReconnected({ results: { notification_list: [makeNotification('n1'), makeNotification('n2')] } });

    expect(result.current.notifications).toHaveLength(2);
    expect(result.current.notifications[0].id).toBe('n1');
    // 重放结果集直接消费——不重复查询
    expect(notificationService.list).toHaveBeenCalledTimes(1);
  });

  it('useNotifications：重放缺失回退主动重拉', async () => {
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(notificationService.list).toHaveBeenCalledTimes(1));

    vi.mocked(notificationService.list).mockResolvedValue([makeNotification('n-refreshed')]);
    deliverReconnected({ results: {} });

    await waitFor(() => expect(notificationService.list).toHaveBeenCalledTimes(2));
    expect(result.current.notifications[0].id).toBe('n-refreshed');
  });

  it('useTasks（butler scope）：task_list_butler 命中直接消费', async () => {
    const { result } = renderHook(() => useTasks({ ownerType: 'butler' }));
    await waitFor(() => expect(taskService.listButler).toHaveBeenCalledTimes(1));

    deliverReconnected({ results: { task_list_butler: [makeTask('t1', { ownerType: 'butler' })] } });

    expect(result.current.tasks).toHaveLength(1);
    expect(result.current.tasks[0].id).toBe('t1');
    expect(taskService.listButler).toHaveBeenCalledTimes(1);
  });

  it('useTasks（role scope 带参不在白名单）：bump reloadKey 回退重拉', async () => {
    const { result } = renderHook(() => useTasks({ ownerType: 'role', roleId: 'role-1' }));
    await waitFor(() => expect(taskService.listByRole).toHaveBeenCalledTimes(1));

    vi.mocked(taskService.listByRole).mockResolvedValue([makeTask('t-refreshed', { ownerType: 'role', roleId: 'role-1' })]);
    deliverReconnected({ results: {} });

    await waitFor(() => expect(taskService.listByRole).toHaveBeenCalledTimes(2));
    expect(result.current.tasks[0].id).toBe('t-refreshed');
  });

  it('useAllTasks（无过滤）：task_list_all 命中直接消费', async () => {
    const { result } = renderHook(() => useAllTasks());
    await waitFor(() => expect(taskService.listAll).toHaveBeenCalledTimes(1));

    deliverReconnected({ results: { task_list_all: [makeTask('t-all-1')] } });

    expect(result.current.tasks).toHaveLength(1);
    expect(result.current.tasks[0].id).toBe('t-all-1');
    expect(taskService.listAll).toHaveBeenCalledTimes(1);
  });

  it('useAllTasks（quadrant 过滤带参不在白名单）：回退重拉', async () => {
    const { result } = renderHook(() => useAllTasks({ quadrant: 'Q1' }));
    await waitFor(() => expect(taskService.listAll).toHaveBeenCalledTimes(1));

    vi.mocked(taskService.listAll).mockResolvedValue([makeTask('t-q1')]);
    deliverReconnected({ results: { task_list_all: [makeTask('t-not-matched', { quadrant: 'Q2' })] } });

    await waitFor(() => expect(taskService.listAll).toHaveBeenCalledTimes(2));
    expect(result.current.tasks[0].id).toBe('t-q1');
  });

  it('useDashboard：dashboard_get_status 命中直接消费并重拉带参指标', async () => {
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(dashboardService.getStatus).toHaveBeenCalledTimes(1));
    // 初始加载会因 statuses 引用更新触发两次带参指标查询（[] → 实际列表）
    await waitFor(() => expect(result.current.statuses).toHaveLength(1));
    const metricsCallsBeforeReconnect = vi.mocked(dashboardService.getMetrics).mock.calls.length;

    vi.mocked(dashboardService.getMetrics).mockResolvedValue({
      taskCount: 9,
      memoryCount: 1,
      conversationCount: 1,
      pendingTaskCount: 0,
      generatedAt: '2026-06-20T00:00:00Z',
    });
    deliverReconnected({ results: { dashboard_get_status: [makeStatus('role-1')] } });

    // 重放状态直接消费（不重复查询状态列表）
    expect(result.current.statuses).toHaveLength(1);
    expect(result.current.statuses[0].roleId).toBe('role-1');
    expect(dashboardService.getStatus).toHaveBeenCalledTimes(1);
    // 状态引用更新触发带参指标（dashboard_get_metrics）重拉
    await waitFor(() =>
      expect(vi.mocked(dashboardService.getMetrics).mock.calls.length)
        .toBeGreaterThan(metricsCallsBeforeReconnect),
    );
    await waitFor(() => expect(result.current.metrics?.taskCount).toBe(9));
  });

  it('useDashboard：重放缺失回退 getStatus 重拉', async () => {
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(dashboardService.getStatus).toHaveBeenCalledTimes(1));

    vi.mocked(dashboardService.getStatus).mockResolvedValue([makeStatus('role-fallback')]);
    deliverReconnected({ results: {} });

    await waitFor(() => expect(dashboardService.getStatus).toHaveBeenCalledTimes(2));
    expect(result.current.statuses[0].roleId).toBe('role-fallback');
  });

  // 评审修复（Story 16.2 Review）：初载失败的错误横幅须随重连成功消费清除
  // ——修复前重放结果直接 set 数据但不清 error，错误横幅与新鲜数据同屏常驻。
  it('useNotifications：初载失败后重放命中清错误横幅', async () => {
    vi.mocked(notificationService.list).mockRejectedValueOnce(new Error('down'));
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(result.current.error).not.toBeNull());

    deliverReconnected({ results: { notification_list: [makeNotification('n1')] } });

    expect(result.current.notifications[0].id).toBe('n1');
    expect(result.current.error).toBeNull();
  });

  it('useTasks（butler scope）：初载失败后重放命中清错误横幅', async () => {
    vi.mocked(taskService.listButler).mockRejectedValueOnce(new Error('down'));
    const { result } = renderHook(() => useTasks({ ownerType: 'butler' }));
    await waitFor(() => expect(result.current.error).not.toBeNull());

    deliverReconnected({ results: { task_list_butler: [makeTask('t1', { ownerType: 'butler' })] } });

    expect(result.current.tasks[0].id).toBe('t1');
    expect(result.current.error).toBeNull();
  });

  it('useAllTasks（无过滤）：初载失败后重放命中清错误横幅', async () => {
    vi.mocked(taskService.listAll).mockRejectedValueOnce(new Error('down'));
    const { result } = renderHook(() => useAllTasks());
    await waitFor(() => expect(result.current.error).not.toBeNull());

    deliverReconnected({ results: { task_list_all: [makeTask('t1')] } });

    expect(result.current.tasks[0].id).toBe('t1');
    expect(result.current.error).toBeNull();
  });

  it('useDashboard：初载失败后重放命中清错误横幅', async () => {
    vi.mocked(dashboardService.getStatus).mockRejectedValueOnce(new Error('down'));
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(result.current.error).not.toBeNull());

    deliverReconnected({ results: { dashboard_get_status: [makeStatus('role-1')] } });

    expect(result.current.statuses[0].roleId).toBe('role-1');
    expect(result.current.error).toBeNull();
  });

  it('useDashboard：重连后回退重拉失败不静默（与初载路径一致设错误）', async () => {
    const { result } = renderHook(() => useDashboard());
    await waitFor(() => expect(dashboardService.getStatus).toHaveBeenCalledTimes(1));

    // 重放缺失 + 回退重拉失败——不静默（NFR-C7）
    vi.mocked(dashboardService.getStatus).mockRejectedValueOnce(new Error('still down'));
    deliverReconnected({ results: {} });

    await waitFor(() => expect(result.current.error).not.toBeNull());
  });
});
