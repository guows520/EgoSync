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

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
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
    webExport: vi.fn(),
    webImport: vi.fn(),
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

  // ── Story 17.3：云端数据备份入口（/api/export、/api/import 独立路由，
  //    !isTauriHost() 门控——web 模式可见、桌面模式不重复）──

  it('data tab：云端数据备份区在浏览器渲染（导出数据包/选择导出包入口）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

    // 云端备份区渲染（与桌面入口互斥——上方导出/导入区仍不渲染）
    expect(await screen.findByText('云端数据备份')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /导出数据包/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /选择导出包/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /导出存档/ })).not.toBeInTheDocument();
  });

  it('data tab：云端导出点击调用 webExport（blob 下载）', async () => {
    // jsdom 无 URL.createObjectURL——stub 下载管线（断言服务调用即可）
    const createObjectURL = vi.fn(() => 'blob:mock-url');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, configurable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, configurable: true });
    vi.mocked(dataService.webExport).mockResolvedValue(new Blob(['{"exportVersion":"1.0"}']));

    render(<GlobalSettingsModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
    fireEvent.click(await screen.findByRole('button', { name: /导出数据包/ }));

    await waitFor(() => expect(dataService.webExport).toHaveBeenCalledTimes(1));
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    // 桌面导出服务零调用（web 分支不经 cmd 通道）
    expect(dataService.dataExport).not.toHaveBeenCalled();
  });

  it('data tab：云端导入流程——选包确认后调用 webImport 并展示缺失密钥报告', async () => {
    vi.mocked(dataService.webImport).mockResolvedValue({
      imported: {
        rolesCount: 3, tasksCount: 5, memoriesCount: 7,
        conversationsCount: 2, messagesCount: 11,
      },
      missingSecrets: [
        {
          configName: 'DeepSeek 主力',
          apiKeyRef: 'llm_demo_api_key',
          message: "未找到配置 'DeepSeek 主力' 的 API Key（密钥缺失，常见于桌面→云端迁移后）。请前往 设置 → 模型服务配置，编辑该配置并重新保存密钥",
        },
      ],
    });

    render(<GlobalSettingsModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

    // 隐藏 file input：模拟选择导出包文件
    const input = await screen.findByTestId('web-import-file-input');
    const file = new File(['{"exportVersion":"1.0"}'], 'egosync-export-2026-09-21.json', {
      type: 'application/json',
    });
    fireEvent.change(input, { target: { files: [file] } });

    // 确认面出现（含文件名）
    expect(await screen.findByText('文件：egosync-export-2026-09-21.json')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => expect(dataService.webImport).toHaveBeenCalledTimes(1));
    // 导入结果 + 计数
    expect(await screen.findByText(/已恢复 3 个角色、5 个任务、7 条记忆/)).toBeInTheDocument();
    // 密钥缺失报告：逐项展示重录路径文案
    expect(screen.getByText(/有 1 个模型服务配置缺少 API Key/)).toBeInTheDocument();
    expect(screen.getByText(/未找到配置 'DeepSeek 主力' 的 API Key/)).toBeInTheDocument();
    // 桌面导入服务零调用
    expect(dataService.dataImport).not.toHaveBeenCalled();
    expect(dataService.pickImportFile).not.toHaveBeenCalled();
  });

// ── 评审补丁（验证缺口层 V4）：云端导入错误分支——与桌面孪生
//（dataExport 失败显示错误）同款验收，webImport 失败须呈现 AppError
// 单键 map 文案且不渲染结果面板。──
  it('webImport reject AppError 单键 map 时显示具体错误且不渲染结果面板', async () => {
    // 同文件前序用例的调用计数不清零——先清（断言只数本用例）
    vi.mocked(dataService.webImport).mockClear();
    vi.mocked(dataService.webImport).mockRejectedValue({
      ValidationError: '导出包格式不正确：exportVersion 不受支持',
    });

    render(<GlobalSettingsModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

    const input = await screen.findByTestId('web-import-file-input');
    const file = new File(['{"exportVersion":"9.9"}'], 'bad-package.json', {
      type: 'application/json',
    });
    fireEvent.change(input, { target: { files: [file] } });
    fireEvent.click(await screen.findByRole('button', { name: '确认导入' }));

    await waitFor(() => expect(dataService.webImport).toHaveBeenCalledTimes(1));
    // AppError 单键 map 的具体文案上屏（非笼统「导入失败」）
    expect(await screen.findByText(/导出包格式不正确/)).toBeInTheDocument();
    // 结果面板不出现（失败≠成功）
    expect(screen.queryByText(/已恢复/)).not.toBeInTheDocument();
    // 桌面导入服务零调用
    expect(dataService.dataImport).not.toHaveBeenCalled();
  });
});
