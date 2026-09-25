// Story 16.4 评审修复：ActionCard 移动端触控目标测试（新建文件——
// ActionCard.test.tsx 一行不改，规格冻结块「新测试一律新建文件」）。
//
// 钉死：确认/拒绝按钮小屏 ≥44px（max-md:min-h-[44px]）+ 全宽堆叠
// （max-md:flex-1——线框「ActionCard 全宽堆叠」）；桌面 py-1.5 原值
// 零变化（AC2 触控红线的小屏收口）。

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ActionCard } from './ActionCard';
import type { SuggestionWithRole } from '../../types/suggestion';

function makeSuggestion(): SuggestionWithRole {
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
  };
}

describe('ActionCard 移动端触控目标（Story 16.4）', () => {
  it('确认/拒绝按钮小屏 ≥44px 且全宽堆叠（flex-1），桌面原值不变', () => {
    render(
      <ActionCard
        suggestion={makeSuggestion()}
        onConfirm={vi.fn()}
        onReject={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    for (const name of ['确认', '拒绝']) {
      const button = screen.getByRole('button', { name });
      expect(button).toHaveClass('max-md:min-h-[44px]', 'max-md:flex-1');
      // 桌面原值保持（NFR-C4：py-1.5 紧凑排）
      expect(button).toHaveClass('py-1.5');
    }
  });

  it('按钮容器小屏纵排全宽（max-md:flex-col + justify-stretch）', () => {
    const { container } = render(
      <ActionCard
        suggestion={makeSuggestion()}
        onConfirm={vi.fn()}
        onReject={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    const row = screen.getByRole('button', { name: '确认' }).parentElement;
    expect(row).toHaveClass('max-md:flex-col', 'max-md:justify-stretch', 'justify-end');
    expect(container).toBeTruthy();
  });
});
