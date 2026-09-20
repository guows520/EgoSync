// Story 16.2：ConnectionStatus 三态呈现矩阵——真实行为钉。
//
// I/O 矩阵 + Design Notes「连接状态形态」决策：
// - online：常驻低显著度徽标（无横幅）；
// - connecting：短暂态徽标（无横幅、中性色）；
// - reconnecting：显著横幅（含「重连」中文文案）+ 徽标变色。

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';

const fakeUseConnectionState = vi.hoisted(() => vi.fn());
vi.mock('../../hooks/useConnectionState', () => ({
  useConnectionState: fakeUseConnectionState,
}));

import { ConnectionStatus } from './ConnectionStatus';

describe('ConnectionStatus（三态呈现矩阵）', () => {
  beforeEach(() => {
    fakeUseConnectionState.mockReset();
  });

  it('online：常驻低显著度徽标、无横幅', () => {
    fakeUseConnectionState.mockReturnValue('online');
    const { container } = render(<ConnectionStatus />);

    const badge = screen.getByTestId('connection-status');
    expect(badge).toHaveAttribute('data-state', 'online');
    expect(screen.queryByTestId('connection-banner')).toBeNull();
    expect(badge).toHaveTextContent('在线');
    // 低显著度：绿色系（不与警示色混淆）
    expect(badge.className).toContain('text-emerald-500/80');
    expect(container).toBeTruthy();
  });

  it('connecting：短暂态徽标（中性色）、无横幅', () => {
    fakeUseConnectionState.mockReturnValue('connecting');
    render(<ConnectionStatus />);

    const badge = screen.getByTestId('connection-status');
    expect(badge).toHaveAttribute('data-state', 'connecting');
    expect(screen.queryByTestId('connection-banner')).toBeNull();
    expect(badge).toHaveTextContent('连接中');
    expect(badge.className).toContain('text-slate-400');
  });

  it('reconnecting：显著横幅（重连中文文案）+ 徽标警示色', () => {
    fakeUseConnectionState.mockReturnValue('reconnecting');
    render(<ConnectionStatus />);

    const banner = screen.getByTestId('connection-banner');
    expect(banner).toBeInTheDocument();
    expect(banner).toHaveAttribute('role', 'status');
    // 断线诚实明示（NFR-C7）：横幅含「重连」中文文案
    expect(banner.textContent).toContain('重连');

    const badge = screen.getByTestId('connection-status');
    expect(badge).toHaveAttribute('data-state', 'reconnecting');
    expect(badge).toHaveTextContent('重连中');
    expect(badge.className).toContain('text-amber-500');
  });

  it('状态切换时徽标 data-state 随迁移更新（connecting → online）', () => {
    fakeUseConnectionState.mockReturnValue('connecting');
    const { rerender } = render(<ConnectionStatus />);
    expect(screen.getByTestId('connection-status')).toHaveAttribute('data-state', 'connecting');

    fakeUseConnectionState.mockReturnValue('online');
    rerender(<ConnectionStatus />);
    expect(screen.getByTestId('connection-status')).toHaveAttribute('data-state', 'online');
    expect(screen.queryByTestId('connection-banner')).toBeNull();
  });

  it('状态切换时横幅随迁移出现（online → reconnecting）', () => {
    fakeUseConnectionState.mockReturnValue('online');
    const { rerender } = render(<ConnectionStatus />);
    expect(screen.queryByTestId('connection-banner')).toBeNull();

    fakeUseConnectionState.mockReturnValue('reconnecting');
    rerender(<ConnectionStatus />);
    expect(screen.getByTestId('connection-banner')).toBeInTheDocument();
  });

  // 评审修复（Story 16.2 Review）：横幅不被通知面板遮挡 + iOS 刘海不压顶 +
  // 徽标读屏可播报（role=status）
  it('reconnecting 横幅与徽标的可达性/层级钉（评审修复）', () => {
    fakeUseConnectionState.mockReturnValue('reconnecting');
    render(<ConnectionStatus />);

    const banner = screen.getByTestId('connection-banner');
    // z-[60]：高于通知面板（z-50），面板开着时断线提示仍可见
    expect(banner.className).toContain('z-[60]');
    // top 侧安全区：viewport-fit=cover 下不压入 iOS 刘海/灵动岛
    expect(banner.className).toContain('pt-[max(0px,env(safe-area-inset-top))]');

    const badge = screen.getByTestId('connection-status');
    // 徽标 role=status：读屏播报连接状态（div+aria-label 不在无障碍树成话）
    expect(badge).toHaveAttribute('role', 'status');
  });
});
