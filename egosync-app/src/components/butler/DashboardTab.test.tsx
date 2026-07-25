import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { DashboardTab } from './DashboardTab';
import type { DashboardStatus } from '../../types/dashboard';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../lib/roleIcons', () => ({
  getRoleIconComponent: () => () => null,
  normalizeColorHex: (c: string) => c || '#4F46E5',
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeStatus(id: string, overrides: Partial<DashboardStatus> = {}): DashboardStatus {
  return {
    roleId: id,
    roleName: `角色${id}`,
    roleIcon: 'target',
    roleColor: '#4F46E5',
    energy: 70,
    pendingTasksCount: 2,
    lastActiveAt: '2026-06-23T10:00:00Z',
    hasUrgent: false,
    ...overrides,
  };
}

function makeMetrics(overrides: Partial<any> = {}): any {
  return {
    taskCount: 10,
    memoryCount: 5,
    conversationCount: 3,
    pendingTaskCount: 2,
    generatedAt: '2026-07-24T10:00:00Z',
    ...overrides,
  };
}

describe('DashboardTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('加载中时显示加载提示', () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}));
    render(<DashboardTab onViewChange={vi.fn()} />);
    expect(screen.getByText('加载中…')).toBeInTheDocument();
  });

  it('加载失败时显示错误信息', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.reject(new Error('boom'));
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('仪表盘数据加载失败，请稍后再试')).toBeInTheDocument();
    });
  });

  it('无角色数据时显示空状态', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('暂无角色数据')).toBeInTheDocument();
    });
  });

  it('渲染角色名称、能量百分比和待办数', async () => {
    const status = makeStatus('r1', { roleName: '产品经理', energy: 80, pendingTasksCount: 5 });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getAllByText('产品经理').length).toBeGreaterThan(0);
    });
    expect(screen.getByText('80%')).toBeInTheDocument();
    expect(screen.getByText('5 待办')).toBeInTheDocument();
  });

  it('hasUrgent 为 true 时显示需关注标签', async () => {
    const status = makeStatus('r1', { hasUrgent: true });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('需关注')).toBeInTheDocument();
    });
  });

  it('hasUrgent 为 false 时不显示需关注标签', async () => {
    const status = makeStatus('r1', { hasUrgent: false });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.queryByText('需关注')).not.toBeInTheDocument();
    });
  });

  it('点击角色卡片触发 onViewChange 并传入 roleId', async () => {
    const onViewChange = vi.fn();
    const status = makeStatus('r1');
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={onViewChange} />);

    await waitFor(() => {
      expect(screen.getAllByText('角色r1').length).toBeGreaterThan(0);
    });

    // 点击角色状态卡按钮（不是 select 中的 option）
    const cards = screen.getAllByText('角色r1');
    const cardButton = cards.find(el => el.closest('button'));
    fireEvent.click(cardButton!);
    expect(onViewChange).toHaveBeenCalledWith('r1');
  });

  it('lastActiveAt 为 null 时显示暂无活动', async () => {
    const status = makeStatus('r1', { lastActiveAt: null });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('暂无活动')).toBeInTheDocument();
    });
  });

  it('渲染四张指标卡显示对应数值', async () => {
    const status = makeStatus('r1');
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics({ taskCount: 12, memoryCount: 7, conversationCount: 4, pendingTaskCount: 3 }));
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('12')).toBeInTheDocument();
    });
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getByText('4')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('指标加载中时显示省略号占位', async () => {
    const status = makeStatus('r1');
    let resolveMetrics: (val: any) => void = () => {};
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return new Promise((resolve) => { resolveMetrics = resolve; });
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getAllByText('…').length).toBeGreaterThan(0);
    });

    resolveMetrics(makeMetrics());
  });

  it('指标加载失败且无缓存时显示错误信息', async () => {
    const status = makeStatus('r1');
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.reject(new Error('metrics boom'));
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('活动统计加载失败，请稍后再试')).toBeInTheDocument();
    });
  });

  it('渲染 Agent 筛选器下拉框', async () => {
    const status = makeStatus('r1', { roleName: '产品经理' });
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByLabelText('Agent 筛选器')).toBeInTheDocument();
    });
    const select = screen.getByLabelText('Agent 筛选器') as HTMLSelectElement;
    expect(select).toBeInTheDocument();
    // 默认选中全部
    expect(select.value).toBe('all');
    // 包含全部、管家、产品经理三个选项
    const selectOptions = within(select);
    expect(selectOptions.getByText('全部')).toBeInTheDocument();
    expect(selectOptions.getByText('管家')).toBeInTheDocument();
    expect(selectOptions.getByText('产品经理')).toBeInTheDocument();
  });

  it('切换 Agent 筛选器触发重新加载', async () => {
    const status = makeStatus('r1', { roleName: '产品经理' });
    mockInvoke.mockImplementation((cmd: string, args: any) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') {
        if (args.query.scope.type === 'butler') return Promise.resolve(makeMetrics({ taskCount: 99 }));
        return Promise.resolve(makeMetrics({ taskCount: 10 }));
      }
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('10')).toBeInTheDocument();
    });

    const select = screen.getByLabelText('Agent 筛选器') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: 'butler' } });

    await waitFor(() => {
      expect(screen.getByText('99')).toBeInTheDocument();
    });
  });

  it('渲染时间范围选择器', async () => {
    const status = makeStatus('r1');
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([status]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByLabelText('开始日期')).toHaveAttribute('type', 'date');
      expect(screen.getByLabelText('结束日期')).toHaveAttribute('type', 'date');
    });
  });

  it('按自然日发送半开时间范围并正确回填结束日期', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    const startInput = await screen.findByLabelText('开始日期') as HTMLInputElement;
    const endInput = screen.getByLabelText('结束日期') as HTMLInputElement;
    fireEvent.change(startInput, { target: { value: '2026-07-24' } });
    fireEvent.change(endInput, { target: { value: '2026-07-24' } });

    const expectedStart = new Date(2026, 6, 24).toISOString();
    const expectedEnd = new Date(2026, 6, 25).toISOString();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({ startAt: expectedStart, endAt: expectedEnd }),
      });
    });
    expect(endInput.value).toBe('2026-07-24');
  });

  it('清空日期时发送 null 并保留另一侧边界', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    const startInput = await screen.findByLabelText('开始日期') as HTMLInputElement;
    const endInput = screen.getByLabelText('结束日期') as HTMLInputElement;
    fireEvent.change(startInput, { target: { value: '2026-07-24' } });
    fireEvent.change(endInput, { target: { value: '2026-07-24' } });

    const expectedEnd = new Date(2026, 6, 25).toISOString();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({ endAt: expectedEnd }),
      });
    });

    fireEvent.change(startInput, { target: { value: '' } });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({ startAt: null, endAt: expectedEnd }),
      });
    });

    fireEvent.change(endInput, { target: { value: '' } });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({ startAt: null, endAt: null }),
      });
    });
  });

  it('按目标文案和位置渲染四项指标', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    const region = await screen.findByLabelText('活动指标卡区域');
    expect(Array.from(region.children).map((card) => card.getAttribute('aria-label'))).toEqual([
      '任务总数',
      '记忆数量',
      '待处理任务',
      '对话数量',
    ]);
  });

});
