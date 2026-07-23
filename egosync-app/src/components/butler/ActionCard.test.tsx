import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ActionCard } from './ActionCard';
import type { SuggestionWithRole } from '../../types/suggestion';

function makeSuggestion(overrides: Partial<SuggestionWithRole> = {}): SuggestionWithRole {
  return {
    id: 'sug-1',
    roleId: 'role-1',
    title: '建议标题',
    content: '建议内容详情',
    priority: 'medium',
    status: 'pending',
    rejectionReason: null,
    convertedTaskId: null,
    conversationId: null,
    createdAt: new Date(Date.now() - 60_000).toISOString(),
    roleName: '产品经理',
    roleIcon: 'briefcase',
    roleColor: '#6366F1',
    ...overrides,
  };
}

describe('ActionCard', () => {
  it('渲染建议标题、内容、来源角色名和优先级标签', () => {
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={vi.fn()} onDismiss={vi.fn()} />);

    expect(screen.getByText('建议标题')).toBeInTheDocument();
    expect(screen.getByText('建议内容详情')).toBeInTheDocument();
    expect(screen.getByText('产品经理')).toBeInTheDocument();
    expect(screen.getByText('中优')).toBeInTheDocument();
  });

  it('点击确认按钮后触发 onConfirm 回调', () => {
    const onConfirm = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={onConfirm} onReject={vi.fn()} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '确认' }));
    expect(onConfirm).toHaveBeenCalledWith('sug-1');
  });

  it('点击拒绝按钮后展开拒绝原因选择器', () => {
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={vi.fn()} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '拒绝' }));
    expect(screen.getByText('拒绝原因：')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '不相关' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '时机不对' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '已完成' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '其他' })).toBeInTheDocument();
  });

  it('选择拒绝原因后触发 onReject 回调', () => {
    const onReject = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={onReject} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '拒绝' }));
    fireEvent.click(screen.getByRole('button', { name: '不相关' }));
    expect(onReject).toHaveBeenCalledWith('sug-1', 'irrelevant');
  });

  it('确认成功后显示 ✓ 图标并淡出消失', async () => {
    const onConfirm = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={onConfirm} onReject={vi.fn()} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '确认' }));

    await waitFor(() => {
      expect(screen.getByLabelText('建议标题 - 产品经理')).toHaveClass('opacity-50');
    });
  });

  it('拒绝成功后显示 ✕ 图标并淡出消失', async () => {
    const onReject = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={onReject} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '拒绝' }));
    fireEvent.click(screen.getByRole('button', { name: '时机不对' }));

    await waitFor(() => {
      expect(screen.getByLabelText('建议标题 - 产品经理')).toHaveClass('opacity-50');
    });
  });

  it('确认动画结束后触发 onDismiss', async () => {
    const onDismiss = vi.fn();
    const onConfirm = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={onConfirm} onReject={vi.fn()} onDismiss={onDismiss} />);

    fireEvent.click(screen.getByRole('button', { name: '确认' }));

    await waitFor(() => {
      expect(onDismiss).toHaveBeenCalledWith('sug-1');
    });
  });

  it('拒绝动画结束后触发 onDismiss', async () => {
    const onDismiss = vi.fn();
    const onReject = vi.fn(() => Promise.resolve());
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={onReject} onDismiss={onDismiss} />);

    fireEvent.click(screen.getByRole('button', { name: '拒绝' }));
    fireEvent.click(screen.getByRole('button', { name: '已完成' }));

    await waitFor(() => {
      expect(onDismiss).toHaveBeenCalledWith('sug-1');
    });
  });

  it('确认失败后恢复 pending 状态可再次操作', async () => {
    const onConfirm = vi.fn(() => Promise.reject(new Error('fail')));
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={onConfirm} onReject={vi.fn()} onDismiss={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '确认' }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: '确认' })).toBeInTheDocument();
    });
  });

  it('建议卡片及拒绝原因控件包含深色模式样式', () => {
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={vi.fn()} onDismiss={vi.fn()} />);

    expect(screen.getByRole('article')).toHaveClass('dark:bg-slate-800', 'dark:border-slate-700');
    expect(screen.getByText('建议标题')).toHaveClass('dark:text-slate-100');

    fireEvent.click(screen.getByRole('button', { name: '拒绝' }));
    expect(screen.getByText('拒绝原因：')).toHaveClass('dark:text-slate-300');
    fireEvent.click(screen.getByRole('button', { name: '其他' }));
    expect(screen.getByPlaceholderText('输入具体原因（可选）')).toHaveClass('dark:bg-slate-900', 'dark:text-slate-100');
  });

  it('无障碍：role=article 和 aria-label 正确', () => {
    render(<ActionCard suggestion={makeSuggestion()} onConfirm={vi.fn()} onReject={vi.fn()} onDismiss={vi.fn()} />);
    const article = screen.getByRole('article');
    expect(article).toHaveAttribute('aria-label', '建议标题 - 产品经理');
  });
});
