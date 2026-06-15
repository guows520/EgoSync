import { act, render, waitFor, screen, fireEvent, within } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ChatStream } from './ChatStream';
import { chatService } from '../../services/chatService';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import type { ChatMessage, StreamPayload } from '../../types/chat';
import type { Role } from '../../types/role';

vi.mock('../../services/chatService', () => ({
  chatService: {
    getButlerConversation: vi.fn(),
    getRoleConversation: vi.fn(),
    getConversation: vi.fn(),
    listConversations: vi.fn(),
    getHistory: vi.fn(),
    sendMessage: vi.fn(),
    deleteConversation: vi.fn(),
    newConversation: vi.fn(),
    stopStreaming: vi.fn(),
    pickWorkingDirectory: vi.fn(),
    getMessageProcessEvents: vi.fn(),
  },
}));

vi.mock('../../hooks/useTauriEvent', () => ({
  useTauriEvent: vi.fn(),
}));

const butlerConv = {
  id: 'conv-butler',
  roleId: null,
  title: '',
  startedAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const roleConv = {
  id: 'conv-role-1',
  roleId: 'role-1',
  title: '',
  startedAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const baseRole: Role = {
  id: 'role-1',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  goal: '打磨产品节奏',
  personalityPrompt: '',
  status: 'active',
  energy: 72,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

function chatMessage(overrides: Partial<ChatMessage>): ChatMessage {
  return {
    id: 'message-default',
    conversationId: 'conv-butler',
    role: 'user',
    content: '',
    thinkingContent: '',
    isComplete: true,
    createdAt: '2026-01-01T00:00:00Z',
    routingMetadata: null,
    ...overrides,
  };
}

type StreamHandler = (payload: StreamPayload) => void;

function captureStreamHandler() {
  let handler: StreamHandler | undefined;
  vi.mocked(useTauriEvent).mockImplementation((event, cb) => {
    if (event === 'llm:stream') {
      handler = cb as StreamHandler;
    }
  });
  return () => {
    if (!handler) throw new Error('llm:stream handler was not registered');
    return handler;
  };
}

describe('ChatStream conversation initialization (Story 2.2 AC-2 / AC-7)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(chatService.getHistory).mockResolvedValue([]);
    vi.mocked(chatService.listConversations).mockResolvedValue([]);
  });

  /// AC-2: 没有 role 时必须走管家会话；如果回退成角色会话或抛错，
  /// 管家视角就会"莫名其妙地接到某个角色的历史"，违反顶层场景的语义。
  it('role 为 null 时调用 chat_get_butler_conversation', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);

    render(<ChatStream role={null} />);

    await waitFor(() => {
      expect(chatService.getButlerConversation).toHaveBeenCalledTimes(1);
    });
    expect(chatService.getRoleConversation).not.toHaveBeenCalled();
    expect(chatService.listConversations).toHaveBeenCalledWith(undefined);
  });

  /// AC-2 / AC-6: 传入 role 时必须改用 chat_get_role_conversation 拿到属于该
  /// 角色的会话。这是"角色独立对话历史"的真相之锚 —— 走错就会让整个 Story 2.2
  /// 退化回 Story 2.1 之前的 mock 状态。
  it('role 非空时调用 chat_get_role_conversation 并按角色过滤历史列表', async () => {
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);

    render(<ChatStream role={baseRole} />);

    await waitFor(() => {
      expect(chatService.getRoleConversation).toHaveBeenCalledWith('role-1');
    });
    expect(chatService.getButlerConversation).not.toHaveBeenCalled();
    expect(chatService.listConversations).toHaveBeenCalledWith('role-1');
  });

  it('历史助手消息中的记忆引用点击后回传 memoryId', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({
        id: 'assistant-1',
        role: 'assistant',
        content: '依据 [记忆#memory-1] 建议调整。',
      }),
    ]);
    const onMemoryReferenceClick = vi.fn();

    render(<ChatStream role={null} onMemoryReferenceClick={onMemoryReferenceClick} />);

    const reference = await screen.findByRole('button', { name: '打开记忆 memory-1' });
    fireEvent.click(reference);

    expect(onMemoryReferenceClick).toHaveBeenCalledWith('memory-1');
  });

  it('streaming 完成后的助手消息记忆引用点击后回传 memoryId', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '为什么' }));
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);
    const onMemoryReferenceClick = vi.fn();

    render(<ChatStream role={null} onMemoryReferenceClick={onMemoryReferenceClick} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());
    fireEvent.change(screen.getByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...'), { target: { value: '为什么' } });
    fireEvent.click(screen.getByRole('button', { name: '发送' }));
    await waitFor(() => expect(chatService.sendMessage).toHaveBeenCalled());

    act(() => {
      const handler = getStreamHandler();
      handler({ conversationId: 'conv-butler', token: '依据 [记忆#memory-2]', done: false, thinking: false });
      handler({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });

    const reference = await screen.findByRole('button', { name: '打开记忆 memory-2' });
    fireEvent.click(reference);

    expect(onMemoryReferenceClick).toHaveBeenCalledWith('memory-2');
  });

  it('thinking token 不直接展示为执行过程或正文链接', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);

    render(<ChatStream role={null} onMemoryReferenceClick={vi.fn()} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    act(() => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '[记忆#memory-hidden]', done: false, thinking: true });
    });

    expect(screen.queryByText('[记忆#memory-hidden]')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '打开记忆 memory-hidden' })).not.toBeInTheDocument();
  });

  it('tool 状态显示通用工具文案且不生成正文链接', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);

    render(<ChatStream role={null} onMemoryReferenceClick={vi.fn()} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    act(() => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'tool',
        statusText: '正在使用 find-skills...',
        toolName: 'find-skills',
      });
    });

    expect(screen.getByText('正在使用 find-skills...')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '打开记忆 find-skills' })).not.toBeInTheDocument();
  });

  it('流式 processEvent 使用历史执行过程同一格式展示描述和操作', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);

    render(<ChatStream role={null} onMemoryReferenceClick={vi.fn()} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    act(() => {
      const handler = getStreamHandler();
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'stream-narration-1',
          conversationId: 'conv-butler',
          messageId: 'assistant-1',
          opencodeSessionId: 'ses-1',
          eventType: 'narration',
          toolName: null,
          status: null,
          summary: '先检查 markitdown 是否已安装。',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'stream-narration-middle',
          conversationId: 'conv-butler',
          messageId: 'assistant-1',
          opencodeSessionId: 'ses-1',
          eventType: 'narration',
          toolName: null,
          status: null,
          summary: '中间描述必须保留在执行过程中。',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00.500Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'stream-shell-running',
          conversationId: 'conv-butler',
          messageId: 'assistant-1',
          opencodeSessionId: 'ses-1',
          eventType: 'tool',
          toolName: 'bash',
          status: 'running',
          summary: '正在使用 bash...',
          rawJson: JSON.stringify({ tool: 'bash', status: 'running', input: { command: 'pip show markitdown', description: '检查 markitdown 是否已安装' } }),
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:01Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'stream-shell-completed',
          conversationId: 'conv-butler',
          messageId: 'assistant-1',
          opencodeSessionId: 'ses-1',
          eventType: 'tool',
          toolName: 'bash',
          status: 'completed',
          summary: 'bash 已完成，正在整理结果...',
          rawJson: JSON.stringify({ tool: 'bash', status: 'completed', input: { command: 'pip show markitdown', description: '检查 markitdown 是否已安装' }, output: 'Name: markitdown\nVersion: 0.1.6' }),
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:02Z',
        },
      } as StreamPayload);
    });

    expect(screen.getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(screen.getByText('先检查 markitdown 是否已安装。')).toBeInTheDocument();
    expect(screen.getByText('中间描述必须保留在执行过程中。')).toBeInTheDocument();
    const shellCard = screen.getByRole('button', { name: /检查 markitdown 是否已安装/ });
    expect(shellCard).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /正在使用 bash/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /bash 已完成/ })).not.toBeInTheDocument();

    fireEvent.click(shellCard);

    expect(screen.getByText('$ pip show markitdown')).toBeInTheDocument();
    expect(screen.getByText(/Name: markitdown/)).toBeInTheDocument();
    expect(screen.queryByText('Command')).not.toBeInTheDocument();
    expect(screen.queryByText('Output')).not.toBeInTheDocument();
  });

  it('工作目录入口在输入区左下角保持低频展示', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);

    render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    expect(screen.queryByText('工作目录')).not.toBeInTheDocument();
    expect(screen.queryByText('选择工作目录')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '选择工作目录' })).toHaveTextContent('默认目录 · 选择');
  });

  it('选择工作目录后发送消息会携带该目录', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.pickWorkingDirectory).mockResolvedValue('D:\\Workspace\\CaseA');
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '处理这个项目' }));
    vi.mocked(chatService.getHistory).mockResolvedValue([]);

    render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    fireEvent.click(await screen.findByRole('button', { name: '选择工作目录' }));
    expect(await screen.findByText(/CaseA/)).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...'), { target: { value: '处理这个项目' } });
    fireEvent.click(screen.getByRole('button', { name: '发送' }));

    await waitFor(() => {
      expect(chatService.sendMessage).toHaveBeenCalledWith(expect.objectContaining({
        content: '处理这个项目',
        workingDirectory: 'D:\\Workspace\\CaseA',
      }));
    });
  });

  it('处理结束后的助手消息在顶部统一显示执行过程且默认折叠', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({ id: 'assistant-1', role: 'assistant', content: '处理完成。', thinkingContent: '先检查 markitdown 是否已安装。' }),
    ]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([
      {
        id: 'event-1',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'markitdown',
        status: 'completed',
        summary: 'markitdown 已完成',
        rawJson: '{"tool":"markitdown"}',
        workingDirectory: 'D:\\Workspace\\CaseA',
        createdAt: '2026-01-01T00:00:00Z',
      },
    ]);

    render(<ChatStream role={null} />);

    const message = await screen.findByTestId('chat-message-assistant-1');
    expect(within(message).getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: '思考过程' })).not.toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: '查看处理过程' })).not.toBeInTheDocument();
    expect(within(message).queryByText('先检查 markitdown 是否已安装。')).not.toBeInTheDocument();

    fireEvent.click(within(message).getByRole('button', { name: '执行过程' }));

    expect(within(message).queryByText('先检查 markitdown 是否已安装。')).not.toBeInTheDocument();
    expect(await within(message).findByText('markitdown 已完成')).toBeInTheDocument();
    expect(within(message).queryByText(/工作目录/)).not.toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: '处理过程' })).not.toBeInTheDocument();
  });

  it('历史执行过程按叙述和操作交替展示且操作展开为原始指令和结果', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({
        id: 'assistant-1',
        role: 'assistant',
        content: '转换完成，文件已生成：\nD:\\移动云盘同步盘\\AI应用场景\\微软\\职能\\Microsoft-Copilot-scenarios-for-Marketing.md',
        thinkingContent: '用户要求一个PPT文件转换为Markdown格式。我应该使用skill工具来调用markitdown技能。',
      }),
    ]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([
      {
        id: 'narration-1',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'narration',
        toolName: null,
        status: null,
        summary: '先加载 markitdown 转换能力，然后检查本机是否已安装转换工具。',
        rawJson: '{}',
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:00Z',
      },
      {
        id: 'skill-running',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'skill',
        status: 'running',
        summary: '正在使用 skill...',
        rawJson: JSON.stringify({ tool: 'skill', status: 'running', input: { name: 'markitdown' } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:01Z',
      },
      {
        id: 'skill-completed',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'skill',
        status: 'completed',
        summary: 'skill 已完成，正在整理结果...',
        rawJson: JSON.stringify({ tool: 'skill', status: 'completed', input: { name: 'markitdown' }, output: '<skill_content name="markitdown">long docs</skill_content>' }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:02Z',
      },
      {
        id: 'narration-2',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'narration',
        toolName: null,
        status: null,
        summary: '确认 markitdown 已安装，可以继续转换。',
        rawJson: '{}',
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:03Z',
      },
      {
        id: 'shell-running',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'bash',
        status: 'running',
        summary: '正在使用 bash...',
        rawJson: JSON.stringify({ tool: 'bash', status: 'running', input: { command: 'pip show markitdown 2>$null; if ($LASTEXITCODE -ne 0) { echo "NOT_INSTALLED" }', description: '检查 markitdown 是否已安装' } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:04Z',
      },
      {
        id: 'shell-completed',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'bash',
        status: 'completed',
        summary: 'bash 已完成，正在整理结果...',
        rawJson: JSON.stringify({ tool: 'bash', status: 'completed', input: { command: 'pip show markitdown 2>$null; if ($LASTEXITCODE -ne 0) { echo "NOT_INSTALLED" }', description: '检查 markitdown 是否已安装' }, output: 'Name: markitdown\nVersion: 0.1.6' }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:05Z',
      },
      {
        id: 'narration-3',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'narration',
        toolName: null,
        status: null,
        summary: '确认 Markdown 文件已生成，并读取内容用于校验转换结果。',
        rawJson: '{}',
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:06Z',
      },
      {
        id: 'read-1',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: '读取文件',
        rawJson: JSON.stringify({ tool: 'read', status: 'completed', input: { file_path: 'D:\\移动云盘同步盘\\AI应用场景\\微软\\职能\\Microsoft-Copilot-scenarios-for-Marketing.md' } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:07Z',
      },
      {
        id: 'read-2',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: '继续读取文件',
        rawJson: JSON.stringify({ tool: 'read', status: 'completed', input: { file_path: 'D:\\移动云盘同步盘\\AI应用场景\\微软\\职能\\Microsoft-Copilot-scenarios-for-Marketing.md', offset: 120 } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:08Z',
      },
    ]);

    render(<ChatStream role={null} />);

    const message = await screen.findByTestId('chat-message-assistant-1');
    expect(within(message).getByText(/转换完成，文件已生成/)).toBeInTheDocument();
    fireEvent.click(within(message).getByRole('button', { name: '执行过程' }));

    expect(await within(message).findByText('先加载 markitdown 转换能力，然后检查本机是否已安装转换工具。')).toBeInTheDocument();
    expect(within(message).getByRole('button', { name: /加载 markitdown 转换能力/ })).toBeInTheDocument();
    expect(within(message).getByText('确认 markitdown 已安装，可以继续转换。')).toBeInTheDocument();
    const shellCard = within(message).getByRole('button', { name: /检查 markitdown 是否已安装/ });
    expect(shellCard).toBeInTheDocument();
    expect(within(message).getByText('确认 Markdown 文件已生成，并读取内容用于校验转换结果。')).toBeInTheDocument();
    expect(within(message).getByRole('button', { name: /^已探索 2 次读取$/ })).toBeInTheDocument();
    expect(within(message).queryByText('读取 Microsoft-Copilot-scenarios-for-Marketing.md')).not.toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: /已探索已探索/ })).not.toBeInTheDocument();
    expect(within(message).queryByText(/用户要求一个PPT文件转换为Markdown格式/)).not.toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: /正在使用 bash/ })).not.toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: /bash 已完成/ })).not.toBeInTheDocument();
    expect(within(message).queryByText(/<skill_content/)).not.toBeInTheDocument();

    fireEvent.click(shellCard);

    expect(within(message).getByText('$ pip show markitdown 2>$null; if ($LASTEXITCODE -ne 0) { echo "NOT_INSTALLED" }')).toBeInTheDocument();
    expect(within(message).getByText(/Name: markitdown/)).toBeInTheDocument();
    expect(within(message).getByText(/Version: 0\.1\.6/)).toBeInTheDocument();
    expect(within(message).queryByText('Command')).not.toBeInTheDocument();
    expect(within(message).queryByText('Input')).not.toBeInTheDocument();
    expect(within(message).queryByText('Output')).not.toBeInTheDocument();

    fireEvent.click(within(message).getByRole('button', { name: /已探索 2 次读取/ }));

    expect(within(message).getByText('读取 Microsoft-Copilot-scenarios-for-Marketing.md')).toBeInTheDocument();
    expect(within(message).getByText('读取 Microsoft-Copilot-scenarios-for-Marketing.md offset: 120')).toBeInTheDocument();
    const expandedReadButton = within(message).getByRole('button', { name: /已探索 2 次读取/ });
    expect(expandedReadButton.parentElement).not.toHaveTextContent(/D:\\移动云盘同步盘/);
  });

  it('历史 Shell 失败事件展开后显示命令输出和错误且不展示 rawJson', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({ id: 'assistant-1', role: 'assistant', content: '处理失败后已重试。' }),
    ]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([
      {
        id: 'event-shell-failed',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'bash',
        status: 'failed',
        summary: 'bash 失败',
        rawJson: JSON.stringify({
          tool: 'bash',
          state: {
            status: 'failed',
            input: { command: 'npm run test:frontend' },
            output: 'stdout text',
            error: 'exit code 1',
          },
        }),
        workingDirectory: 'D:\\Workspace\\CaseA',
        createdAt: '2026-01-01T00:00:00Z',
      },
    ]);

    render(<ChatStream role={null} />);

    const message = await screen.findByTestId('chat-message-assistant-1');
    fireEvent.click(within(message).getByRole('button', { name: '执行过程' }));
    fireEvent.click(await within(message).findByRole('button', { name: /bash 失败/ }));

    expect(within(message).getByText('$ npm run test:frontend')).toBeInTheDocument();
    expect(within(message).getByText('stdout text')).toBeInTheDocument();
    expect(within(message).getByText('exit code 1')).toBeInTheDocument();
    expect(within(message).queryByText('Command')).not.toBeInTheDocument();
    expect(within(message).queryByText('Output')).not.toBeInTheDocument();
    expect(within(message).queryByText('Error')).not.toBeInTheDocument();
    expect(within(message).queryByText(/"state"/)).not.toBeInTheDocument();
    expect(within(message).queryByText(/"tool":"bash"/)).not.toBeInTheDocument();
  });

  it('连续 read 历史事件合并为一个已探索读取卡片', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({ id: 'assistant-1', role: 'assistant', content: '读取完成。' }),
    ]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([
      {
        id: 'read-1',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: '读取第一段',
        rawJson: JSON.stringify({ tool: 'read', state: { status: 'completed', input: { file_path: 'a.md' } } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:00Z',
      },
      {
        id: 'read-2',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: '读取第二段',
        rawJson: JSON.stringify({ tool: 'read', state: { status: 'completed', input: { file_path: 'a.md', offset: 120 } } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:01Z',
      },
      {
        id: 'read-3',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: '读取第三段',
        rawJson: JSON.stringify({ tool: 'read', state: { status: 'completed', input: { file_path: 'b.md' } } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:02Z',
      },
    ]);

    render(<ChatStream role={null} />);

    const message = await screen.findByTestId('chat-message-assistant-1');
    fireEvent.click(await within(message).findByRole('button', { name: '执行过程' }));
    const readCard = await within(message).findByRole('button', { name: /^已探索 3 次读取$/ });
    expect(readCard).toBeInTheDocument();
    expect(within(message).queryByText('读取 a.md')).not.toBeInTheDocument();
    expect(within(message).queryByText('读取 b.md')).not.toBeInTheDocument();
    expect(within(message).queryByText('读取第一段')).not.toBeInTheDocument();
    expect(within(message).queryByRole('button', { name: /已探索已探索/ })).not.toBeInTheDocument();

    fireEvent.click(readCard);

    expect(within(message).getByText('读取 a.md')).toBeInTheDocument();
    expect(within(message).getByText('读取 a.md offset: 120')).toBeInTheDocument();
    expect(within(message).getByText('读取 b.md')).toBeInTheDocument();
  });

  it('read 历史事件从 rawPart 深层参数提取文件名且缺失时不显示状态文案', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({ id: 'assistant-1', role: 'assistant', content: '读取完成。' }),
    ]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([
      {
        id: 'read-rawpart-filepath',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'running',
        summary: '正在使用 read...',
        rawJson: JSON.stringify({
          tool: 'read',
          status: 'running',
          rawPart: {
            state: {
              input: {
                filePath: 'D:\\移动云盘同步盘\\AI应用场景\\微软\\职能\\Microsoft-Copilot-scenarios-for-Marketing.md',
              },
            },
          },
        }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:00Z',
      },
      {
        id: 'read-missing-file',
        conversationId: 'conv-butler',
        messageId: 'assistant-1',
        opencodeSessionId: 'ses-1',
        eventType: 'tool',
        toolName: 'read',
        status: 'completed',
        summary: 'read 已完成，正在整理结果...',
        rawJson: JSON.stringify({ tool: 'read', status: 'completed', rawPart: { state: { input: {} } } }),
        workingDirectory: null,
        createdAt: '2026-01-01T00:00:01Z',
      },
    ]);

    render(<ChatStream role={null} />);

    const message = await screen.findByTestId('chat-message-assistant-1');
    fireEvent.click(await within(message).findByRole('button', { name: '执行过程' }));

    const readCard = await within(message).findByRole('button', { name: /^已探索 2 次读取$/ });
    expect(readCard).toBeInTheDocument();
    expect(within(message).queryByText('读取 Microsoft-Copilot-scenarios-for-Marketing.md')).not.toBeInTheDocument();
    expect(within(message).queryByText('读取文件')).not.toBeInTheDocument();

    fireEvent.click(readCard);

    expect(within(message).getByText('读取 Microsoft-Copilot-scenarios-for-Marketing.md')).toBeInTheDocument();
    expect(within(message).getByText('读取文件')).toBeInTheDocument();
    expect(within(message).queryByText('读取 正在使用 read...')).not.toBeInTheDocument();
    expect(within(message).queryByText('读取 read 已完成，正在整理结果...')).not.toBeInTheDocument();
  });

  it('多个历史消息的执行过程异步返回时只更新各自消息', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([
      chatMessage({ id: 'assistant-1', role: 'assistant', content: '第一条。' }),
      chatMessage({ id: 'assistant-2', role: 'assistant', content: '第二条。' }),
    ]);
    let resolveFirst: (events: Awaited<ReturnType<typeof chatService.getMessageProcessEvents>>) => void = () => {};
    vi.mocked(chatService.getMessageProcessEvents).mockImplementation(messageId => {
      if (messageId === 'assistant-1') {
        return new Promise(resolve => {
          resolveFirst = resolve;
        });
      }
      return Promise.resolve([
        {
          id: 'event-2',
          conversationId: 'conv-butler',
          messageId: 'assistant-2',
          opencodeSessionId: 'ses-2',
          eventType: 'tool',
          toolName: 'skill-b',
          status: 'completed',
          summary: '第二条处理完成',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00Z',
        },
      ]);
    });

    render(<ChatStream role={null} />);
    const first = await screen.findByTestId('chat-message-assistant-1');
    const second = await screen.findByTestId('chat-message-assistant-2');

    fireEvent.click(await within(second).findByRole('button', { name: '执行过程' }));
    expect(await within(second).findByText('第二条处理完成')).toBeInTheDocument();
    expect(within(first).queryByText('第二条处理完成')).not.toBeInTheDocument();

    await act(async () => {
      resolveFirst([
        {
          id: 'event-1',
          conversationId: 'conv-butler',
          messageId: 'assistant-1',
          opencodeSessionId: 'ses-1',
          eventType: 'tool',
          toolName: 'skill-a',
          status: 'completed',
          summary: '第一条处理完成',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00Z',
        },
      ]);
    });

    fireEvent.click(within(first).getByRole('button', { name: '执行过程' }));
    expect(await within(first).findByText('第一条处理完成')).toBeInTheDocument();
    expect(within(second).queryByText('第一条处理完成')).not.toBeInTheDocument();
  });

  it('streaming tool 状态出现时在消息顶部执行过程中默认展开', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getHistory).mockResolvedValue([]);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([]);

    render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    act(() => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'tool',
        statusText: '正在使用 markitdown...',
        toolName: 'markitdown',
      });
    });

    expect(screen.getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(screen.getByText('正在使用 markitdown...')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '查看处理过程' })).not.toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: '处理过程' })).not.toBeInTheDocument();
  });

  it('来源导航目标存在时切换到目标对话并高亮来源消息', async () => {
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = scrollIntoView;
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getConversation).mockResolvedValue({
      ...butlerConv,
      id: 'conv-source',
      title: '来源对话',
    });
    vi.mocked(chatService.getHistory).mockImplementation(async conversationId => {
      if (conversationId === 'conv-source') {
        return [chatMessage({ id: 'msg-source', conversationId: 'conv-source', role: 'user', content: '来源上下文' })];
      }
      return [];
    });
    const onSourceNavigationHandled = vi.fn();

    try {
      Object.defineProperty(HTMLElement.prototype, 'scrollHeight', { configurable: true, get: () => 5000 });
      render(
        <ChatStream
          role={null}
          sourceNavigationTarget={{ conversationId: 'conv-source', messageId: 'msg-source', roleId: null }}
          onSourceNavigationHandled={onSourceNavigationHandled}
        />,
      );

      const sourceMessage = await screen.findByTestId('chat-message-msg-source');
      Object.defineProperty(sourceMessage, 'offsetTop', { configurable: true, value: 1200 });
      Object.defineProperty(sourceMessage, 'clientHeight', { configurable: true, value: 100 });
      const scrollContainer = screen.getByTestId('chat-scroll-container');
      Object.defineProperty(scrollContainer, 'clientHeight', { configurable: true, value: 400 });
      expect(sourceMessage).toHaveClass('ring-2');
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalledWith({ behavior: 'smooth', block: 'center' }));
      await waitFor(() => expect(scrollContainer.scrollTop).toBe(1050));
      expect(scrollContainer.scrollTop).not.toBe(5000);
      expect(onSourceNavigationHandled).toHaveBeenCalledTimes(1);
    } finally {
      delete (HTMLElement.prototype as any).scrollHeight;
      Element.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it('来源对话已删除时显示温和提示且不切换历史', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getConversation).mockResolvedValue(null);
    const onSourceNavigationHandled = vi.fn();

    render(
      <ChatStream
        role={null}
        sourceNavigationTarget={{ conversationId: 'deleted-conv', messageId: 'missing-msg', roleId: null }}
        onSourceNavigationHandled={onSourceNavigationHandled}
      />,
    );

    expect(await screen.findByText('来源信息已删除，无法跳转')).toBeInTheDocument();
    expect(onSourceNavigationHandled).toHaveBeenCalledTimes(1);
  });

  it('来源消息已删除但对话仍存在时显示统一删除提示且不切换历史', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getConversation).mockResolvedValue({
      ...butlerConv,
      id: 'conv-source',
      title: '来源对话',
    });
    vi.mocked(chatService.getHistory).mockImplementation(async conversationId => {
      if (conversationId === 'conv-source') {
        return [chatMessage({ id: 'other-msg', conversationId: 'conv-source', role: 'user', content: '来源对话里剩余的消息' })];
      }
      return [];
    });
    const onSourceNavigationHandled = vi.fn();

    render(
      <ChatStream
        role={null}
        sourceNavigationTarget={{ conversationId: 'conv-source', messageId: 'missing-msg', roleId: null }}
        onSourceNavigationHandled={onSourceNavigationHandled}
      />,
    );

    expect(await screen.findByText('来源信息已删除，无法跳转')).toBeInTheDocument();
    expect(screen.queryByText('来源对话里剩余的消息')).not.toBeInTheDocument();
    expect(onSourceNavigationHandled).toHaveBeenCalledTimes(1);
  });

  /// AC-2 / AC-7: 角色视图里的"新对话"也必须继续归属于当前角色。
  /// 否则用户在角色空间里新开的会话会落到管家历史里，形成最隐蔽的历史污染。
  it('role 非空时点击新对话会创建归属该角色的新会话', async () => {
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);
    vi.mocked(chatService.newConversation).mockResolvedValue({
      ...roleConv,
      id: 'conv-role-new',
    });

    render(<ChatStream role={baseRole} />);

    await waitFor(() => {
      expect(chatService.getRoleConversation).toHaveBeenCalledWith('role-1');
    });

    fireEvent.click(screen.getByRole('button', { name: /新对话/ }));

    await waitFor(() => {
      expect(chatService.newConversation).toHaveBeenCalledWith('conv-role-1', 'role-1');
    });
  });

  /// 修复回归: 角色视图的输入框占位符必须包含角色名。
  /// 否则用户进入角色后看到“跟管家说点什么...”，与当前对话身份脱节。
  it('role 非空时输入框占位符包含角色名', async () => {
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);

    render(<ChatStream role={baseRole} />);

    expect(await screen.findByPlaceholderText('跟 产品经理 说点什么...')).toBeInTheDocument();
  });

  /// 修复回归: 最终 done 到达后输入框必须立即解锁。
  /// 历史刷新可能要等后端落库/查询几秒，不能让用户在文本已经输出完后继续等待。
  it('最终 done 到达后不等待历史刷新就解锁输入框', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue({
      id: 'user-1',
      conversationId: 'conv-butler',
      role: 'user',
      content: '继续',
      thinkingContent: '',
      isComplete: true,
      createdAt: '2026-01-01T00:00:00Z',
      routingMetadata: null,
    });
    const getStreamHandler = captureStreamHandler();
    let resolveHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveHistoryRefresh = () => resolve([]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '回复完成',
        done: false,
        thinking: false,
      });
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '',
        done: true,
        thinking: false,
      });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('回复完成')).toBeInTheDocument();

    await act(async () => {
      resolveHistoryRefresh();
    });
    expect(screen.getByText('继续')).toBeInTheDocument();
    expect(screen.getByText('回复完成')).toBeInTheDocument();
  });

  it('委派首段 done 到达时不提前解锁输入框', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue({
      id: 'user-1',
      conversationId: 'conv-butler',
      role: 'user',
      content: '明天有产品设计评审',
      thinkingContent: '',
      isComplete: true,
      createdAt: '2026-01-01T00:00:00Z',
      routingMetadata: null,
    });
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '明天有产品设计评审' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '稍等，我让产品经理看一下。',
        done: false,
        thinking: false,
      });
      getStreamHandler()({
        conversationId: 'conv-butler',
        messageId: 'assistant-message-1',
        token: '',
        done: true,
        thinking: false,
      });
    });

    expect(input).toBeDisabled();
  });

  it('旧一轮历史刷新返回时不清除下一轮流式回复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce({
        id: 'user-1',
        conversationId: 'conv-butler',
        role: 'user',
        content: '第一条',
        thinkingContent: '',
        isComplete: true,
        createdAt: '2026-01-01T00:00:00Z',
        routingMetadata: null,
      })
      .mockResolvedValueOnce({
        id: 'user-2',
        conversationId: 'conv-butler',
        role: 'user',
        content: '第二条',
        thinkingContent: '',
        isComplete: true,
        createdAt: '2026-01-01T00:00:01Z',
        routingMetadata: null,
      });
    const getStreamHandler = captureStreamHandler();
    let resolveFirstHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFirstHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '第一次回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '第一条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '第一次回复',
        done: false,
        thinking: false,
      });
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '',
        done: true,
        thinking: false,
      });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    fireEvent.change(input, { target: { value: '第二条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '第二次回复',
        done: false,
        thinking: false,
      });
    });

    await act(async () => {
      resolveFirstHistoryRefresh();
    });

    expect(screen.getByText('第二条')).toBeInTheDocument();
    expect(screen.getByText('第二次回复')).toBeInTheDocument();
    expect(input).toBeDisabled();
  });

  it('流事件早于 sendMessage 返回且历史刷新较慢时仍保持用户消息在回复前面', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    let resolveSendMessage!: () => void;
    let resolveHistoryRefresh!: () => void;
    vi.mocked(chatService.sendMessage).mockImplementation(() => new Promise(resolve => {
      resolveSendMessage = () => resolve(chatMessage({ id: 'user-1', content: '继续' }));
    }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '继续' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '提前回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '提前回复',
        done: false,
        thinking: false,
      });
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '',
        done: true,
        thinking: false,
      });
    });

    await act(async () => {
      resolveSendMessage();
    });

    const user = screen.getByText('继续');
    const assistant = screen.getByText('提前回复');
    expect(user.compareDocumentPosition(assistant) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getAllByText('继续')).toHaveLength(1);
    expect(input).not.toBeDisabled();

    await act(async () => {
      resolveHistoryRefresh();
    });
    expect(screen.getByText('继续')).toBeInTheDocument();
    expect(screen.getByText('提前回复')).toBeInTheDocument();
  });

  it('委派首段旧历史刷新晚于最终 done 返回时不覆盖最终回复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '明天有产品设计评审' }));
    const getStreamHandler = captureStreamHandler();
    let resolveSegmentHistoryRefresh!: () => void;
    let resolveFinalHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveSegmentHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '稍等，我让产品经理看一下。' }),
        ]);
      }))
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFinalHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '明天有产品设计评审' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '稍等，我让产品经理看一下。' }),
          chatMessage({ id: 'history-assistant-2', role: 'assistant', content: '产品经理的真实回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '明天有产品设计评审' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '稍等，我让产品经理看一下。', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'assistant-message-1', token: '', done: true, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'followup-message-1', token: '产品经理的真实回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'followup-message-1', token: '', done: true, thinking: false });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('产品经理的真实回复')).toBeInTheDocument();

    await act(async () => {
      resolveSegmentHistoryRefresh();
    });
    expect(screen.getByText('产品经理的真实回复')).toBeInTheDocument();
    expect(input).not.toBeDisabled();

    await act(async () => {
      resolveFinalHistoryRefresh();
    });
    expect(screen.getByText('产品经理的真实回复')).toBeInTheDocument();
  });

  it('委派最终历史返回后不重复显示 follow-up 回复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '我明天要提交下个版本的PRD' }));
    const getStreamHandler = captureStreamHandler();
    let resolveSegmentHistoryRefresh!: () => void;
    let resolveFinalHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveSegmentHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '我明天要提交下个版本的PRD' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '稍等，我让产品经理来帮你准备。' }),
        ]);
      }))
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFinalHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '我明天要提交下个版本的PRD' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '稍等，我让产品经理来帮你准备。' }),
          chatMessage({ id: 'history-assistant-2', role: 'assistant', content: '产品经理帮你整理了一份PRD框架清单。' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '我明天要提交下个版本的PRD' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '稍等，我让产品经理来帮你准备。', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'assistant-message-1', token: '', done: true, thinking: false });
    });
    await act(async () => {
      resolveSegmentHistoryRefresh();
    });

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'followup-message-1', token: '产品经理帮你整理了一份PRD框架清单。', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', messageId: 'followup-message-1', token: '', done: true, thinking: false });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('产品经理帮你整理了一份PRD框架清单。')).toBeInTheDocument();

    await act(async () => {
      resolveFinalHistoryRefresh();
    });

    expect(screen.getAllByText('产品经理帮你整理了一份PRD框架清单。')).toHaveLength(1);
  });

  it('最终历史返回前发送下一条时清理上一轮本地完成回复，历史返回后按落库顺序恢复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce(chatMessage({ id: 'user-1', content: '第一条' }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '第二条' }));
    const getStreamHandler = captureStreamHandler();
    let resolveFirstHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFirstHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '第一条回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '第一条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第一条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('第一条回复')).toBeInTheDocument();

    fireEvent.change(input, { target: { value: '第二条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());
    expect(screen.queryByText('第一条回复')).not.toBeInTheDocument();

    await act(async () => {
      resolveFirstHistoryRefresh();
    });
    expect(screen.getAllByText('第一条回复')).toHaveLength(1);
    const firstReply = screen.getByText('第一条回复');
    const secondUser = screen.getByText('第二条');
    expect(firstReply.compareDocumentPosition(secondUser) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('连续相同问答且历史未返回时只保留当前轮本地完成回复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce(chatMessage({ id: 'user-1', content: '继续' }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '继续' }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(() => undefined))
      .mockRejectedValueOnce(new Error('history failed'));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '好的', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '好的', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getAllByText('继续')).toHaveLength(2);
    expect(screen.getAllByText('好的')).toHaveLength(1);
  });

  it('后续全量历史返回时替换普通单段本地完成回复且不重复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce(chatMessage({ id: 'user-1', content: '第一条' }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '第二条' }));
    const getStreamHandler = captureStreamHandler();
    let resolveFirstHistoryRefresh!: () => void;
    let resolveSecondHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFirstHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '第一条回复' }),
        ]);
      }))
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveSecondHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '第一条回复' }),
          chatMessage({ id: 'history-user-2', content: '第二条' }),
          chatMessage({ id: 'history-assistant-2', role: 'assistant', content: '第二条回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '第一条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第一条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    fireEvent.change(input, { target: { value: '第二条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第二条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    await act(async () => {
      resolveSecondHistoryRefresh();
    });
    expect(screen.getAllByText('第一条回复')).toHaveLength(1);
    expect(screen.getAllByText('第二条回复')).toHaveLength(1);
    expect(screen.getByText('第一条').compareDocumentPosition(screen.getByText('第一条回复')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByText('第一条回复').compareDocumentPosition(screen.getByText('第二条')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByText('第二条').compareDocumentPosition(screen.getByText('第二条回复')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    await act(async () => {
      resolveFirstHistoryRefresh();
    });
    expect(screen.getAllByText('第一条回复')).toHaveLength(1);
    expect(screen.getAllByText('第二条回复')).toHaveLength(1);
  });

  it('新一轮发送前清理上一轮未落库的本地完成回复，避免漂移到下一轮末尾', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce(chatMessage({ id: 'user-1', content: '第一条' }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '第二条' }));
    const getStreamHandler = captureStreamHandler();
    let resolveFirstHistoryRefresh!: () => void;
    let resolveSecondHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFirstHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
        ]);
      }))
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveSecondHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '第一条' }),
          chatMessage({ id: 'history-user-2', content: '第二条' }),
          chatMessage({ id: 'history-assistant-2', role: 'assistant', content: '第二条回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '第一条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第一条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('第一条回复')).toBeInTheDocument();

    await act(async () => {
      resolveFirstHistoryRefresh();
    });
    expect(screen.getByText('第一条回复')).toBeInTheDocument();

    fireEvent.change(input, { target: { value: '第二条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());
    expect(screen.queryByText('第一条回复')).not.toBeInTheDocument();

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第二条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    await act(async () => {
      resolveSecondHistoryRefresh();
    });
    expect(screen.queryByText('第一条回复')).not.toBeInTheDocument();
    expect(screen.getAllByText('第二条回复')).toHaveLength(1);
    expect(screen.getByText('第一条').compareDocumentPosition(screen.getByText('第二条')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByText('第二条').compareDocumentPosition(screen.getByText('第二条回复')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('历史里已有旧同内容回复时仍保留当前未落库完成回复', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '继续' }));
    const getStreamHandler = captureStreamHandler();
    let resolveHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([
        chatMessage({ id: 'history-user-old', content: '旧问题' }),
        chatMessage({ id: 'history-assistant-old', role: 'assistant', content: '好的' }),
      ])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-old', content: '旧问题' }),
          chatMessage({ id: 'history-assistant-old', role: 'assistant', content: '好的' }),
          chatMessage({ id: 'history-user-1', content: '继续' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '好的', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await act(async () => {
      resolveHistoryRefresh();
    });

    expect(screen.getAllByText('好的')).toHaveLength(2);
  });

  it('最终历史刷新失败时用本地完成回复保底并结束流式状态', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '继续' }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockRejectedValueOnce(new Error('history failed'));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '回复完成', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.getByText('回复完成')).toBeInTheDocument();
    expect(screen.getByText('新对话').closest('button')).not.toBeDisabled();
  });

  it('只有 thinking token 且历史刷新失败时不展示原始 thinking 并解锁输入', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '他喜欢吃薯条' }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockRejectedValueOnce(new Error('history failed'));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '他喜欢吃薯条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '记录这条饮食偏好', done: false, thinking: true });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.queryByRole('button', { name: '执行过程' })).not.toBeInTheDocument();
    expect(screen.queryByText('记录这条饮食偏好')).not.toBeInTheDocument();
    expect(screen.getByText('管家')).toBeInTheDocument();
    expect(screen.getByText('新对话').closest('button')).not.toBeDisabled();
  });

  it('会话初始化完成前禁用输入，避免发送被静默丢弃', async () => {
    let resolveConversation!: () => void;
    vi.mocked(chatService.getButlerConversation).mockImplementation(() => new Promise(resolve => {
      resolveConversation = () => resolve(butlerConv);
    }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    expect(input).toBeDisabled();

    await act(async () => {
      resolveConversation();
    });

    await waitFor(() => expect(input).not.toBeDisabled());
  });

  it('sendMessage 返回 assistant 时移除未落库的本地用户占位消息', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({
      id: 'busy-assistant',
      role: 'assistant',
      content: '当前会话正在回复中，请稍后再试。',
    }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    await waitFor(() => expect(input).not.toBeDisabled());
    fireEvent.change(input, { target: { value: '这条不会真正发送' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(chatService.sendMessage).toHaveBeenCalled());
    expect(await screen.findByText('当前会话正在回复中，请稍后再试。')).toBeInTheDocument();
    expect(screen.queryByText('这条不会真正发送')).not.toBeInTheDocument();
    expect(input).not.toBeDisabled();
  });

  it('初始旧历史同内容用户消息不误消费本地占位消息', async () => {
    let resolveInitialHistory!: () => void;
    let resolveSendMessage!: () => void;
    const initialHistoryPromise = new Promise<ChatMessage[]>(resolve => {
      resolveInitialHistory = () => resolve([
        chatMessage({ id: 'history-user-1', content: '继续', createdAt: '2026-01-01T00:00:00Z' }),
      ]);
    });
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockImplementation(() => new Promise(resolve => {
      resolveSendMessage = () => resolve(chatMessage({ id: 'user-2', content: '继续', createdAt: '2026-01-01T00:00:01Z' }));
    }));
    vi.mocked(chatService.getHistory).mockReturnValueOnce(initialHistoryPromise);

    render(<ChatStream role={null} />);

    await waitFor(() => expect(chatService.getHistory).toHaveBeenCalledTimes(1));
    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(screen.getByText('继续')).toBeInTheDocument());

    await act(async () => {
      resolveInitialHistory();
    });
    await act(async () => {
      resolveSendMessage();
    });

    expect(screen.getAllByText('继续')).toHaveLength(2);
  });

  it('发送失败时回滚本地用户占位消息', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockRejectedValue(new Error('send failed'));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '不会发送成功' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(input).not.toBeDisabled());
    expect(screen.queryByText('不会发送成功')).not.toBeInTheDocument();
  });

  it('会话未初始化时忽略旧流事件', async () => {
    let resolveConversation!: () => void;
    vi.mocked(chatService.getButlerConversation).mockImplementation(() => new Promise(resolve => {
      resolveConversation = () => resolve(butlerConv);
    }));
    const getStreamHandler = captureStreamHandler();

    render(<ChatStream role={null} />);

    await act(async () => {
      getStreamHandler()({ conversationId: 'old-conv', token: '旧会话回复', done: false, thinking: false });
      resolveConversation();
    });

    expect(screen.queryByText('旧会话回复')).not.toBeInTheDocument();
  });

  it('连续发送相同内容时仍显示每一条用户消息', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    let resolveFirstHistoryRefresh!: () => void;
    vi.mocked(chatService.sendMessage)
      .mockResolvedValueOnce(chatMessage({ id: 'user-1', content: '继续', createdAt: '2026-01-01T00:00:00Z' }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '继续', createdAt: '2026-01-01T00:00:01Z' }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveFirstHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-user-1', content: '继续' }),
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '第一次回复' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第一次回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    fireEvent.change(input, { target: { value: '继续' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    expect(screen.getAllByText('继续')).toHaveLength(2);

    resolveFirstHistoryRefresh();
  });

  it('最终气泡不显示已记录为执行过程的开场说明', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([]);
    vi.mocked(chatService.getHistory).mockResolvedValue([]);

    render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    await act(async () => {
      const handler = getStreamHandler();
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'process-opening-narration',
          conversationId: 'conv-butler',
          messageId: 'assistant-message-1',
          opencodeSessionId: 'ses-1',
          eventType: 'narration',
          toolName: null,
          status: null,
          summary: '好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-butler',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'process-shell',
          conversationId: 'conv-butler',
          messageId: 'assistant-message-1',
          opencodeSessionId: 'ses-1',
          eventType: 'tool',
          toolName: 'bash',
          status: 'completed',
          summary: 'Shell执行 PPTX 到 Markdown 的转换',
          rawJson: JSON.stringify({ tool: 'bash', status: 'completed', input: { command: 'markitdown input.pptx > output.md' } }),
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:01Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-butler',
        messageId: 'assistant-message-1',
        token: '好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。转换完成了。已保存到 output.md',
        done: false,
        thinking: false,
      });
      handler({
        conversationId: 'conv-butler',
        messageId: 'assistant-message-1',
        token: '',
        done: true,
        thinking: false,
      });
    });

    await waitFor(() => expect(screen.getByText('转换完成了。已保存到 output.md')).toBeInTheDocument());
    expect(screen.queryByText(/好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。转换完成了/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '执行过程' }));
    expect(screen.getByText('好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。')).toBeInTheDocument();
  });

  it('角色工具执行中只显示弹跳点，完成后只显示最终结果', async () => {
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);
    vi.mocked(chatService.getMessageProcessEvents).mockResolvedValue([]);
    vi.mocked(chatService.getHistory).mockResolvedValue([]);

    const { container } = render(<ChatStream role={baseRole} />);
    await waitFor(() => expect(chatService.getRoleConversation).toHaveBeenCalledWith('role-1'));

    await act(async () => {
      const handler = getStreamHandler();
      handler({
        conversationId: 'conv-role-1',
        token: '',
        done: false,
        thinking: false,
        phase: 'process',
        processEvent: {
          id: 'role-process-1',
          conversationId: 'conv-role-1',
          messageId: 'assistant-message-1',
          opencodeSessionId: 'ses-role-1',
          eventType: 'narration',
          toolName: null,
          status: null,
          summary: '开始检查文件并执行转换。',
          rawJson: '{}',
          workingDirectory: null,
          createdAt: '2026-01-01T00:00:00Z',
        },
      } as StreamPayload);
      handler({
        conversationId: 'conv-role-1',
        token: '我来帮你把 PPTX 文件转换成 Markdown 格式。',
        done: false,
        thinking: false,
      });
    });

    expect(screen.getByRole('button', { name: '执行过程' })).toBeInTheDocument();
    expect(screen.getByText('开始检查文件并执行转换。')).toBeInTheDocument();
    expect(screen.queryByText('我来帮你把 PPTX 文件转换成 Markdown 格式。')).not.toBeInTheDocument();
    expect(container.querySelectorAll('.animate-bounce-forever')).toHaveLength(3);

    await act(async () => {
      const handler = getStreamHandler();
      handler({
        conversationId: 'conv-role-1',
        messageId: 'assistant-message-1',
        token: '转换完成。文件已保存为：D:\\AI\\output.md',
        done: false,
        thinking: false,
      });
      handler({
        conversationId: 'conv-role-1',
        messageId: 'assistant-message-1',
        token: '',
        done: true,
        thinking: false,
      });
    });

    await waitFor(() => expect(screen.getByText('转换完成。文件已保存为：D:\\AI\\output.md')).toBeInTheDocument());
    expect(screen.queryByText('我来帮你把 PPTX 文件转换成 Markdown 格式。')).not.toBeInTheDocument();
    expect(container.querySelectorAll('.animate-bounce-forever')).toHaveLength(0);
  });

  it('委派首段历史刷新先返回时保留等待下一段的流式状态', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue({
      id: 'user-1',
      conversationId: 'conv-butler',
      role: 'user',
      content: '明天有产品设计评审',
      thinkingContent: '',
      isComplete: true,
      createdAt: '2026-01-01T00:00:00Z',
      routingMetadata: null,
    });
    const getStreamHandler = captureStreamHandler();
    let resolveSegmentHistoryRefresh!: () => void;
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveSegmentHistoryRefresh = () => resolve([
          chatMessage({ id: 'history-assistant-1', role: 'assistant', content: '稍等，我让产品经理看一下。' }),
        ]);
      }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '明天有产品设计评审' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      getStreamHandler()({
        conversationId: 'conv-butler',
        token: '稍等，我让产品经理看一下。',
        done: false,
        thinking: false,
      });
      getStreamHandler()({
        conversationId: 'conv-butler',
        messageId: 'assistant-message-1',
        token: '',
        done: true,
        thinking: false,
      });
    });

    await act(async () => {
      resolveSegmentHistoryRefresh();
    });

    expect(screen.getAllByText('稍等，我让产品经理看一下。')).toHaveLength(1);
    expect(input).toBeDisabled();
  });

  it('上一轮 sendMessage 晚失败时不解锁下一轮输入框', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    let rejectFirstSend!: () => void;
    vi.mocked(chatService.sendMessage)
      .mockImplementationOnce(() => new Promise((_, reject) => {
        rejectFirstSend = () => reject(new Error('first send failed late'));
      }))
      .mockResolvedValueOnce(chatMessage({ id: 'user-2', content: '第二条' }));
    const getStreamHandler = captureStreamHandler();
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '第一条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());

    await act(async () => {
      getStreamHandler()({ conversationId: 'conv-butler', token: '第一条回复', done: false, thinking: false });
      getStreamHandler()({ conversationId: 'conv-butler', token: '', done: true, thinking: false });
    });
    await waitFor(() => expect(input).not.toBeDisabled());

    fireEvent.change(input, { target: { value: '第二条' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(input).toBeDisabled());
    await act(async () => {
      rejectFirstSend();
    });

    expect(screen.queryByText('第一条')).not.toBeInTheDocument();
    expect(screen.getByText('第二条')).toBeInTheDocument();
    expect(input).toBeDisabled();
  });

  it('旧会话历史晚返回时不覆盖当前会话历史', async () => {
    let resolveButlerHistory!: () => void;
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);
    vi.mocked(chatService.getHistory)
      .mockImplementationOnce(() => new Promise(resolve => {
        resolveButlerHistory = () => resolve([
          chatMessage({ id: 'butler-history', content: '管家旧历史' }),
        ]);
      }))
      .mockResolvedValueOnce([
        chatMessage({ id: 'role-history', conversationId: 'conv-role-1', content: '角色当前历史' }),
      ]);

    const { rerender } = render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    rerender(<ChatStream role={baseRole} />);
    expect(await screen.findByText('角色当前历史')).toBeInTheDocument();

    await act(async () => {
      resolveButlerHistory();
    });

    expect(screen.getByText('角色当前历史')).toBeInTheDocument();
    expect(screen.queryByText('管家旧历史')).not.toBeInTheDocument();
  });

  it('旧会话请求晚返回时不覆盖当前角色会话', async () => {
    let resolveButlerConversation!: () => void;
    vi.mocked(chatService.getButlerConversation).mockImplementation(() => new Promise(resolve => {
      resolveButlerConversation = () => resolve(butlerConv);
    }));
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);
    vi.mocked(chatService.getHistory)
      .mockResolvedValueOnce([
        chatMessage({ id: 'role-history', conversationId: 'conv-role-1', content: '角色当前历史' }),
      ])
      .mockResolvedValueOnce([
        chatMessage({ id: 'butler-history', content: '管家旧历史' }),
      ]);

    const { rerender } = render(<ChatStream role={null} />);
    await waitFor(() => expect(chatService.getButlerConversation).toHaveBeenCalled());

    rerender(<ChatStream role={baseRole} />);
    expect(await screen.findByText('角色当前历史')).toBeInTheDocument();

    await act(async () => {
      resolveButlerConversation();
    });

    expect(screen.getByText('角色当前历史')).toBeInTheDocument();
    expect(screen.queryByText('管家旧历史')).not.toBeInTheDocument();
  });

  it('初始历史晚返回时不覆盖本地发送消息', async () => {
    let resolveInitialHistory!: () => void;
    const initialHistoryPromise = new Promise<ChatMessage[]>(resolve => {
      resolveInitialHistory = () => resolve([]);
    });
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-1', content: '历史加载中发送' }));
    vi.mocked(chatService.getHistory).mockReturnValueOnce(initialHistoryPromise);

    render(<ChatStream role={null} />);

    await waitFor(() => expect(chatService.getHistory).toHaveBeenCalledTimes(1));
    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '历史加载中发送' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(screen.getByText('历史加载中发送')).toBeInTheDocument());

    await act(async () => {
      resolveInitialHistory();
    });

    expect(screen.getByText('历史加载中发送')).toBeInTheDocument();
  });

  it('非空旧历史晚返回时不丢失刚发送的服务端用户消息', async () => {
    let resolveInitialHistory!: () => void;
    const initialHistoryPromise = new Promise<ChatMessage[]>(resolve => {
      resolveInitialHistory = () => resolve([
        chatMessage({ id: 'old-user', content: '旧消息' }),
      ]);
    });
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({ id: 'user-new', content: '新消息' }));
    vi.mocked(chatService.getHistory).mockReturnValueOnce(initialHistoryPromise);

    render(<ChatStream role={null} />);

    await waitFor(() => expect(chatService.getHistory).toHaveBeenCalledTimes(1));
    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '新消息' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(screen.getByText('新消息')).toBeInTheDocument());

    await act(async () => {
      resolveInitialHistory();
    });

    await waitFor(() => expect(screen.getByText('旧消息')).toBeInTheDocument());
    expect(screen.getByText('新消息')).toBeInTheDocument();
  });
});
