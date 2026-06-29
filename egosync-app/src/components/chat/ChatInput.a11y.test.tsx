import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatInput } from './ChatInput';

describe('ChatInput 无障碍', () => {
  it('输入框应有 aria-label', () => {
    render(<ChatInput onSend={vi.fn()} />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('aria-label');
  });

  it('使用自定义 placeholder 时 aria-label 应反映 placeholder', () => {
    render(<ChatInput onSend={vi.fn()} placeholder="跟产品经理说点什么..." />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('aria-label', '跟产品经理说点什么...');
  });

  it('无 placeholder 时 aria-label 应使用默认值', () => {
    render(<ChatInput onSend={vi.fn()} />);
    const input = screen.getByRole('textbox');
    expect(input.getAttribute('aria-label')).toBeTruthy();
  });
});
