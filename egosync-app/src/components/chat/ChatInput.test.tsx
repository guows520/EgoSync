import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatInput } from './ChatInput';

describe('ChatInput role accent', () => {
  /// AC-1 / AC-3: 角色视图的发送入口必须使用当前角色色温变量。
  /// 否则 App 虽然更新了 CSS 变量，用户仍然看不到角色色温切换。
  it('useRoleAccent 为 true 时发送按钮使用 --role-accent', () => {
    render(<ChatInput onSend={vi.fn()} useRoleAccent />);

    const button = screen.getByRole('button');
    expect(button).toHaveStyle({ backgroundColor: 'var(--role-accent)' });
  });
});