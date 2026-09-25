// Story 16.4：ChatInput 移动端触控目标测试——发送/停止按钮 ≥44×44px
// （max-md:h-11 max-md:w-11；桌面 w-9 h-9 原值零变化，双声明共存）。

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatInput } from './ChatInput';

describe('ChatInput 移动端触控目标（Story 16.4）', () => {
  it('发送按钮在小屏放大到 44×44（max-md:h-11 max-md:w-11），桌面 w-9 h-9 不变', () => {
    render(<ChatInput onSend={vi.fn()} />);

    const sendButton = screen.getByRole('button', { name: '发送' });
    // 桌面原值保持（NFR-C4 逐像素零变化）
    expect(sendButton).toHaveClass('w-9', 'h-9');
    // 小屏触控 ≥44×44（11*4px=44px）
    expect(sendButton).toHaveClass('max-md:h-11', 'max-md:w-11');
  });

  it('停止按钮同样满足 ≥44×44 触控目标', () => {
    render(<ChatInput onSend={vi.fn()} isStreaming />);

    const stopButton = screen.getByRole('button', { name: '停止' });
    expect(stopButton).toHaveClass('w-9', 'h-9');
    expect(stopButton).toHaveClass('max-md:h-11', 'max-md:w-11');
  });

  it('输入区右留白 pr-14 容纳 44px 按钮（文本不压按钮）', () => {
    render(<ChatInput onSend={vi.fn()} />);

    const input = screen.getByRole('combobox');
    expect(input).toHaveClass('pr-14');
  });
});
