// Story 16.4 评审修复：GlobalSettingsModal initialTab 落点契约（新建文件——
// 既有 GlobalSettingsModal.test.tsx 一行不改，规格冻结块「新测试一律新建
// 文件，禁改既有断言」）。
//
// 钉死两件事：
// 1. initialTab 缺省 = 'llm'（桌面 Sidebar/OnboardingView 入口不传参——
//    不回归到 16.4 之前的默认行为）；
// 2. initialTab="scheduler" 时真实落到调度时间 tab（App 移动设置「通知」
//    入口复用该落点——敲门通知声音设置所在）。

import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: { list: vi.fn().mockResolvedValue([]) },
}));
vi.mock('../../services/mcpService', () => ({
  mcpService: { list: vi.fn().mockResolvedValue([]) },
}));
vi.mock('../../services/dataService', () => ({
  dataService: { dataExport: vi.fn(), dataDestroy: vi.fn() },
}));
vi.mock('../../services/schedulerService', () => ({
  schedulerService: { getTimes: vi.fn().mockResolvedValue({ moderate: [], proactive: [] }) },
}));

describe('GlobalSettingsModal initialTab 落点（Story 16.4）', () => {
  it('initialTab 缺省落 LLM Provider 配置（桌面入口零参调用不变）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    expect(await screen.findByRole('heading', { name: 'LLM Provider 配置' })).toBeInTheDocument();
  });

  it('initialTab="scheduler" 落调度时间配置（移动设置「通知」入口落点）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} initialTab="scheduler" />);

    expect(await screen.findByRole('heading', { name: '调度时间配置' })).toBeInTheDocument();
    // 调度 tab 真实内容在场（同一批组件不重写的证据——非空壳）
    await waitFor(() => {
      expect(screen.getByText('积极主动')).toBeInTheDocument();
    });
  });
});
