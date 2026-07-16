import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TaskDecompositionCard } from './TaskDecompositionCard';
import type { TaskDecompositionProposal } from '../../types/taskDecomposition';

const proposal: TaskDecompositionProposal = {
  id: 'proposal-1',
  roleId: 'role-1',
  sourceConversationId: 'conv-1',
  taskSummary: '安排家长会',
  items: [
    { title: '准备家长会材料', deadline: null },
    { title: '联系班主任', deadline: '2026-07-17' },
  ],
  status: 'pending',
  createdAt: '2026-07-16T10:00:00Z',
  resolvedAt: null,
  roleName: '家庭',
  roleIcon: 'home',
  roleColor: '#6366F1',
};

describe('TaskDecompositionCard', () => {
  it('展示原事项、全部子任务和两个整批选择', () => {
    render(<TaskDecompositionCard proposal={proposal} onAccept={vi.fn()} onKeepSingle={vi.fn()} />);

    expect(screen.getByText('原事项：安排家长会')).toBeInTheDocument();
    expect(screen.getByText(/准备家长会材料/)).toBeInTheDocument();
    expect(screen.getByText(/联系班主任/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '接受拆分' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '不要拆分' })).toBeInTheDocument();
  });

  it('接受拆分只提交一次并在处理中禁用两个选择', async () => {
    let resolveAction: (() => void) | undefined;
    const onAccept = vi.fn(() => new Promise<void>(resolve => {
      resolveAction = resolve;
    }));
    render(<TaskDecompositionCard proposal={proposal} onAccept={onAccept} onKeepSingle={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '接受拆分' }));
    fireEvent.click(screen.getByRole('button', { name: '创建中…' }));

    expect(onAccept).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: '创建中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '不要拆分' })).toBeDisabled();
    resolveAction?.();
  });

  it('不要拆分调用保持单任务操作', async () => {
    const onKeepSingle = vi.fn(() => Promise.resolve());
    render(<TaskDecompositionCard proposal={proposal} onAccept={vi.fn()} onKeepSingle={onKeepSingle} />);

    fireEvent.click(screen.getByRole('button', { name: '不要拆分' }));
    await waitFor(() => expect(onKeepSingle).toHaveBeenCalledWith('proposal-1'));
  });

  it('操作失败后保留卡片并允许重试', async () => {
    const onAccept = vi.fn(() => Promise.reject(new Error('failed')));
    render(<TaskDecompositionCard proposal={proposal} onAccept={onAccept} onKeepSingle={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '接受拆分' }));

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('提案仍保留'));
    expect(screen.getByRole('button', { name: '接受拆分' })).toBeEnabled();
  });
});
