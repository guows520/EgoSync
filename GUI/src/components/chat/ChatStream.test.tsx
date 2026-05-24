import { render, waitFor, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ChatStream } from './ChatStream';
import { chatService } from '../../services/chatService';
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

describe('ChatStream conversation initialization (Story 2.2 AC-2 / AC-7)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
  /// 否则用户进入角色后看到"跟管家说点什么..."，与当前对话身份脱节。
  it('role 非空时输入框 placeholder 含角色名', async () => {
    vi.mocked(chatService.getRoleConversation).mockResolvedValue(roleConv);

    render(<ChatStream role={baseRole} />);

    await waitFor(() => {
      expect(chatService.getRoleConversation).toHaveBeenCalledWith('role-1');
    });

    const input = screen.getByPlaceholderText('跟 产品经理 说点什么...');
    expect(input).toBeInTheDocument();
  });
});