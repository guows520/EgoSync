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

  it('默认显示日期筛选组合控件，不直接展示两个日期输入', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: '日期筛选' })).toHaveTextContent('全部日期');
    });
    expect(screen.queryByLabelText('自定义开始日期')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('自定义结束日期')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    const dialog = screen.getByRole('dialog', { name: '日期筛选选项' });
    expect(within(dialog).getByRole('button', { name: '全部日期' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: '最近3天' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: '最近7天' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: '最近1个月' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: '自定义时间' })).toBeInTheDocument();
  });

  it('选择最近3天和最近7天时发送本地自然日半开范围', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);
    await screen.findByRole('button', { name: '日期筛选' });

    const dayStart = new Date();
    dayStart.setHours(0, 0, 0, 0);
    const tomorrow = new Date(dayStart);
    tomorrow.setDate(tomorrow.getDate() + 1);
    const recent3Start = new Date(dayStart);
    recent3Start.setDate(recent3Start.getDate() - 2);
    const recent7Start = new Date(dayStart);
    recent7Start.setDate(recent7Start.getDate() - 6);

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '最近3天' }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({
          startAt: recent3Start.toISOString(),
          endAt: tomorrow.toISOString(),
        }),
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '最近7天' }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({
          startAt: recent7Start.toISOString(),
          endAt: tomorrow.toISOString(),
        }),
      });
    });
  });

  it('最近1个月按日历月回溯并保持结束边界为明天零点', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);
    await screen.findByRole('button', { name: '日期筛选' });

    const today = new Date();
    const day = today.getDate();
    const expectedEnd = new Date(today);
    expectedEnd.setHours(0, 0, 0, 0);
    expectedEnd.setDate(expectedEnd.getDate() + 1);
    const expectedStart = new Date(expectedEnd);
    expectedStart.setDate(1);
    expectedStart.setMonth(expectedStart.getMonth() - 1);
    const lastDay = new Date(expectedStart.getFullYear(), expectedStart.getMonth() + 1, 0).getDate();
    expectedStart.setDate(Math.min(day, lastDay));

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '最近1个月' }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({
          startAt: expectedStart.toISOString(),
          endAt: expectedEnd.toISOString(),
        }),
      });
    });
  });

  it('全部日期发送空边界，自定义时间只在应用时发送一次请求', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'dashboard_get_status') return Promise.resolve([makeStatus('r1')]);
      if (cmd === 'dashboard_get_metrics') return Promise.resolve(makeMetrics());
      return Promise.resolve(null);
    });

    render(<DashboardTab onViewChange={vi.fn()} />);
    await screen.findByRole('button', { name: '日期筛选' });

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '最近3天' }));
    await waitFor(() => expect(screen.getByRole('button', { name: '日期筛选' })).toHaveTextContent('最近3天'));

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '自定义时间' }));
    const startInput = screen.getByLabelText('自定义开始日期') as HTMLInputElement;
    const endInput = screen.getByLabelText('自定义结束日期') as HTMLInputElement;
    expect(startInput.value).not.toBe('');
    expect(endInput.value).not.toBe('');

    fireEvent.change(startInput, { target: { value: '2026-07-24' } });
    fireEvent.change(endInput, { target: { value: '2026-07-24' } });
    expect(screen.getByRole('button', { name: '应用日期' })).not.toBeDisabled();

    const metricsCallsBeforeApply = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'dashboard_get_metrics').length;
    fireEvent.click(screen.getByRole('button', { name: '应用日期' }));
    await waitFor(() => {
      const metricsCalls = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'dashboard_get_metrics');
      expect(metricsCalls.length).toBe(metricsCallsBeforeApply + 1);
      expect(mockInvoke).toHaveBeenCalledWith('dashboard_get_metrics', {
        query: expect.objectContaining({
          startAt: new Date(2026, 6, 24).toISOString(),
          endAt: new Date(2026, 6, 25).toISOString(),
        }),
      });
    });
    expect(screen.getByRole('button', { name: '日期筛选' })).toHaveTextContent('2026-07-24 至 2026-07-24');

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '自定义时间' }));
    fireEvent.change(screen.getByLabelText('自定义开始日期'), { target: { value: '2026-07-25' } });
    fireEvent.change(screen.getByLabelText('自定义结束日期'), { target: { value: '2026-07-24' } });
    expect(screen.getByText('结束日期不能早于开始日期')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '应用日期' })).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: '取消' }));
    expect(screen.getByRole('button', { name: '日期筛选' })).toHaveTextContent('2026-07-24 至 2026-07-24');

    fireEvent.click(screen.getByRole('button', { name: '日期筛选' }));
    fireEvent.click(screen.getByRole('button', { name: '全部日期' }));
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
