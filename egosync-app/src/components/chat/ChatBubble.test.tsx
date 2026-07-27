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

  it('非流式气泡不默认播放入场动画', () => {
    const { container } = render(<ChatBubble message={assistantMsg} />);

    expect(container.firstElementChild).not.toHaveClass('animate-in');
    expect(container.firstElementChild).not.toHaveClass('slide-in-from-bottom-2');
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

  it('assistant thinkingContent 作为可折叠执行过程展示', () => {
    render(<ChatBubble message={{ ...assistantMsg, thinkingContent: '用户要求一个PPT文件转换为Markdown格式。' }} />);

    expect(screen.getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(screen.queryByText('用户要求一个PPT文件转换为Markdown格式。')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '执行过程' }));
    expect(screen.getByText('Think 思考时长未知')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Think 思考时长未知' }));
    expect(screen.getByTestId('thinking-content')).toHaveTextContent('用户要求一个PPT文件转换为Markdown格式。');
    expect(screen.getByRole('button', { name: '执行过程' }).parentElement).toHaveClass('w-full', 'max-w-[85%]');
    expect(screen.getByText('管家').parentElement?.parentElement).toHaveClass('w-full', 'max-w-[85%]');
  });

  it('streaming thinking token 由 ChatStream 统一执行过程承载', () => {
    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '', isComplete: false }}
        isStreaming
        streamingThinking="我应该使用skill工具来调用markitdown技能。"
        isThinkingPhase
      />,
    );

    expect(screen.queryByText('思考中...')).not.toBeInTheDocument();
    expect(screen.queryByText('我应该使用skill工具来调用markitdown技能。')).not.toBeInTheDocument();
  });

  it('工具状态归入顶部执行过程且不再显示查看处理过程入口', () => {
    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '', isComplete: false }}
        isStreaming
        streamStatus={{ phase: 'tool', statusText: '正在使用 find-skills...', toolName: 'find-skills' }}
      />,
    );

    expect(screen.getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(screen.getByText('正在使用 find-skills...')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '查看处理过程' })).not.toBeInTheDocument();
  });

  it('活跃 Think 使用固定三行视窗，并在内容更新时保持最新内容可见', () => {
    const renderBubble = (thinkingContent: string) => (
      <ChatBubble
        message={{ ...assistantMsg, content: '' }}
        executionTraceBlocks={[
          {
            id: 'thinking-active',
            type: 'thinking',
            content: thinkingContent,
            elapsedSeconds: 4,
            isActive: true,
          },
          {
            id: 'tool-after-thinking',
            type: 'action',
            actionType: 'shell',
            title: '执行后续命令',
            status: 'running',
          },
        ]}
      />
    );
    const { container, rerender } = render(renderBubble('第一行\n第二行\n第三行\n第四行\n第五行'));

    fireEvent.click(screen.getByRole('button', { name: '执行过程' }));

    expect(screen.getByText('Think 思考了4秒')).toBeInTheDocument();
    const viewport = screen.getByTestId('thinking-viewport');
    expect(viewport).toHaveClass('h-[4.5rem]', 'overflow-hidden', 'whitespace-pre-wrap');
    expect(viewport).toHaveTextContent(/第一行\s+第二行\s+第三行\s+第四行\s+第五行/);
    expect(container.textContent!.indexOf('第五行')).toBeLessThan(container.textContent!.indexOf('执行后续命令'));

    Object.defineProperty(viewport, 'scrollHeight', { configurable: true, value: 240 });
    rerender(renderBubble('第一行\n第二行\n第三行\n第四行\n第五行\n第六行'));

    expect(screen.getByTestId('thinking-viewport').scrollTop).toBe(240);
  });

  it('Think 完成后默认折叠，点击标题可查看完整内容', () => {
    render(
      <ChatBubble
        message={{ ...assistantMsg, content: '' }}
        executionTraceBlocks={[{
          id: 'thinking-completed',
          type: 'thinking',
          content: '第一行\n第二行\n第三行\n第四行',
          elapsedSeconds: 7,
          isActive: false,
        }]}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '执行过程' }));
    const thinkButton = screen.getByRole('button', { name: 'Think 思考了7秒' });
    expect(screen.queryByTestId('thinking-content')).not.toBeInTheDocument();

    fireEvent.click(thinkButton);
    expect(screen.getByTestId('thinking-content')).toHaveTextContent(/第一行\s+第二行\s+第三行\s+第四行/);
  });

  it('操作卡展开后只显示原始执行指令和实际执行结果', () => {
    render(
      <ChatBubble
        message={assistantMsg}
        executionTraceBlocks={[
          {
            id: 'event-shell',
            type: 'action',
            actionType: 'shell',
            title: '检查 markitdown 是否已安装',
            status: 'completed',
            details: [
              { label: 'Command', value: 'pip show markitdown 2>$null; if ($LASTEXITCODE -ne 0) { echo "NOT_INSTALLED" }' },
              { label: 'Output', value: 'Name: markitdown\nVersion: 0.1.6' },
            ],
          },
        ]}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: '执行过程' }));
    const actionCard = screen.getByRole('button', { name: /检查 markitdown 是否已安装/ });
    expect(actionCard).toBeInTheDocument();
    expect(screen.queryByText(/pip show markitdown/)).not.toBeInTheDocument();

    fireEvent.click(actionCard);

    expect(screen.getByText('$ pip show markitdown 2>$null; if ($LASTEXITCODE -ne 0) { echo "NOT_INSTALLED" }')).toBeInTheDocument();
    expect(screen.getByText(/Name: markitdown/)).toBeInTheDocument();
    expect(screen.getByText(/Version: 0\.1\.6/)).toBeInTheDocument();
    expect(screen.queryByText('Command')).not.toBeInTheDocument();
    expect(screen.queryByText('Input')).not.toBeInTheDocument();
    expect(screen.queryByText('Output')).not.toBeInTheDocument();
    expect(screen.queryByText(/"tool":"bash"/)).not.toBeInTheDocument();
  });
});

describe('ChatBubble GFM 表格', () => {
  it('将标准 GFM 表格渲染为语义化表格并保留中文、emoji、粗体和链接', () => {
    const { container } = render(
      <ChatBubble
        message={{
          ...assistantMsg,
          content: '| 城市 | 国家 | 特色 | 链接 |\n| --- | --- | --- | --- |\n| **成都** | 🇨🇳 中国 | 熊猫 | [详情](https://example.com) |',
        }}
      />,
    );

    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(container.querySelector('thead')).toBeInTheDocument();
    expect(container.querySelector('tbody')).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: '城市' })).toBeInTheDocument();
    expect(screen.getByText('成都').tagName).toBe('STRONG');
    expect(screen.getByText('🇨🇳 中国')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: '详情' })).toHaveAttribute('href', 'https://example.com');
  });

  it('代码块中的管道内容保持代码，不生成表格', () => {
    render(<ChatBubble message={{ ...assistantMsg, content: '```markdown\n| A | B |\n| --- | --- |\n| 1 | 2 |\n```' }} />);

    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByText(/\| A \| B \|/)).toBeInTheDocument();
  });

  it('TAB 分隔文本不做非标准表格转换', () => {
    render(<ChatBubble message={{ ...assistantMsg, content: '城市\t国家\n成都\t中国' }} />);

    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByText(/城市/)).toBeInTheDocument();
  });
});
