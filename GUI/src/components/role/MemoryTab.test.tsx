import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { MemoryTab } from './MemoryTab';
import { memoryService } from '../../services/memoryService';
import type { Memory, MemorySourceMessage } from '../../types/memory';

vi.mock('../../services/memoryService', () => ({
  memoryService: {
    list: vi.fn(),
    listAll: vi.fn(),
    count: vi.fn(),
    getSourceMessages: vi.fn(),
    delete: vi.fn(),
  },
}));

const preferenceMemory: Memory = {
  id: 'memory-1',
  roleId: null,
  category: 'preference',
  content: '喜欢在早晨写 PRD，认为这个时候头脑最清醒',
  sourceConversationId: 'conv-1',
  sourceMessageIds: '["msg-1"]',
  createdAt: '2026-05-30T12:01:29Z',
};

const roleFactMemory: Memory = {
  id: 'memory-2',
  roleId: 'role-1',
  category: 'fact',
  content: '产品经理负责设计评审',
  sourceConversationId: 'conv-2',
  sourceMessageIds: '["msg-2"]',
  createdAt: '2026-05-30T12:03:29Z',
};

const taskMemory: Memory = {
  id: 'memory-3',
  roleId: 'role-1',
  category: 'task_status',
  content: '产品经理正在准备设计评审',
  sourceConversationId: 'conv-3',
  sourceMessageIds: '["msg-3"]',
  createdAt: '2026-05-30T12:04:29Z',
};

const sourceMessage: MemorySourceMessage = {
  id: 'msg-1',
  conversationId: 'conv-1',
  role: 'user',
  content: '我喜欢早晨写 PRD，这时候头脑最清醒。',
  createdAt: '2026-05-30T12:01:29Z',
  isSource: true,
};

describe('MemoryTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('管家记忆页加载并展示全局与角色真实记忆，但不展示任务状态', async () => {
    vi.mocked(memoryService.listAll).mockResolvedValue([preferenceMemory, roleFactMemory, taskMemory]);

    render(<MemoryTab roleId={null} includeRoleMemories showOwnerLabel roleLabels={{ 'role-1': '产品经理' }} />);

    await waitFor(() => {
      expect(memoryService.listAll).toHaveBeenCalledWith(undefined);
    });
    expect(memoryService.list).not.toHaveBeenCalled();
    expect(await screen.findByText('喜欢在早晨写 PRD，认为这个时候头脑最清醒')).toBeInTheDocument();
    expect(screen.getByText('产品经理负责设计评审')).toBeInTheDocument();
    expect(screen.queryByText('产品经理正在准备设计评审')).not.toBeInTheDocument();
    expect(screen.getByText('管家')).toBeInTheDocument();
    expect(screen.getByText('产品经理')).toBeInTheDocument();
    expect(screen.queryByText(/数据驱动偏好/)).not.toBeInTheDocument();
  });

  it('角色记忆页按角色 ID 加载记忆', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([]);

    render(<MemoryTab roleId="role-1" />);

    await waitFor(() => {
      expect(memoryService.list).toHaveBeenCalledWith('role-1', undefined);
    });
  });

  it('类别筛选作用于当前查询范围并按产品顺序显示分类', async () => {
    vi.mocked(memoryService.listAll).mockResolvedValue([]);

    render(<MemoryTab roleId={null} includeRoleMemories />);

    await waitFor(() => expect(memoryService.listAll).toHaveBeenCalledTimes(1));
    const filterLabels = screen.getByLabelText('记忆类别筛选').querySelectorAll('button');
    expect(Array.from(filterLabels).map(button => button.textContent)).toEqual([
      '全部',
      '事实',
      '偏好',
      '认知模式',
    ]);
    expect(screen.queryByRole('button', { name: '任务状态' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '偏好' }));

    await waitFor(() => {
      expect(memoryService.listAll).toHaveBeenLastCalledWith({ category: 'preference' });
    });
    expect(screen.getByRole('button', { name: '偏好' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('角色和管家空态都使用温暖文案', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([]);
    const { unmount } = render(<MemoryTab roleId="role-1" />);

    expect(await screen.findByText('还没有记忆，多和这个角色聊聊吧')).toBeInTheDocument();
    unmount();

    vi.mocked(memoryService.listAll).mockResolvedValue([]);
    render(<MemoryTab roleId={null} includeRoleMemories />);

    expect(await screen.findByText('还没有记忆，多聊几次，我会慢慢记住重要的事')).toBeInTheDocument();
    expect(screen.queryByText('暂无数据')).not.toBeInTheDocument();
  });

  it('展开来源时懒加载并渲染原始来源消息', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    vi.mocked(memoryService.getSourceMessages).mockResolvedValue([sourceMessage]);

    render(<MemoryTab roleId={null} />);

    const sourceButton = await screen.findByRole('button', { name: /来源对话 .*查看原文/ });
    expect(sourceButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('button', { name: '遗忘' })).toBeEnabled();

    fireEvent.click(sourceButton);

    await waitFor(() => {
      expect(memoryService.getSourceMessages).toHaveBeenCalledWith('memory-1');
    });
    expect(await screen.findByText('我喜欢早晨写 PRD，这时候头脑最清醒。')).toBeInTheDocument();
    expect(screen.getByText('用户')).toBeInTheDocument();
    expect(screen.queryByText('来源原文将在后续故事中接入')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /收起/ })).toHaveAttribute('aria-expanded', 'true');
  });

  it('来源缺失时显示不可用空态且不崩溃', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    vi.mocked(memoryService.getSourceMessages).mockResolvedValue([]);

    render(<MemoryTab roleId={null} />);

    fireEvent.click(await screen.findByRole('button', { name: /来源对话 .*查看原文/ }));

    expect(await screen.findByText('来源对话已不可用')).toBeInTheDocument();
  });

  it('点击遗忘后显示自定义确认且不调用系统 confirm', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    const confirmSpy = vi.spyOn(window, 'confirm');

    render(<MemoryTab roleId={null} />);

    fireEvent.click(await screen.findByRole('button', { name: '遗忘' }));

    expect(await screen.findByText('确定要忘记这条吗？忘了就真忘了哦。原始对话还会留在历史里。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '确认遗忘' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '再想想' })).toBeInTheDocument();
    expect(confirmSpy).not.toHaveBeenCalled();
  });

  it('取消遗忘不调用删除服务且保留卡片与来源展开状态', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    vi.mocked(memoryService.getSourceMessages).mockResolvedValue([sourceMessage]);

    render(<MemoryTab roleId={null} />);

    fireEvent.click(await screen.findByRole('button', { name: /来源对话 .*查看原文/ }));
    expect(await screen.findByText('我喜欢早晨写 PRD，这时候头脑最清醒。')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '遗忘' }));
    fireEvent.click(await screen.findByRole('button', { name: '再想想' }));

    expect(memoryService.delete).not.toHaveBeenCalled();
    expect(screen.getByText('喜欢在早晨写 PRD，认为这个时候头脑最清醒')).toBeInTheDocument();
    expect(screen.getByText('我喜欢早晨写 PRD，这时候头脑最清醒。')).toBeInTheDocument();
  });

  it('确认遗忘后调用删除、刷新列表并通知父级刷新 badge', async () => {
    vi.mocked(memoryService.list)
      .mockResolvedValueOnce([preferenceMemory])
      .mockResolvedValueOnce([]);
    vi.mocked(memoryService.delete).mockResolvedValue(undefined);
    const onMemoryDeleted = vi.fn();

    render(<MemoryTab roleId={null} onMemoryDeleted={onMemoryDeleted} />);

    fireEvent.click(await screen.findByRole('button', { name: '遗忘' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认遗忘' }));

    await waitFor(() => {
      expect(memoryService.delete).toHaveBeenCalledWith('memory-1');
    });
    await waitFor(() => {
      expect(memoryService.list).toHaveBeenCalledTimes(2);
    });
    expect(onMemoryDeleted).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('还没有记忆，多和这个角色聊聊吧')).toBeInTheDocument();
    expect(screen.queryByText('喜欢在早晨写 PRD，认为这个时候头脑最清醒')).not.toBeInTheDocument();
  });

  it('删除失败时保留卡片、显示温和错误并允许重试', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    vi.mocked(memoryService.delete)
      .mockRejectedValueOnce(new Error('delete failed'))
      .mockResolvedValueOnce(undefined);
    const onMemoryDeleted = vi.fn();

    render(<MemoryTab roleId={null} onMemoryDeleted={onMemoryDeleted} />);

    fireEvent.click(await screen.findByRole('button', { name: '遗忘' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认遗忘' }));

    expect(await screen.findByText('这条记忆暂时没忘掉，稍后再试一下')).toBeInTheDocument();
    expect(screen.getByText('喜欢在早晨写 PRD，认为这个时候头脑最清醒')).toBeInTheDocument();
    expect(onMemoryDeleted).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '确认遗忘' }));

    await waitFor(() => {
      expect(memoryService.delete).toHaveBeenCalledTimes(2);
    });
  });

  it('删除进行中重复点击确认遗忘不会触发第二次删除', async () => {
    vi.mocked(memoryService.list).mockResolvedValue([preferenceMemory]);
    let resolveDelete: (() => void) | undefined;
    vi.mocked(memoryService.delete).mockReturnValue(
      new Promise<void>(resolve => {
        resolveDelete = resolve;
      })
    );

    render(<MemoryTab roleId={null} />);

    fireEvent.click(await screen.findByRole('button', { name: '遗忘' }));
    const confirmButton = await screen.findByRole('button', { name: '确认遗忘' });

    fireEvent.click(confirmButton);
    fireEvent.click(confirmButton);
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(memoryService.delete).toHaveBeenCalledTimes(1);
    });
    expect(confirmButton).toBeDisabled();

    resolveDelete?.();
  });

  it('记忆已在后端不存在(NotFound)时按遗忘成功处理并移除卡片', async () => {
    vi.mocked(memoryService.list)
      .mockResolvedValueOnce([preferenceMemory])
      .mockResolvedValueOnce([]);
    vi.mocked(memoryService.delete).mockRejectedValue({ NotFound: '记忆不存在: memory-1' });
    const onMemoryDeleted = vi.fn();

    render(<MemoryTab roleId={null} onMemoryDeleted={onMemoryDeleted} />);

    fireEvent.click(await screen.findByRole('button', { name: '遗忘' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认遗忘' }));

    await waitFor(() => {
      expect(memoryService.list).toHaveBeenCalledTimes(2);
    });
    expect(onMemoryDeleted).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('还没有记忆，多和这个角色聊聊吧')).toBeInTheDocument();
    expect(screen.queryByText('喜欢在早晨写 PRD，认为这个时候头脑最清醒')).not.toBeInTheDocument();
    expect(screen.queryByText('这条记忆暂时没忘掉，稍后再试一下')).not.toBeInTheDocument();
  });
});
