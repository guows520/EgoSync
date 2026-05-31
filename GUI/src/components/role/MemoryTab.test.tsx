import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { MemoryTab } from './MemoryTab';
import { memoryService } from '../../services/memoryService';

vi.mock('../../services/memoryService', () => ({
  memoryService: {
    list: vi.fn(),
    listAll: vi.fn(),
  },
}));

describe('MemoryTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('管家记忆页加载并展示全局与角色真实记忆', async () => {
    vi.mocked(memoryService.listAll).mockResolvedValue([
      {
        id: 'memory-1',
        roleId: null,
        category: 'preference',
        content: '用户喜欢在早晨写 PRD，认为这个时候头脑最清醒',
        sourceConversationId: 'conv-1',
        sourceMessageIds: '["msg-1"]',
        createdAt: '2026-05-30T12:01:29Z',
      },
      {
        id: 'memory-2',
        roleId: 'role-1',
        category: 'task_status',
        content: '产品经理正在准备设计评审',
        sourceConversationId: 'conv-2',
        sourceMessageIds: '["msg-2"]',
        createdAt: '2026-05-30T12:03:29Z',
      },
    ]);

    render(<MemoryTab roleId={null} includeRoleMemories />);

    await waitFor(() => {
      expect(memoryService.listAll).toHaveBeenCalled();
    });
    expect(memoryService.list).not.toHaveBeenCalled();
    expect(await screen.findByText('用户喜欢在早晨写 PRD，认为这个时候头脑最清醒')).toBeInTheDocument();
    expect(screen.getByText('产品经理正在准备设计评审')).toBeInTheDocument();
    expect(screen.queryByText(/数据驱动偏好/)).not.toBeInTheDocument();
  });

  it('角色记忆页按角色 ID 加载记忆', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([]);

    render(<MemoryTab roleId="role-1" />);

    await waitFor(() => {
      expect(memoryService.list).toHaveBeenCalledWith('role-1');
    });
  });

  it('没有记忆时显示空态', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([]);

    render(<MemoryTab roleId={null} />);

    expect(await screen.findByText('暂无记忆')).toBeInTheDocument();
  });

  it('真实记忆卡片保留遗忘和查看原文入口占位', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([
      {
        id: 'memory-1',
        roleId: null,
        category: 'preference',
        content: '用户喜欢在早晨写 PRD，认为这个时候头脑最清醒',
        sourceConversationId: 'conv-1',
        sourceMessageIds: '["msg-1"]',
        createdAt: '2026-05-30T12:01:29Z',
      },
    ]);

    render(<MemoryTab roleId={null} />);

    expect(await screen.findByText('遗忘')).toBeInTheDocument();
    const sourceButton = screen.getByRole('button', { name: /来源对话 .*2026\/05\/30 20:01 .*查看原文/ });
    expect(sourceButton).toBeInTheDocument();
    expect(sourceButton).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(sourceButton);

    expect(screen.getByText('来源原文将在后续故事中接入')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /收起/ })).toHaveAttribute('aria-expanded', 'true');
  });
});
