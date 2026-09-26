// web 版手机浏览器控件密度降档·症状二/三（2026-09-26 owner 反馈）：
// LLM/MCP 卡片小屏图标钮钉孔（在五症状修复·症状④新建的钉孔文件上按
// owner 授权同型改写——ActionCard.mobile.test.tsx / 本文件皆属「既有
// 钉孔同型改写」授权链，改写内容见 spec-web-mobile-control-density.md）。
//
// 钉死（owner 裁决一：徽章去背景只留勾、四图标紧凑靠右成一簇；裁决二：
// 图标钮 44px→36px 触控降档，档位 B，显式重协商 16.4 冻结 ≥44×44px 红线
// ——局部特例勿外推；MCP 卡同步降档保两卡同族一致）：
// - 测试连接/编辑/删除 = 单按钮 + 文字 span（max-md:sr-only）+ 图标（md:hidden）
//   ⇒ 可访问名在任意断点由 span 文本派生（既有按名点击零改动通过）；
// - 「当前启用」双断点双形态：桌面 = 名字旁 Chip（max-md:hidden）；小屏 =
//   右组首勾簇（md:hidden，Check + max-md:sr-only 可访问名）与三钮同组
//   ⇒ 「4 图标紧凑靠右」的 DOM 实证；桌面逐像素零变化；
// - 三钮带 max-md:h-9 max-md:w-9（36px 触控档位 B）；右组 max-md:gap-1 收紧；
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

/** 断言语义：文字 span（max-md:sr-only）+ 图标（md:hidden）+ 36px 触控类（档位 B）。 */
function expectIconButton(button: HTMLElement, label: string) {
  expect(button).toHaveClass(
    'max-md:h-9',
    'max-md:w-9',
    'max-md:p-0',
    'max-md:flex',
    'max-md:items-center',
    'max-md:justify-center',
  );
  // 36px 档位痕迹钉死：非 44px 亦非 40px（逐类拆开——多类 not.toHaveClass 是 OR 语义，
  // 只缺一类即通过，无法单独捕获「仅 h 被改回旧值」的档位回退）
  expect(button).not.toHaveClass('max-md:h-11');
  expect(button).not.toHaveClass('max-md:w-11');
  expect(button).not.toHaveClass('max-md:h-10');
  expect(button).not.toHaveClass('max-md:w-10');

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

    // 「当前启用」双断点双形态（owner 裁决一：去背景只留勾、四图标紧凑靠右）：
    // 桌面 = 名字旁 Chip（max-md:hidden、文本常显、无图标、底/字色原样）；
    // 小屏 = 右组首勾簇（md:hidden，Check + max-md:sr-only 可访问名）——与
    // 三钮同组即「4 图标靠右一簇」的 DOM 实证；桌面逐像素零变化。
    const badgeTexts = screen.getAllByText('当前启用');
    expect(badgeTexts).toHaveLength(2);
    // 按类特征定位（不依赖 DOM 序）：Chip = 桌面专属（max-md:hidden），
    // 勾簇文字 = 小屏专属（max-md:sr-only）
    const desktopChip = badgeTexts.find(t => t.classList.contains('max-md:hidden'));
    expect(desktopChip).toBeDefined();
    expect(desktopChip?.classList.contains('max-md:sr-only')).toBe(false);
    expect(desktopChip?.classList.contains('bg-indigo-100')).toBe(true);
    expect(desktopChip?.querySelector('svg')).toBeNull(); // 勾图标不再挂左组
    const mobileCheckText = badgeTexts.find(t => t.classList.contains('max-md:sr-only'));
    expect(mobileCheckText).toBeDefined();
    const checkCluster = mobileCheckText?.parentElement;
    expect(checkCluster?.classList.contains('md:hidden')).toBe(true);
    expect(checkCluster?.classList.contains('max-md:flex')).toBe(true);
    expect(checkCluster?.querySelector('svg')?.classList.contains('text-indigo-600')).toBe(true);
    // 勾簇与三钮同处右组 ⇒ 紧凑靠右（裁决一）；桌面基线 gap-2 保留 +
    // 追加 max-md:gap-1（桌面像素红线守门：基线类丢失则此处红）
    const rightGroup = checkCluster?.parentElement;
    expect(rightGroup).toBe(screen.getByRole('button', { name: '测试连接' }).parentElement);
    expect(rightGroup).toHaveClass('gap-2', 'max-md:gap-1');

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
    // MCP 右组（toggle + 三钮）同步 max-md:gap-1——与 LLM 卡同族一致（降档批次）；
    // 桌面基线 gap-2 保留（红线守门：写成无条件 gap-1 则此处红）
    const mcpRightGroup = toggle.parentElement;
    expect(mcpRightGroup).toHaveClass('gap-2', 'max-md:gap-1');

    await waitFor(() => expect(mcpService.list).toHaveBeenCalled());
  });
});
