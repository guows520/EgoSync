import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { llmConfigService } from '../../services/llmConfigService';
import { mcpService } from '../../services/mcpService';
import { dataService } from '../../services/dataService';

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

describe('GlobalSettingsModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(llmConfigService.list).mockResolvedValue([]);
    vi.mocked(mcpService.list).mockResolvedValue([]);
  });

  it('展示 MCP Server 标签入口并预留全局管理区块', async () => {
    render(
      <GlobalSettingsModal
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));

    expect(screen.getByRole('button', { name: 'MCP Server' })).toBeInTheDocument();
    expect(screen.getByText('MCP Server配置')).toBeInTheDocument();
    expect(screen.queryByText(/这是外部工具接入/)).not.toBeInTheDocument();
  });

  it('MCP 保存失败时保留 Error 实例中的具体错误消息', async () => {
    vi.mocked(mcpService.create).mockRejectedValue(new Error('MCP server 名称不能为空'));

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
    fireEvent.click(await screen.findByRole('button', { name: '添加 MCP server' }));
    fireEvent.click(screen.getByRole('button', { name: '保存 MCP server' }));

    expect(await screen.findByText('MCP server 名称不能为空')).toBeInTheDocument();
  });

  it('删除 MCP server 前要求确认', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    vi.mocked(mcpService.delete).mockResolvedValue(undefined);

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
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

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
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

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
    fireEvent.click(await screen.findByRole('button', { name: '编辑' }));

    expect(screen.queryByRole('checkbox', { name: '启用' })).not.toBeInTheDocument();
  });

  it('在 MCP 详情页点击右上角关闭返回 MCP 列表而不是关闭全局设置', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    const onClose = vi.fn();

    render(<GlobalSettingsModal onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
    fireEvent.click(await screen.findByRole('button', { name: '编辑' }));
    fireEvent.click(screen.getByLabelText('返回 MCP 工具列表'));

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('天气查询')).toBeInTheDocument();
    expect(screen.queryByText('编辑 MCP server')).not.toBeInTheDocument();
  });

  it('支持从标准 mcpServers JSON 导入 MCP server 配置并填充表单', async () => {
    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
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

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
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

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));

    expect(await screen.findByText('描述：暂无描述')).toBeInTheDocument();
  });

  it('MCP 测试连接进行中时使用不受减少动态效果影响的 loading 动画', async () => {
    vi.mocked(mcpService.list).mockResolvedValue([enabledMcpServer]);
    vi.mocked(mcpService.test).mockImplementation(() => new Promise(() => {}));

    render(<GlobalSettingsModal onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'MCP Server' }));
    fireEvent.click(await screen.findByRole('button', { name: '测试连接' }));

    const testingButton = await screen.findByRole('button', { name: '' });
    expect(testingButton.firstElementChild).toHaveClass('animate-loading-spin');
  });

  it('loading 动画定义自有 keyframes，避免脱离 Tailwind animate-spin 后静止', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/index.css'), 'utf8');

    expect(css).toContain('@keyframes loading-spin');
    expect(css).toMatch(/\.animate-loading-spin\s*\{[^}]*animation:\s*loading-spin 1s linear infinite !important;/s);
  });

  describe('数据导出', () => {
    beforeEach(() => {
      vi.mocked(llmConfigService.list).mockResolvedValue([]);
      vi.mocked(mcpService.list).mockResolvedValue([]);
    });

    it('点击导出存档按钮后显示格式选择 UI', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      expect(await screen.findByText('选择导出格式（可多选）')).toBeInTheDocument();
      expect(screen.getByLabelText(/SQLite 备份/)).toBeInTheDocument();
      expect(screen.getByLabelText(/JSON 格式/)).toBeInTheDocument();
      expect(screen.getByLabelText(/Markdown 报告/)).toBeInTheDocument();
    });

    it('未选择格式时确认导出按钮被禁用', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const confirmBtn = await screen.findByRole('button', { name: '确认导出' });
      expect(confirmBtn).toBeDisabled();
    });

    it('选择格式后点击确认导出调用 dataService', async () => {
      const mockResult = {
        files: ['/tmp/egosync-export-2026-06-27.json'],
        sqlitePath: null,
        jsonPath: '/tmp/egosync-export-2026-06-27.json',
        markdownPath: null,
      };
      vi.mocked(dataService.dataExport).mockResolvedValue(mockResult);

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const jsonCheckbox = await screen.findByLabelText(/JSON 格式/);
      fireEvent.click(jsonCheckbox);

      const confirmBtn = screen.getByRole('button', { name: '确认导出' });
      fireEvent.click(confirmBtn);

      await waitFor(() => {
        expect(dataService.dataExport).toHaveBeenCalledWith(['json']);
      });

      expect(await screen.findByText('导出完成')).toBeInTheDocument();
      expect(screen.getByText('/tmp/egosync-export-2026-06-27.json')).toBeInTheDocument();
    });

    it('导出失败时显示错误消息', async () => {
      vi.mocked(dataService.dataExport).mockRejectedValue({ DbError: '数据库读取失败' });

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const mdCheckbox = await screen.findByLabelText(/Markdown 报告/);
      fireEvent.click(mdCheckbox);

      fireEvent.click(screen.getByRole('button', { name: '确认导出' }));

      await waitFor(() => {
        expect(dataService.dataExport).toHaveBeenCalledWith(['markdown']);
      });

      expect(await screen.findByText('数据库读取失败')).toBeInTheDocument();
    });

    it('用户取消目录选择（返回空结果）时不显示成功或错误提示', async () => {
      // 取消目录选择是正常操作，后端返回 files 为空的结果，前端应静默处理。
      vi.mocked(dataService.dataExport).mockResolvedValue({
        files: [],
        sqlitePath: null,
        jsonPath: null,
        markdownPath: null,
      });

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const jsonCheckbox = await screen.findByLabelText(/JSON 格式/);
      fireEvent.click(jsonCheckbox);
      fireEvent.click(screen.getByRole('button', { name: '确认导出' }));

      await waitFor(() => {
        expect(dataService.dataExport).toHaveBeenCalledWith(['json']);
      });

      expect(screen.queryByText('导出完成')).not.toBeInTheDocument();
      expect(screen.queryByText(/导出失败/)).not.toBeInTheDocument();
    });

    it('取消格式选择后返回导出按钮', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      expect(await screen.findByText('选择导出格式（可多选）')).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: '取消' }));

      expect(screen.queryByText('选择导出格式（可多选）')).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /导出存档/ })).toBeInTheDocument();
    });

    it('数据 Tab 同时显示导出区域和危险区域', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      expect(await screen.findByText('导出数据')).toBeInTheDocument();
      expect(screen.getByText('危险区域')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /导出存档/ })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /销毁所有数据/ })).toBeInTheDocument();
    });

    it('导出执行中显示 loading 状态（spinner + 导出中...）', async () => {
      let resolveExport: (value: any) => void = () => {};
      vi.mocked(dataService.dataExport).mockImplementation(
        () => new Promise(resolve => { resolveExport = resolve; })
      );

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const jsonCheckbox = await screen.findByLabelText(/JSON 格式/);
      fireEvent.click(jsonCheckbox);
      fireEvent.click(screen.getByRole('button', { name: '确认导出' }));

      await waitFor(() => {
        expect(screen.getByText('导出中...')).toBeInTheDocument();
      });

      await act(async () => {
        resolveExport({ files: [], sqlitePath: null, jsonPath: null, markdownPath: null });
      });
    });

    it('多格式导出成功后显示所有文件路径', async () => {
      vi.mocked(dataService.dataExport).mockResolvedValue({
        files: ['/tmp/egosync-export.sqlite', '/tmp/egosync-export.json', '/tmp/egosync-export.md'],
        sqlitePath: '/tmp/egosync-export.sqlite',
        jsonPath: '/tmp/egosync-export.json',
        markdownPath: '/tmp/egosync-export.md',
      });

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /导出存档/ }));

      const sqliteCheckbox = await screen.findByLabelText(/SQLite 备份/);
      fireEvent.click(sqliteCheckbox);
      fireEvent.click(screen.getByLabelText(/JSON 格式/));
      fireEvent.click(screen.getByLabelText(/Markdown 报告/));

      fireEvent.click(screen.getByRole('button', { name: '确认导出' }));

      await waitFor(() => {
        expect(dataService.dataExport).toHaveBeenCalledWith(['sqlite', 'json', 'markdown']);
      });

      expect(await screen.findByText('导出完成')).toBeInTheDocument();
      expect(screen.getByText('/tmp/egosync-export.sqlite')).toBeInTheDocument();
      expect(screen.getByText('/tmp/egosync-export.json')).toBeInTheDocument();
      expect(screen.getByText('/tmp/egosync-export.md')).toBeInTheDocument();
    });
  });

  describe('数据销毁', () => {
    beforeEach(() => {
      vi.mocked(llmConfigService.list).mockResolvedValue([]);
      vi.mocked(mcpService.list).mockResolvedValue([]);
    });

    it('点击销毁按钮后显示确认区域和警告文字', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      expect(await screen.findByText('此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。')).toBeInTheDocument();
      expect(screen.getByPlaceholderText(/输入"确认销毁"以继续/)).toBeInTheDocument();
    });

    it('确认按钮在输入正确文字前禁用', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      const confirmBtn = await screen.findByRole('button', { name: '确认销毁' });
      expect(confirmBtn).toBeDisabled();

      fireEvent.change(screen.getByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '错误文字' },
      });
      expect(confirmBtn).toBeDisabled();

      fireEvent.change(screen.getByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '确认销毁' },
      });
      expect(confirmBtn).not.toBeDisabled();
    });

    it('输入"确认销毁"后点击确认调用 dataDestroy', async () => {
      vi.mocked(dataService.dataDestroy).mockResolvedValue(undefined);

      render(<GlobalSettingsModal onClose={vi.fn()} onDataDestroyed={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      fireEvent.change(await screen.findByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '确认销毁' },
      });
      fireEvent.click(screen.getByRole('button', { name: '确认销毁' }));

      await waitFor(() => {
        expect(dataService.dataDestroy).toHaveBeenCalled();
      });
    });

    it('销毁成功后调用 onDataDestroyed 回调', async () => {
      vi.mocked(dataService.dataDestroy).mockResolvedValue(undefined);
      const onDataDestroyed = vi.fn();

      render(<GlobalSettingsModal onClose={vi.fn()} onDataDestroyed={onDataDestroyed} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      fireEvent.change(await screen.findByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '确认销毁' },
      });
      fireEvent.click(screen.getByRole('button', { name: '确认销毁' }));

      await waitFor(() => {
        expect(onDataDestroyed).toHaveBeenCalled();
      });
    });

    it('销毁失败时显示错误消息', async () => {
      vi.mocked(dataService.dataDestroy).mockRejectedValue({ DbError: '数据库删除失败' });

      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      fireEvent.change(await screen.findByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '确认销毁' },
      });
      fireEvent.click(screen.getByRole('button', { name: '确认销毁' }));

      expect(await screen.findByText('数据库删除失败')).toBeInTheDocument();
    });

    it('取消确认后返回销毁按钮', async () => {
      render(<GlobalSettingsModal onClose={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      expect(await screen.findByText('此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。')).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: '取消' }));

      expect(screen.queryByText('此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。')).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /销毁所有数据/ })).toBeInTheDocument();
    });

    it('销毁执行中显示 loading 状态（spinner + 销毁中...）', async () => {
      let resolveDestroy: (value: any) => void = () => {};
      vi.mocked(dataService.dataDestroy).mockImplementation(
        () => new Promise(resolve => { resolveDestroy = resolve; })
      );

      render(<GlobalSettingsModal onClose={vi.fn()} onDataDestroyed={vi.fn()} />);

      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));
      fireEvent.click(await screen.findByRole('button', { name: /销毁所有数据/ }));

      fireEvent.change(await screen.findByPlaceholderText(/输入"确认销毁"以继续/), {
        target: { value: '确认销毁' },
      });
      fireEvent.click(screen.getByRole('button', { name: '确认销毁' }));

      await waitFor(() => {
        expect(screen.getByText('销毁中...')).toBeInTheDocument();
      });

      await act(async () => {
        resolveDestroy(undefined);
      });
    });
  });

  describe('数据导入', () => {
    beforeEach(() => {
      vi.mocked(dataService.pickImportFile).mockReset();
      vi.mocked(dataService.dataImport).mockReset();
    });

    it('点击导入存档后调用文件选择并显示确认弹窗', async () => {
      vi.mocked(dataService.pickImportFile).mockResolvedValue('/tmp/archive.json');

      render(<GlobalSettingsModal onClose={vi.fn()} />);
      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: /导入存档/ }));

      await waitFor(() => {
        expect(dataService.pickImportFile).toHaveBeenCalled();
        expect(screen.getByText('导入将覆盖当前所有数据（导入前已自动备份）。确认要继续吗？')).toBeInTheDocument();
      });
    });

    it('确认导入调用 dataImport 并显示统计信息', async () => {
      vi.mocked(dataService.pickImportFile).mockResolvedValue('/tmp/archive.json');
      vi.mocked(dataService.dataImport).mockResolvedValue({
        rolesCount: 2, tasksCount: 5, memoriesCount: 3, conversationsCount: 1, messagesCount: 10,
      });
      const onDataImported = vi.fn();

      render(<GlobalSettingsModal onClose={vi.fn()} onDataImported={onDataImported} />);
      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: /导入存档/ }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: '确认导入' })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

      await waitFor(() => {
        expect(dataService.dataImport).toHaveBeenCalledWith('/tmp/archive.json');
        expect(screen.getByText(/已恢复 2 个角色/)).toBeInTheDocument();
        expect(onDataImported).toHaveBeenCalled();
      });
    });

    it('导入失败显示错误消息', async () => {
      vi.mocked(dataService.pickImportFile).mockResolvedValue('/tmp/archive.json');
      vi.mocked(dataService.dataImport).mockRejectedValue({ ValidationError: '文件格式不兼容' });

      render(<GlobalSettingsModal onClose={vi.fn()} />);
      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: /导入存档/ }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: '确认导入' })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

      await waitFor(() => {
        expect(screen.getByText('文件格式不兼容')).toBeInTheDocument();
      });
    });

    it('取消确认无副作用', async () => {
      vi.mocked(dataService.pickImportFile).mockResolvedValue('/tmp/archive.json');

      render(<GlobalSettingsModal onClose={vi.fn()} />);
      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: /导入存档/ }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: '取消' })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: '取消' }));

      await waitFor(() => {
        expect(dataService.dataImport).not.toHaveBeenCalled();
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
    });

    it('文件选择取消时不显示确认弹窗', async () => {
      vi.mocked(dataService.pickImportFile).mockResolvedValue(null);

      render(<GlobalSettingsModal onClose={vi.fn()} />);
      fireEvent.click(screen.getByRole('button', { name: '数据与隐私' }));

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: /导入存档/ }));

      await waitFor(() => {
        expect(dataService.pickImportFile).toHaveBeenCalled();
      });

      expect(screen.queryByText('导入将覆盖当前所有数据')).not.toBeInTheDocument();
    });
  });
});
