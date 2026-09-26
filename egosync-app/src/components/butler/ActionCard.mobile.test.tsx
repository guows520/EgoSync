// Story 16.4 评审修复：ActionCard 移动端触控目标测试（新建文件——
// ActionCard.test.tsx 一行不改，规格冻结块「新测试一律新建文件」）。
//
// 钉死：确认/拒绝按钮小屏触控高度（2026-09-26 owner 反馈小屏 44px 观感
// 过高，显式重协商 16.4 冻结 ≥44×44px 红线至 36px——档位 B；此为局部
// 特例，勿外推至其它触控面）；桌面 py-1.5 原值零变化。留痕：spec-16-4
// Spec Change Log 2026-09-26 第二条目 + 案卷 web-mobile-control-density。
//
// 2026-09-26 owner 授权改写（原全宽纵排为 16.4 冻结设计，已显式重协商）：
// 小屏改为一行右对齐——按钮无 max-md:flex-1、容器无 max-md:flex-col/
// max-md:justify-stretch，仅 justify-end（与桌面同口径，375px 下两钮一行）。

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
  it('确认/拒绝按钮小屏 36px（owner 重协商 44px 红线）且不全宽（无 flex-1），桌面原值不变', () => {
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
      expect(button).toHaveClass('max-md:min-h-[36px]', 'max-md:flex', 'max-md:items-center', 'max-md:justify-center');
      // 一行右对齐：小屏不再全宽堆叠（2026-09-26 owner 裁决）
      expect(button).not.toHaveClass('max-md:flex-1');
      // 触控降档痕迹钉死：36px 档位（owner 档位 B），非 44px 亦非 40px
      // （逐类拆开——多类 not.toHaveClass 是 OR 语义，单独回退一类捕获不到）
      expect(button).not.toHaveClass('max-md:min-h-[44px]');
      expect(button).not.toHaveClass('max-md:min-h-10');
      // 桌面原值保持（NFR-C4：py-1.5 紧凑排）
      expect(button).toHaveClass('py-1.5');
    }
  });

  it('按钮容器小屏一行右对齐（justify-end，无纵排/全宽作用）', () => {
    const { container } = render(
      <ActionCard
        suggestion={makeSuggestion()}
        onConfirm={vi.fn()}
        onReject={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    const row = screen.getByRole('button', { name: '确认' }).parentElement;
    expect(row).toHaveClass('justify-end');
    // review F3：长标签换行兜底（inert——当前标签宽度放得下一行）
    expect(row).toHaveClass('flex-wrap');
    expect(row).not.toHaveClass('max-md:flex-col');
    expect(row).not.toHaveClass('max-md:justify-stretch');
    // 桌面基线 gap-2/mt-3 保留（按钮行 density 相关类未动；基线丢失则此处红）
    expect(row).toHaveClass('gap-2', 'mt-3');
    expect(container).toBeTruthy();
  });
});
