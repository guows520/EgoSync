import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NotificationPanel } from './NotificationPanel';
import type { NotificationWithRole } from '../../types/notification';

const baseNotification: NotificationWithRole = {
  id: 'n-1',
  roleId: 'role-a',
  level: 'tap',
  content: '该休息一下了',
  isRead: false,
  createdAt: new Date(Date.now() - 5 * 60_000).toISOString(),
  roleName: '产品',
  roleIcon: '🎯',
  roleColor: '#6366F1',
};

describe('NotificationPanel', () => {
  it('加载中时显示加载提示', () => {
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[]} isLoading={true} markAsRead={vi.fn()} />,
    );
    expect(screen.getByText('加载中...')).toBeInTheDocument();
  });

  it('无通知时显示空状态文案', () => {
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[]} isLoading={false} markAsRead={vi.fn()} />,
    );
    expect(screen.getByText('暂时没有新通知')).toBeInTheDocument();
  });

  it('渲染通知列表并显示角色名和内容', () => {
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[baseNotification]} isLoading={false} markAsRead={vi.fn()} />,
    );
    expect(screen.getByText('产品')).toBeInTheDocument();
    expect(screen.getByText('该休息一下了')).toBeInTheDocument();
    expect(screen.getByText('轻触')).toBeInTheDocument();
  });

  it('点击未读通知时调用 markAsRead', () => {
    const markAsRead = vi.fn();
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[baseNotification]} isLoading={false} markAsRead={markAsRead} />,
    );
    fireEvent.click(screen.getByRole('article'));
    expect(markAsRead).toHaveBeenCalledWith('n-1');
  });

  it('点击已读通知时不调用 markAsRead', () => {
    const markAsRead = vi.fn();
    const readNotification = { ...baseNotification, isRead: true };
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[readNotification]} isLoading={false} markAsRead={markAsRead} />,
    );
    fireEvent.click(screen.getByRole('article'));
    expect(markAsRead).not.toHaveBeenCalled();
  });

  it('敲门级别通知显示敲门标签', () => {
    const knockNotification = { ...baseNotification, level: 'knock' as const };
    render(
      <NotificationPanel onClose={vi.fn()} notifications={[knockNotification]} isLoading={false} markAsRead={vi.fn()} />,
    );
    expect(screen.getByText('敲门')).toBeInTheDocument();
  });

  it('点击关闭按钮时调用 onClose', () => {
    const onClose = vi.fn();
    render(
      <NotificationPanel onClose={onClose} notifications={[]} isLoading={false} markAsRead={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole('button', { name: '' }));
    expect(onClose).toHaveBeenCalled();
  });
});
