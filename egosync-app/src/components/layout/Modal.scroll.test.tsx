// web 版手机浏览器五症状自适应修复·症状①：Modal 小屏高度/滚动测试
// （新建文件——Modal.test.tsx 一行不改，冻结规矩「新测试一律新建文件」）。
//
// 钉死：对话框带 `max-md:max-h-full max-md:overflow-y-auto`（小屏超高内容
// 改为对话框内滚动、标题可达）；无桌面态 max-h 类（max-md: 的对侧不存在
// ⇒ 桌面不触发，≥768px 逐像素零变化）。遮罩类不动——安全区 pt/pb 由
// Modal.safearea.test.tsx 钉死。

import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Modal } from './Modal';

describe('Modal 小屏高度/滚动（web 移动自适应修复）', () => {
  it('对话框带 max-md 高度上限与滚动，且无桌面态 max-h 类', () => {
    const { container } = render(
      <Modal onClose={vi.fn()} ariaLabel="测试模态">
        <div>内容</div>
      </Modal>,
    );

    const dialog = container.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog).toHaveClass('max-md:max-h-full', 'max-md:overflow-y-auto');

    // 桌面态无 max-h-* 类 ⇒ max-md: 的对侧不存在 ⇒ 桌面行为零变化
    const classes = (dialog?.className ?? '').split(/\s+/).filter(Boolean);
    const desktopMaxH = classes.filter(
      c => !c.startsWith('max-md:') && !c.startsWith('md:') && c.startsWith('max-h'),
    );
    expect(desktopMaxH).toEqual([]);
  });
});
