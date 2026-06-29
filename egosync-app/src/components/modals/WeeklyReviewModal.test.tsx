import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { WeeklyReviewModal } from './WeeklyReviewModal';
import type { Role } from '../../types/role';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../layout/Modal', () => ({
  Modal: ({ children, onClose }: any) => (
    <div data-testid="modal" onClick={onClose}>{children}</div>
  ),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

function makeRole(overrides: Partial<Role> = {}): Role {
  return {
    id: 'r1',
    name: '产品经理',
    icon: 'briefcase',
    color: '#4F46E5',
    goal: '做好产品',
    personalityPrompt: '',
    status: 'active',
    energy: 80,
    skillsConfig: '{}',
    proactivityLevel: 'moderate',
    archivedAt: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

function makeReview(overrides: Partial<any> = {}): any {
  return {
    id: 'rev-1',
    weekStart: '2026-06-23',
    weekEnd: '2026-06-29',
    summary: '本周你在产品维度有显著进展。',
    energyTrends: JSON.stringify({
      'r1': { energy: 85, energyUpdatedAt: '2026-06-25T08:00:00Z' },
    }),
    bigrockStatus: JSON.stringify([
      { id: 't1', title: '竞品分析', isCompleted: true, completedAt: '2026-06-25T10:00:00Z', roleName: '产品经理' },
      { id: 't2', title: '用户调研', isCompleted: false, completedAt: null, roleName: '产品经理' },
    ]),
    newMemoriesCount: 3,
    createdAt: '2026-06-28T10:00:00Z',
    ...overrides,
  };
}

describe('WeeklyReviewModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('复盘阶段渲染真实复盘摘要', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(makeReview());
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('本周你在产品维度有显著进展。')).toBeDefined();
    });
  });

  it('复盘阶段渲染大石头完成列表', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(makeReview());
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('竞品分析')).toBeDefined();
      expect(screen.getByText('用户调研')).toBeDefined();
    });
  });

  it('无复盘数据时显示空状态文案', async () => {
    mockInvoke.mockResolvedValue(null);

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('本周复盘尚未生成，可在设置中手动触发或等待自动生成')).toBeDefined();
    });
  });

  it('无复盘数据时仍可切换到规划阶段', async () => {
    mockInvoke.mockResolvedValue(null);

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('本周复盘尚未生成，可在设置中手动触发或等待自动生成')).toBeDefined();
    });

    fireEvent.click(screen.getByText('规划下周大石头'));
    expect(screen.getByText('为每个角色设定下周最重要的大石头，系统会在日程中优先为它们保留时间。')).toBeDefined();
  });

  it('规划阶段渲染角色卡片', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(null);
      if (cmd === 'review_get_bigrock_suggestions') return Promise.resolve([]);
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole(), makeRole({ id: 'r2', name: '家庭', color: '#F59E0B' })]} onClose={vi.fn()} initialPhase="plan" />);

    await waitFor(() => {
      expect(screen.getByText('产品经理')).toBeDefined();
      expect(screen.getByText('家庭')).toBeDefined();
    });
  });

  it('规划阶段 AI 建议加载中显示"正在思考建议..."', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(null);
      if (cmd === 'review_get_bigrock_suggestions') return new Promise(() => {}); // never resolves
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} initialPhase="plan" />);

    await waitFor(() => {
      expect(screen.getByText('正在思考建议...')).toBeDefined();
    });
  });

  it('规划阶段 AI 建议加载失败显示降级文案', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(null);
      if (cmd === 'review_get_bigrock_suggestions') return Promise.reject(new Error('LLM error'));
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={vi.fn()} initialPhase="plan" />);

    await waitFor(() => {
      expect(screen.getByText('手动填写本周大石头')).toBeDefined();
    });
  });

  it('确认规划调用 savePlan 并关闭 Modal', async () => {
    const onClose = vi.fn();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'review_get_by_week') return Promise.resolve(null);
      if (cmd === 'review_get_bigrock_suggestions') return Promise.resolve([]);
      if (cmd === 'review_plan_bigrocks') return Promise.resolve([{ id: 't1', isBigRock: true }]);
      return Promise.resolve(null);
    });

    render(<WeeklyReviewModal roles={[makeRole()]} onClose={onClose} initialPhase="plan" />);

    await waitFor(() => {
      expect(screen.getByPlaceholderText('产品经理 下周重要的事...')).toBeDefined();
    });

    fireEvent.change(screen.getByPlaceholderText('产品经理 下周重要的事...'), { target: { value: 'Q3路线图定稿' } });
    fireEvent.click(screen.getByRole('button', { name: '确认规划' }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });
});

