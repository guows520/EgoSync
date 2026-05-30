import { act, render, waitFor, screen, fireEvent } from '@testing-library/react';
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
    listConversations: vi.fn(),
    getHistory: vi.fn(),
    sendMessage: vi.fn(),
    deleteConversation: vi.fn(),
    newConversation: vi.fn(),
    stopStreaming: vi.fn(),
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

  it('sendMessage 返回 assistant 时移除未落库的本地用户占位消息', async () => {
    vi.mocked(chatService.getButlerConversation).mockResolvedValue(butlerConv);
    vi.mocked(chatService.sendMessage).mockResolvedValue(chatMessage({
      id: 'busy-assistant',
      role: 'assistant',
      content: '当前会话正在回复中，请稍后再试。',
    }));

    render(<ChatStream role={null} />);

    const input = await screen.findByPlaceholderText('跟管家说点什么，比如：帮我安排一个会议...');
    fireEvent.change(input, { target: { value: '这条不会真正发送' } });
    fireEvent.keyDown(input, { key: 'Enter' });

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
