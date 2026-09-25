// Story 16.4 评审修复：模态安全区避让测试（新建文件——Modal.test.tsx 一行
// 不改，规格冻结块「新测试一律新建文件，禁改既有断言」）。
//
// 范式：真类名断言（jsdom 不解析 env()——类在场即机制在场；env(safe-area-inset-*)
// 在无 inset 设备求值 0 ⇒ 桌面逐像素零变化）。覆盖两块遮罩：
// - 共享 Modal（Modal.tsx）：上/下安全区内边距；
// - RoleConfirmModal（自有遮罩，未复用 Modal）：同款避让（评审走查补齐项）。

import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Modal } from './Modal';

describe('Modal 安全区避让（Story 16.4）', () => {
  it('共享 Modal 遮罩带上/下安全区内边距（viewport-fit=cover 不贴边）', () => {
    const { container } = render(
      <Modal onClose={vi.fn()} ariaLabel="测试模态">
        <div>内容</div>
      </Modal>,
    );

    // 遮罩 = 最外层 fixed 容器（相对定位的 dialog 之父）
    const overlay = container.querySelector('.fixed');
    expect(overlay).not.toBeNull();
    expect(overlay).toHaveClass(
      'pt-[max(0px,env(safe-area-inset-top))]',
      'pb-[max(0px,env(safe-area-inset-bottom))]',
    );
  });
});

describe('RoleConfirmModal 安全区避让（Story 16.4）', () => {
  it('自有遮罩带上/下安全区内边距（与共享 Modal 同款模式）', async () => {
    const { RoleConfirmModal } = await import('../onboarding/RoleConfirmModal');
    const { container } = render(
      <RoleConfirmModal
        open
        proposal={{ name: '健身教练', icon: 'dumbbell', color: '#10B981', goal: '保持训练' }}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    const overlay = container.querySelector('.fixed');
    expect(overlay).not.toBeNull();
    expect(overlay).toHaveClass(
      'pt-[max(0px,env(safe-area-inset-top))]',
      'pb-[max(0px,env(safe-area-inset-bottom))]',
    );
  });
});
