import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ProactivityToggle } from './ProactivityToggle';

describe('ProactivityToggle', () => {
  it('按传入 level 高亮并触发 onChange', () => {
    const onChange = vi.fn();

    render(<ProactivityToggle level="moderate" onChange={onChange} />);

    expect(screen.getByRole('button', { name: '适度建议' })).toHaveClass('text-indigo-600');
    fireEvent.click(screen.getByRole('button', { name: '积极主动' }));

    expect(onChange).toHaveBeenCalledWith('proactive');
  });

  it('disabled 时不触发变更', () => {
    const onChange = vi.fn();

    render(<ProactivityToggle level="passive" onChange={onChange} disabled />);

    fireEvent.click(screen.getByRole('button', { name: '适度建议' }));

    expect(onChange).not.toHaveBeenCalled();
  });
});
