import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WeeklyReviewModal } from './WeeklyReviewModal';

vi.mock('../layout/Modal', () => ({
  Modal: ({ children, onClose }: any) => (
    <div data-testid="modal" onClick={onClose}>{children}</div>
  ),
}));

describe('WeeklyReviewModal', () => {
  it('initialPhase 默认为 review 阶段', () => {
    render(<WeeklyReviewModal roles={[]} onClose={vi.fn()} />);
    expect(screen.getByText('周复盘')).toBeDefined();
  });

  it('initialPhase="plan" 时初始显示规划阶段', () => {
    render(<WeeklyReviewModal roles={[]} onClose={vi.fn()} initialPhase="plan" />);
    expect(screen.getByText('规划下周大石头')).toBeDefined();
  });
});
