// web 版手机浏览器五症状自适应修复·症状④：LLM/MCP 卡片小屏图标钮钉孔
// （新建文件——GlobalSettingsModal*.test.tsx 三件套一行不改，冻结规矩
// 「新测试一律新建文件」）。
//
// 钉死（owner 确认「4 个图标」映射：徽章缩图标 + 测试连接/编辑/删除 3 图标钮）：
// - 测试连接/编辑/删除 = 单按钮 + 文字 span（max-md:sr-only）+ 图标（md:hidden）
//   ⇒ 可访问名在任意断点由 span 文本派生（既有按名点击零改动通过）；
// - 「当前启用」徽章含 md:hidden Check + max-md:sr-only 文字；
// - 三钮带 max-md:h-11 max-md:w-11（≥44px 触控红线）；
// - getByRole('button', { name:'测试连接' }) 唯一命中（名字典不漂移）。

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { llmConfigService } from '../../services/llmConfigService';
import { mcpService } from '../../services/mcpService';

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: {
    list: vi.fn(),
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
    list: vi.fn(),
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

const defaultLlmConfig = {
  id: 'llm-1',
  name: '默认配置',
  provider: 'openai_compatible' as const,
  baseUrl: 'https://api.example.com/v1',
  apiKeyRef: 'llm_demo_api_key',
  model: 'gpt-4o',
  networkLocation: 'internal' as const,
  isDefault: true,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const enabledMcpServer = {
  id: 'mcp-weather',
  name: '天气查询',
  serverType: 'sse' as const,
  commandOrUrl: 'https://weather.example/mcp',
  envRefs: '{}',
  description: '',
  enabled: true,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
};

/** 断言语义：文字 span（max-md:sr-only）+ 图标（md:hidden）+ 44px 触控类。 */
function expectIconButton(button: HTMLElement, label: string) {
  expect(button).toHaveClass(
    'max-md:h-11',
    'max-md:w-11',
    'max-md:p-0',
    'max-md:flex',
    'max-md:items-center',
    'max-md:justify-center',
  );

  const spans = Array.from(button.querySelectorAll('span'));
  const textSpan = spans.find(s => s.textContent === label);
  expect(textSpan).toBeDefined();
  expect(textSpan?.classList.contains('max-md:sr-only')).toBe(true);

  const icon = button.querySelector('svg');
  expect(icon).not.toBeNull();
  expect(icon?.classList.contains('md:hidden')).toBe(true);
}

describe('GlobalSettingsModal 小屏图标钮（web 移动自适应修复）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(llmConfigService.list).mockResolvedValue([defaultLlmConfig]);
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
  });

  it('LLM 卡：测试连接/编辑/删除图标化 + 徽章 Check 图标 + 按名唯一命中', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    // 按名唯一命中（可访问名由 span 文本派生——既有按名点击零改动通过）
    expectIconButton(await screen.findByRole('button', { name: '测试连接' }), '测试连接');
    expectIconButton(screen.getByRole('button', { name: '编辑' }), '编辑');
    expectIconButton(screen.getByRole('button', { name: '删除' }), '删除');

    // 「当前启用」徽章：md:hidden Check + max-md:sr-only 文字
    const badgeText = screen.getByText('当前启用');
    expect(badgeText.classList.contains('max-md:sr-only')).toBe(true);
    const badge = badgeText.parentElement;
    expect(badge?.classList.contains('max-md:flex')).toBe(true);
    expect(badge?.querySelector('svg')?.classList.contains('md:hidden')).toBe(true);

    // 抗挤压半边（review F2）：卡头 max-md:flex-wrap/gap-y-2 + 左组
    // max-md:min-w-0——长配置名下小屏可换行收缩，防症状④复现；
    // 类全部 max-md: 前缀化（review F1：桌面逐像素零变化红线）
    const card = screen.getByRole('button', { name: '删除' }).closest('.border');
    expect(card).not.toBeNull();
    const header = card?.firstElementChild;
    expect(header).toHaveClass('max-md:flex-wrap', 'max-md:gap-y-2');
    expect(header).not.toHaveClass('flex-wrap', 'gap-y-2');
    const leftGroup = header?.firstElementChild;
    expect(leftGroup).toHaveClass('max-md:min-w-0');
    expect(leftGroup).not.toHaveClass('min-w-0');
  });

  it('MCP 卡：测试连接/编辑/删除图标化（开关 toggle 不动）', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
    // 切 tab 后 LLM 卡卸载 ⇒ 按名唯一命中即名字典未漂移的实证
    expectIconButton(await screen.findByRole('button', { name: '测试连接' }), '测试连接');
    expectIconButton(screen.getByRole('button', { name: '编辑' }), '编辑');
    expectIconButton(screen.getByRole('button', { name: '删除' }), '删除');

    // 开关 toggle 保持紧凑 switch（未图标化）
    const toggle = await screen.findByRole('switch', { name: '停用 天气查询' });
    expect(toggle).toBeInTheDocument();

    await waitFor(() => expect(mcpService.list).toHaveBeenCalled());
  });
});
