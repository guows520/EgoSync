import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
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

  it('已有文本的流式助手气泡不显示尾部光标', () => {
    const { container } = render(<ChatBubble message={{ ...assistantMsg, isComplete: false }} isStreaming />);

    expect(container.querySelector('span.inline-block.animate-pulse')).not.toBeInTheDocument();
  });

  it('空文本的流式助手气泡仍显示等待点', () => {
    const { container } = render(
      <ChatBubble message={{ ...assistantMsg, content: '', isComplete: false }} isStreaming />,
    );

    expect(container.querySelectorAll('.animate-bounce-forever')).toHaveLength(3);
  });

  it('助手消息中的记忆引用可点击并回传真实 ID，Markdown 仍正常渲染', () => {
    const onMemoryReferenceClick = vi.fn();

    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '**依据** [记忆#memory-1] 建议调整。' }}
        onMemoryReferenceClick={onMemoryReferenceClick}
      />,
    );

    expect(screen.getByText('依据')).toBeInTheDocument();
    const reference = screen.getByRole('button', { name: '打开记忆 memory-1' });

    fireEvent.click(reference);

    expect(reference).toHaveTextContent('[记忆#memory-1]');
    expect(onMemoryReferenceClick).toHaveBeenCalledWith('memory-1');
  });

  it('助手消息中的时间型记忆引用可点击并完整回传时间标签', () => {
    const onMemoryReferenceClick = vi.fn();

    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '依据 [记忆#2026/06/01 17:17] 判断。' }}
        onMemoryReferenceClick={onMemoryReferenceClick}
      />,
    );

    const reference = screen.getByRole('button', { name: '打开记忆 2026/06/01 17:17' });
    fireEvent.click(reference);

    expect(reference).toHaveTextContent('[记忆#2026/06/01 17:17]');
    expect(onMemoryReferenceClick).toHaveBeenCalledWith('2026/06/01 17:17');
  });

  it('助手消息中的内部记忆链接显示时间标签但点击回传真实记忆 ID', () => {
    const onMemoryReferenceClick = vi.fn();

    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '来源：[[记忆#2026/06/02 11:15]](egosync-memory://memory-fries)' }}
        onMemoryReferenceClick={onMemoryReferenceClick}
      />,
    );

    const reference = screen.getByRole('button', { name: '打开记忆 [记忆#2026/06/02 11:15]' });
    fireEvent.click(reference);

    expect(reference).toHaveTextContent('[记忆#2026/06/02 11:15]');
    expect(onMemoryReferenceClick).toHaveBeenCalledWith('memory-fries');
  });

  it('用户消息中的记忆引用保持普通文本且不触发跳转', () => {
    const onMemoryReferenceClick = vi.fn();

    render(
      <ChatBubble
        message={{ ...assistantMsg, role: 'user', content: '我提到 [记忆#memory-1]' }}
        onMemoryReferenceClick={onMemoryReferenceClick}
      />,
    );

    expect(screen.getByText('我提到 [记忆#memory-1]')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '打开记忆 memory-1' })).not.toBeInTheDocument();
    expect(onMemoryReferenceClick).not.toHaveBeenCalled();
  });

  it('assistant thinkingContent 不作为用户可见思考过程暴露', () => {
    render(<ChatBubble message={{ ...assistantMsg, thinkingContent: 'hidden chain of thought' }} />);

    expect(screen.queryByText('思考过程')).not.toBeInTheDocument();
    expect(screen.queryByText('hidden chain of thought')).not.toBeInTheDocument();
  });

  it('streaming thinking token 只显示等待点，不展示 raw thinking 文本', () => {
    const { container } = render(
      <ChatBubble
        message={{ ...assistantMsg, content: '', isComplete: false }}
        isStreaming
        streamingThinking="hidden streaming thought"
        isThinkingPhase
      />,
    );

    expect(container.querySelectorAll('.animate-bounce-forever')).toHaveLength(3);
    expect(screen.queryByText('hidden streaming thought')).not.toBeInTheDocument();
    expect(screen.queryByText('思考中...')).not.toBeInTheDocument();
  });
});