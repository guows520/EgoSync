// Story 16.1：GlobalSettingsModal 浏览器分支门控测试（I/O 矩阵钉死）。
//
// 浏览器分支范式 = 删桩-恢复-`__resetTransportForTests()` 三件套
// （TitleBar.browser.test.tsx 同款）。服务层全量 vi.mock（浏览器分支下
// 无 Tauri invoke 桩兜底——真实 service 会走 HttpTransport fetch）。
//
// 覆盖：
// - companion 侧栏入口 + tab 内容不渲染（tab 级防挂载即调——
//   CompanionPairingSection 零挂载即零 companion_* 调用）；
// - data tab：导出/导入区不渲染（data_export / pick_import_file
//   desktop-only），销毁区保留（data_destroy 是 web-ok）；
// - 桌面零回归由 GlobalSettingsModal.test.tsx 默认桩覆盖（companion 按钮
//   / 导出导入区在桌面分支可见的既有断言）。

import { render, screen, fireEvent } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetTransportForTests } from '@/transport';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { dataService } from '../../services/dataService';
import { schedulerService } from '../../services/schedulerService';
import { appService } from '../../services/appService';

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: {
    list: vi.fn().mockResolvedValue([]),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    setDefault: vi.fn(),
    testConnection: vi.fn(),
    listModels: vi.fn(),
    listModelsByParams: vi.fn(),
  },
}));

vi.mock('../../services/mcpService', () => ({
  mcpService: {
    list: vi.fn().mockResolvedValue([]),
    listForRole: vi.fn(),
    listAvailableForRole: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    test: vi.fn(),
    addToRole: vi.fn(),
    removeFromRole: vi.fn(),
  },
}));

vi.mock('../../services/dataService', () => ({
  dataService: {
    dataExport: vi.fn(),
    dataDestroy: vi.fn(),
    pickImportFile: vi.fn(),
    dataImport: vi.fn(),
  },
}));

vi.mock('../../services/schedulerService', () => ({
  schedulerService: {
    getTimes: vi.fn().mockResolvedValue({ moderate: [], proactive: [] }),
    updateTimes: vi.fn(),
  },
}));

vi.mock('../../services/appService', () => ({
  appService: {
    isFirstLaunch: vi.fn(),
    completeOnboarding: vi.fn(),
    isLlmConfigured: vi.fn(),
    getSetting: vi.fn().mockResolvedValue('false'),
    setSetting: vi.fn(),
  },
}));

// CompanionPairingSection 挂载即调 companion_*（desktop-only）——mock 出
// 挂载标记，断言浏览器分支零挂载（tab 级门控）。
vi.mock('./CompanionPairingSection', () => ({
  CompanionPairingSection: () => <div data-testid="companion-section" />,
}));

describe('GlobalSettingsModal 浏览器分支 desktop-only 门控（Story 16.1）', () => {
  // 删桩前留存全局桩引用（afterEach 恢复——三件套完整范式，防未来加桌面
  // 分支用例时静默跑在被删桩的浏览器分支下）
  const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetTransportForTests();
    vi.mocked(schedulerService.getTimes).mockResolvedValue({ moderate: [], proactive: [] });
    vi.mocked(appService.getSetting).mockResolvedValue('false');
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
  });

  it('companion 侧栏入口不渲染，CompanionPairingSection 零挂载（防挂载即调）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    expect(await screen.findByRole('button', { name: '数据与隐私' })).toBeInTheDocument();
    // 侧栏入口隐藏（companion_get_status desktop-only）
    expect(screen.queryByRole('button', { name: '手机伴侣' })).not.toBeInTheDocument();
    // tab 级门控：内容组件零挂载（挂载即调 companion_*）
    expect(screen.queryByTestId('companion-section')).not.toBeInTheDocument();
    // 桌面可用的其余 tab 照常
    expect(screen.getByRole('button', { name: '模型服务配置' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'MCP Server' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '调度时间' })).toBeInTheDocument();
  });

  it('data tab：导出/导入区不渲染，销毁区保留（data_destroy 是 web-ok）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

    // 导出/导入区隐藏（data_export / pick_import_file desktop-only）
    expect(await screen.findByText('危险区域')).toBeInTheDocument();
    expect(screen.queryByText('导出数据')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /导出存档/ })).not.toBeInTheDocument();
    expect(screen.queryByText('导入数据')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /导入存档/ })).not.toBeInTheDocument();
    // 销毁入口保留（web-ok 命令）
    expect(screen.getByRole('button', { name: /销毁所有数据/ })).toBeInTheDocument();
    // 桌面导出/导入服务零调用
    expect(dataService.dataExport).not.toHaveBeenCalled();
    expect(dataService.pickImportFile).not.toHaveBeenCalled();
    expect(dataService.dataImport).not.toHaveBeenCalled();
  });

  it('data tab：销毁确认流程在浏览器可用（web-ok 命令）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
    fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

    // 桌面同款确认交互在浏览器照常（data_destroy 是 web-ok 命令）
    expect(
      await screen.findByText('此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。')
    ).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/输入"确认销毁"以继续/)).toBeInTheDocument();
    const confirmBtn = await screen.findByRole('button', { name: '确认销毁' });
    expect(confirmBtn).toBeInTheDocument();
  });
});
