import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { llmConfigService } from '../../services/llmConfigService';
import { roleService } from '../../services/roleService';
import { mcpService } from '../../services/mcpService';
import type { Role } from '../../types/role';

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    setDefault: vi.fn(),
    testConnection: vi.fn(),
  },
}));

vi.mock('../../services/roleService', () => ({
  roleService: {
    listArchived: vi.fn(),
    restore: vi.fn(),
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

const archivedRole: Role = {
  id: 'role-archived',
  name: '学习者',
  icon: 'book-open',
  color: '#10B981',
  goal: '保持学习节奏',
  personalityPrompt: '',
  status: 'archived',
  energy: 70,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: '2026-01-02T00:00:00Z',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
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

describe('GlobalSettingsModal archived roles', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(llmConfigService.list).mockResolvedValue([]);
    vi.mocked(roleService.listArchived).mockResolvedValue([archivedRole]);
    vi.mocked(mcpService.list).mockResolvedValue([]);
  });

  it('打开设置时加载归档角色并可恢复', async () => {
    const onRestoreRole = vi.fn().mockResolvedValue(undefined);

    render(
      <GlobalSettingsModal
        onClose={vi.fn()}
        onRestoreRole={onRestoreRole}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: '数据与主权' }));

    expect(await screen.findByText('学习者')).toBeInTheDocument();
    expect(screen.getByText('保持学习节奏')).toBeInTheDocument();
    expect(screen.getByText(/归档时间：/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /恢复/ }));

    await waitFor(() => {
      expect(onRestoreRole).toHaveBeenCalledWith('role-archived');
      expect(roleService.listArchived).toHaveBeenCalledTimes(2);
    });
  });

  it('没有归档角色时显示空态', async () => {
    vi.mocked(roleService.listArchived).mockResolvedValue([]);

    render(<GlobalSettingsModal onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: '数据与主权' }));

    expect(await screen.findByText('暂无归档角色')).toBeInTheDocument();
  });

  it('展示 MCP 工具标签入口并预留全局管理区块', async () => {
    render(
      <GlobalSettingsModal
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));

    expect(screen.getByRole('button', { name: 'MCP 工具' })).toBeInTheDocument();
    expect(screen.getByText('MCP 工具配置')).toBeInTheDocument();
    expect(screen.getByText(/这是外部工具接入，不是 EgoSync 内部 create_role\/delegate 工具/)).toBeInTheDocument();
  });

  it('MCP 保存失败时保留 Error 实例中的具体错误消息', async () => {
    vi.mocked(mcpService.create).mockRejectedValue(new Error('MCP server 名称不能为空'));

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '添加 MCP server' }));
    fireEvent.click(screen.getByRole('button', { name: '保存 MCP server' }));

    expect(await screen.findByText('MCP server 名称不能为空')).toBeInTheDocument();
  });

  it('删除 MCP server 前要求确认', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    vi.mocked(mcpService.delete).mockResolvedValue(undefined);

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '删除' }));

    expect(mcpService.delete).not.toHaveBeenCalled();
    expect(screen.getByText('确认删除 MCP server？')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(mcpService.delete).toHaveBeenCalledWith('mcp-weather');
    });
  });

  it('在 MCP 列表横条中用开关启用和停用 server', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    vi.mocked(mcpService.update).mockResolvedValue({ ...enabledMcpServer, enabled: false });

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    const toggle = await screen.findByRole('switch', { name: '停用 天气查询' });

    expect(toggle).toHaveAttribute('aria-checked', 'true');
    expect(toggle.parentElement?.firstElementChild).toBe(toggle);
    expect(toggle.firstElementChild).toHaveClass('left-0.5');

    fireEvent.click(toggle);

    await waitFor(() => {
      expect(mcpService.update).toHaveBeenCalledWith('mcp-weather', { enabled: false });
    });
  });

  it('MCP 详情页不再显示启用复选框', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '编辑' }));

    expect(screen.queryByRole('checkbox', { name: '启用' })).not.toBeInTheDocument();
  });

  it('在 MCP 详情页点击右上角关闭返回 MCP 列表而不是关闭全局设置', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    const onClose = vi.fn();

    render(<GlobalSettingsModal onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '编辑' }));
    fireEvent.click(screen.getByLabelText('返回 MCP 工具列表'));

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('天气查询')).toBeInTheDocument();
    expect(screen.queryByText('编辑 MCP server')).not.toBeInTheDocument();
  });

  it('支持从标准 mcpServers JSON 导入 MCP server 配置并填充表单', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '添加 MCP server' }));
    fireEvent.change(screen.getByLabelText('MCP JSON'), {
      target: {
        value: JSON.stringify({
          mcpServers: {
            '12306-mcp': {
              type: 'streamable_http',
              url: 'https://mcp.api-inference.modelscope.net/abf4b0a8cb864b/mcp',
            },
          },
        }),
      },
    });
    fireEvent.click(screen.getByRole('button', { name: '导入 JSON' }));

    expect(screen.getByDisplayValue('12306-mcp')).toBeInTheDocument();
    expect(screen.getByDisplayValue('https://mcp.api-inference.modelscope.net/abf4b0a8cb864b/mcp')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Streamable HTTP')).toBeInTheDocument();
  });

  it('支持从 JSON 导入 MCP server 配置并填充表单', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '添加 MCP server' }));
    fireEvent.change(screen.getByLabelText('MCP JSON'), {
      target: {
        value: JSON.stringify({
          name: '地图天气',
          type: 'streamable_http',
          url: 'https://weather.example/mcp',
          env: { TOKEN: 'env:WEATHER_TOKEN' },
          description: '查询实时天气',
        }),
      },
    });
    fireEvent.click(screen.getByRole('button', { name: '导入 JSON' }));

    expect(screen.getByDisplayValue('地图天气')).toBeInTheDocument();
    expect(screen.getByDisplayValue('https://weather.example/mcp')).toBeInTheDocument();
    expect(screen.getByDisplayValue('查询实时天气')).toBeInTheDocument();
    expect(screen.getByDisplayValue(/"TOKEN": "env:WEATHER_TOKEN"/)).toBeInTheDocument();
  });

  it('MCP 列表明确展示描述字段的空态', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));

    expect(await screen.findByText('描述：暂无描述')).toBeInTheDocument();
  });

  it('MCP 测试连接进行中时使用不受减少动态效果影响的 loading 动画', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    vi.mocked(mcpService.test).mockImplementation(() => new Promise(() => {}));

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP 工具' }));
    fireEvent.click(await screen.findByRole('button', { name: '测试连接' }));

    const testingButton = await screen.findByRole('button', { name: '' });
    expect(testingButton.firstElementChild).toHaveClass('animate-loading-spin');
  });

  it('loading 动画定义自有 keyframes，避免脱离 Tailwind animate-spin 后静止', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/index.css'), 'utf8');

    expect(css).toContain('@keyframes loading-spin');
    expect(css).toMatch(/\.animate-loading-spin\s*\{[^}]*animation:\s*loading-spin 1s linear infinite !important;/s);
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{[\s\S]*\.animate-loading-spin\s*\{[^}]*animation:\s*loading-spin 1s linear infinite !important;/);
  });
});
