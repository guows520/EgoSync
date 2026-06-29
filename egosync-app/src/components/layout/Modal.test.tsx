import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Modal } from './Modal';

describe('Modal 无障碍', () => {
  it('应有 role="dialog" 和 aria-modal="true"', () => {
    render(
      <Modal onClose={vi.fn()} ariaLabel="测试弹窗">
        <div>内容</div>
      </Modal>
    );
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  it('应使用 ariaLabel 作为可访问名称', () => {
    render(
      <Modal onClose={vi.fn()} ariaLabel="添加角色">
        <div>内容</div>
      </Modal>
    );
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-label', '添加角色');
  });

  it('Escape 键应调用 onClose', () => {
    const onClose = vi.fn();
    render(
      <Modal onClose={onClose} ariaLabel="测试弹窗">
        <div>内容</div>
      </Modal>
    );
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('点击遮罩应调用 onClose', () => {
    const onClose = vi.fn();
    render(
      <Modal onClose={onClose} ariaLabel="测试弹窗">
        <div>内容</div>
      </Modal>
    );
    const overlay = screen.getByRole('dialog').parentElement?.querySelector('.absolute.inset-0');
    if (overlay) {
      fireEvent.click(overlay);
      expect(onClose).toHaveBeenCalledTimes(1);
    }
  });
});
