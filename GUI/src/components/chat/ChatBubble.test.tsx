import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Briefcase } from 'lucide-react';
import { ChatBubble } from './ChatBubble';
import type { ChatMessage } from '../../types/chat';

const assistantMsg: ChatMessage = {
  id: 'm1',
  conversationId: 'c1',
  role: 'assistant',
  content: '你好',
  thinkingContent: '',
  isComplete: true,
  createdAt: '2026-01-01T00:00:00Z',
  routingMetadata: null,
};

describe('ChatBubble assistant identity', () => {
  /// 修复回归: 管家视图下没有 role，气泡应当回退到默认「管家」标签，
  /// 这是 ButlerView 一直以来的真相，不能被角色视图修改污染。
  it('未传 assistantName 时回退到「管家」', () => {
    render(<ChatBubble message={assistantMsg} />);
    expect(screen.getByText('管家')).toBeInTheDocument();
  });

  /// 修复回归: 角色视图必须显示该角色的名字，否则用户进入「产品经理」
  /// 还看到「管家」头像，与 RoleHeader 的身份直接冲突，破坏 AC-1 的角色化体验。
  it('传入 assistantName/Icon/Color 时按角色身份渲染', () => {
    render(
      <ChatBubble
        message={assistantMsg}
        assistantName="产品经理"
        assistantIcon={Briefcase}
        assistantColor="#4F46E5"
      />,
    );
    expect(screen.getByText('产品经理')).toBeInTheDocument();
    expect(screen.queryByText('管家')).not.toBeInTheDocument();
  });
});