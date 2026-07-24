import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { SettingsTab } from './SettingsTab';
import { roleService } from '../../services/roleService';
import { skillService } from '../../services/skillService';
import { mcpService } from '../../services/mcpService';
import type { Role } from '../../types/role';
import type { McpServer } from '../../types/mcp';
import type { SkillRegistryEntry } from '../../types/skill';

const tauriEventHandlers = vi.hoisted(() => ({
  skillRegistryUpdated: undefined as ((payload: { ownerId: string }) => void) | undefined,
}));

vi.mock('../../hooks/useTauriEvent', () => ({
  useTauriEvent: vi.fn((eventName: string, handler: (payload: { ownerId: string }) => void) => {
    if (eventName === 'skill-registry-updated') tauriEventHandlers.skillRegistryUpdated = handler;
  }),
}));

vi.mock('../../services/roleService', () => ({
  roleService: {
    create: vi.fn(),
    list: vi.fn(),
    update: vi.fn(),
    updateSkills: vi.fn(),
    updateProactivity: vi.fn(),
  },
}));

vi.mock('../../services/skillService', () => ({
  skillService: {
    notifyScopeUpdated: vi.fn().mockResolvedValue(undefined),
    listRegistry: vi.fn(),
    listForRole: vi.fn(),
    pickCustomDirectory: vi.fn(),
    previewCustom: vi.fn(),
    importCustom: vi.fn(),
    discoverOpencode: vi.fn(),
    importOpencode: vi.fn(),
    delete: vi.fn(),
    removeFromRole: vi.fn(),
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

const baseRole: Role = {
  id: 'role-1',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  goal: '管理产品规划',
  personalityPrompt: '',
  status: 'active',
  energy: 100,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const secondRole: Role = {
  ...baseRole,
  id: 'role-2',
  name: '学习者',
  icon: 'graduation-cap',
  color: '#10B981',
  goal: '保持学习节奏',
};

const updatedRole: Role = {
  ...baseRole,
  name: '学习者',
  icon: 'graduation-cap',
  color: '#10B981',
  goal: '保持学习节奏',
  updatedAt: '2026-01-02T00:00:00Z',
};

const customSkill: SkillRegistryEntry = {
  id: 'skill-1',
  name: 'daily-review',
  description: '日复盘助手',
  sourceType: 'custom',
  managedPath: 'managed/daily-review/SKILL.md',
  contentHash: 'hash-1',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const boundMcpServer: McpServer = {
  id: 'mcp-calendar',
  name: '日历 MCP',
  serverType: 'sse',
  commandOrUrl: 'http://localhost:8000/sse',
  envRefs: '{"TOKEN":"env:CALENDAR_TOKEN"}',
  description: '读取日历安排',
  enabled: true,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const availableMcpServer: McpServer = {
  ...boundMcpServer,
  id: 'mcp-mail',
  name: '邮件 MCP',
  commandOrUrl: 'http://localhost:8001/sse',
  description: '读取邮件摘要',
};

describe('SettingsTab role CRUD actions', () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    localStorage.clear();
    tauriEventHandlers.skillRegistryUpdated = undefined;
    vi.mocked(skillService.listRegistry).mockResolvedValue([]);
    vi.mocked(skillService.listForRole).mockResolvedValue([]);
    const mockedSkillService = skillService as typeof skillService & {
      discoverOpencode: ReturnType<typeof vi.fn>;
      importOpencode: ReturnType<typeof vi.fn>;
    };
    mockedSkillService.discoverOpencode.mockResolvedValue({ items: [], skipped: { total: 0, reasons: [] } });
    mockedSkillService.importOpencode.mockResolvedValue({ status: 'imported', entry: null, synced: true });
    vi.mocked(mcpService.listForRole).mockResolvedValue([]);
    vi.mocked(mcpService.listAvailableForRole).mockResolvedValue([]);
    vi.mocked(mcpService.addToRole).mockResolvedValue(undefined);
    vi.mocked(mcpService.removeFromRole).mockResolvedValue(undefined);
  });

  it('设置页打开时，Agent 创建 Skill 后刷新可用 Skill 列表', async () => {
    const customSkill: SkillRegistryEntry = { id: 'skill-created', name: 'uat-greeting', description: '问候', sourceType: 'custom', managedPath: 'managed', contentHash: 'hash', createdAt: '', updatedAt: '' };
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    render(<SettingsTab role={{ ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-created"]}' }} activeRoleCount={2} />);
    await waitFor(() => expect(skillService.listForRole).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('switch', { name: 'uat-greeting' })).not.toBeInTheDocument();
    await act(async () => { tauriEventHandlers.skillRegistryUpdated?.({ ownerId: 'role-1' }); });
    expect(await screen.findByRole('switch', { name: 'uat-greeting' })).toHaveAttribute('aria-checked', 'true');
    expect(skillService.listForRole).toHaveBeenCalledTimes(2);
  });

  it('保存后调用 update 并回传更新后的角色', async () => {
    vi.mocked(roleService.update).mockResolvedValue(updatedRole);
    const onUpdateRole = vi.fn();

    render(
      <SettingsTab
        role={baseRole}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.change(screen.getByLabelText('名称'), { target: { value: '学习者' } });
    fireEvent.change(screen.getByLabelText('目标'), { target: { value: '保持学习节奏' } });
    fireEvent.change(screen.getByLabelText('角色个性描述'), { target: { value: '简洁专业，先判断优先级再给建议' } });
    fireEvent.click(screen.getByTitle('学习'));
    fireEvent.click(screen.getByText('翠绿'));
    fireEvent.click(screen.getByRole('button', { name: '保存角色信息' }));

    await waitFor(() => {
      expect(roleService.update).toHaveBeenCalledWith('role-1', {
        name: '学习者',
        icon: 'graduation-cap',
        color: '#10B981',
        goal: '保持学习节奏',
        personalityPrompt: '简洁专业，先判断优先级再给建议',
      });
    });
    expect(onUpdateRole).toHaveBeenCalledWith(updatedRole);
    expect(await screen.findByText('已保存')).toBeInTheDocument();
  });

  it('展示并持久化两个默认元 Skill 开关，同时保留自定义 Skill ids', async () => {
    const onUpdateRole = vi.fn();
    const skillsRole: Role = {
      ...baseRole,
      skillsConfig: '{"meta":{"findSkills":false,"skillCreator":true},"enabledSkillIds":["skill-1"],"permissions":{"bash":"ask"}}',
    };
    const updatedSkillsRole: Role = {
      ...skillsRole,
      skillsConfig: '{"meta":{"findSkills":true,"skillCreator":true},"enabledSkillIds":["skill-1"],"permissions":{"bash":"ask"}}',
    };
    vi.mocked(roleService.updateSkills).mockResolvedValue(updatedSkillsRole);

    render(
      <SettingsTab
        role={skillsRole}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    expect(screen.getByText('Skill 配置')).toBeInTheDocument();
    expect(screen.queryByText('Skill 插件配置')).not.toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'find-skills' })).toBeInTheDocument();
    expect(screen.getByText('Vercel 官方')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'skill-creator' })).toBeInTheDocument();
    expect(screen.getByText('Anthropic 官方')).toBeInTheDocument();
    expect(screen.queryByDisplayValue('sk-xxxx-xxxx-xxxx-xxxx')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('switch', { name: 'find-skills' }));

    await waitFor(() => {
      expect(roleService.updateSkills).toHaveBeenCalledWith('role-1', {
        findSkills: true,
        skillCreator: true,
        enabledSkillIds: ['skill-1'],
      });
    });
    expect(onUpdateRole).toHaveBeenCalledWith(updatedSkillsRole);
    expect(await screen.findByText('Skill 配置已保存')).toBeInTheDocument();
    expect(screen.queryByText('保存中...')).not.toBeInTheDocument();
  });

  it('只提供 Skill 文件夹导入入口，不再展示单文件导入控件', async () => {
    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByRole('button', { name: '选择skill文件夹' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '导入自定义 Skill' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('导入自定义 Skill', { selector: 'input' })).not.toBeInTheDocument();
  });

  it('选择skill文件夹时通过原生命令读取 SKILL.md 并展示预览', async () => {
    const onUpdateRole = vi.fn();
    const originalPicker = window.showDirectoryPicker;
    const browserPicker = vi.fn();
    Object.defineProperty(window, 'showDirectoryPicker', {
      configurable: true,
      value: browserPicker,
    });
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
      sourcePath: 'C:/UAT/skills/daily-review',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      runtimeReady: true,
      runtimeError: null,
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(roleService.list).mockResolvedValue([
      { ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
    ]);

    render(
      <SettingsTab
        role={baseRole}
        activeRoles={[baseRole, secondRole]}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));

    expect(await screen.findByText('预览：daily-review')).toBeInTheDocument();
    expect(screen.getByText('日复盘助手')).toBeInTheDocument();
    expect(skillService.pickCustomDirectory).toHaveBeenCalledTimes(1);
    expect(browserPicker).not.toHaveBeenCalled();
    expect(screen.getByRole('checkbox', { name: '全部角色' })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: '产品经理' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '学习者' })).not.toBeChecked();

    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        sourcePath: 'C:/UAT/skills/daily-review',
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['role-1'] },
      });
    });
    await waitFor(() => {
      expect(roleService.list).toHaveBeenCalledTimes(1);
    });
    expect(roleService.updateSkills).not.toHaveBeenCalled();
    expect(await screen.findByText('自定义 Skill 已导入并可立即使用')).toBeInTheDocument();

    Object.defineProperty(window, 'showDirectoryPicker', {
      configurable: true,
      value: originalPicker,
    });
  });

  it('导入 Skill 时勾选全部角色会持久化全部角色范围', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
      sourcePath: 'C:/UAT/skills/daily-review',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      runtimeReady: true,
      runtimeError: null,
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(roleService.list).mockResolvedValue([
      { ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
      { ...secondRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
    ]);

    render(
      <SettingsTab
        role={baseRole}
        activeRoles={[baseRole, secondRole]}
        activeRoleCount={2}
      />
    );

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    fireEvent.click(await screen.findByRole('checkbox', { name: '全部角色' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        sourcePath: 'C:/UAT/skills/daily-review',
        overwriteExisting: false,
        roleScope: { allRoles: true, roleIds: [] },
      });
    });
  });

  it('只展示当前角色已启用 MCP，并通过搜索选择器添加可用 MCP', async () => {
    vi.mocked(mcpService.listForRole)
      .mockResolvedValueOnce([boundMcpServer])
      .mockResolvedValueOnce([boundMcpServer, availableMcpServer]);
    vi.mocked(mcpService.listAvailableForRole)
      .mockResolvedValueOnce([availableMcpServer])
      .mockResolvedValueOnce([]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByText('日历 MCP')).toBeInTheDocument();
    expect(screen.queryByText('邮件 MCP')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '添加 MCP server' }));
    expect(await screen.findByText('邮件 MCP')).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText('搜索 MCP server'), { target: { value: '邮件' } });
    fireEvent.click(screen.getByRole('button', { name: '添加 邮件 MCP' }));

    await waitFor(() => {
      expect(mcpService.addToRole).toHaveBeenCalledWith('role-1', 'mcp-mail');
    });
    await waitFor(() => {
      expect(mcpService.listForRole).toHaveBeenCalledTimes(2);
    });
    expect(await screen.findByText('邮件 MCP')).toBeInTheDocument();
  });

  it('可从当前角色移除已启用 MCP，不影响全局配置', async () => {
    vi.mocked(mcpService.listForRole)
      .mockResolvedValueOnce([boundMcpServer])
      .mockResolvedValueOnce([]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByText('日历 MCP')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '移除 日历 MCP' }));

    await waitFor(() => {
      expect(mcpService.removeFromRole).toHaveBeenCalledWith('role-1', 'mcp-calendar');
    });
    expect(mcpService.delete).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(screen.queryByText('日历 MCP')).not.toBeInTheDocument();
    });
  });

  it('切换角色时忽略上一角色 MCP 操作完成后的刷新结果', async () => {
    let resolveAdd: (() => void) | undefined;
    vi.mocked(mcpService.addToRole).mockReturnValue(new Promise<void>(resolve => { resolveAdd = resolve; }));
    vi.mocked(mcpService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([boundMcpServer])
      .mockResolvedValueOnce([availableMcpServer]);
    vi.mocked(mcpService.listAvailableForRole)
      .mockResolvedValueOnce([availableMcpServer])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    const { rerender } = render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '添加 MCP server' }));
    fireEvent.click(await screen.findByRole('button', { name: '添加 邮件 MCP' }));
    rerender(<SettingsTab role={secondRole} activeRoleCount={2} />);
    expect(await screen.findByText('日历 MCP')).toBeInTheDocument();

    await act(async () => {
      resolveAdd?.();
    });

    await waitFor(() => {
      expect(mcpService.listForRole).toHaveBeenCalledWith('role-1');
      expect(mcpService.listForRole).toHaveBeenCalledWith('role-2');
    });
    expect(screen.getByText('日历 MCP')).toBeInTheDocument();
    expect(screen.queryByText('邮件 MCP')).not.toBeInTheDocument();
  });

  it('未绑定到当前角色的自定义 Skill 不在角色设置中显示', async () => {
    vi.mocked(skillService.listRegistry).mockResolvedValue([customSkill]);
    vi.mocked(skillService.listForRole).mockResolvedValue([]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    await waitFor(() => {
      expect(skillService.listForRole).toHaveBeenCalledWith('role-1');
    });
    expect(screen.queryByRole('switch', { name: 'daily-review' })).not.toBeInTheDocument();
    expect(screen.getByText('暂无自定义 Skill。')).toBeInTheDocument();
  });

  it('Skill 文件夹读取失败时显示友好错误', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockRejectedValue({ ValidationError: 'SKILL.md frontmatter 格式不完整' });

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));

    expect(await screen.findByText('SKILL.md 解析失败，请检查 frontmatter 中的 name 和 description')).toBeInTheDocument();
  });

  it('取消选择 Skill 文件夹时不显示错误', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockRejectedValue('未选择 Skill 文件夹');

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));

    await waitFor(() => {
      expect(skillService.pickCustomDirectory).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByText('SKILL.md 解析失败，请检查 frontmatter 中的 name 和 description')).not.toBeInTheDocument();
    expect(screen.queryByText('未能从所选 Skill 文件夹读取 SKILL.md')).not.toBeInTheDocument();
  });

  it('启用和禁用自定义 Skill 会保存 registry skill ids', async () => {
    vi.mocked(skillService.listForRole).mockResolvedValue([customSkill]);
    vi.mocked(roleService.updateSkills).mockResolvedValue({
      ...baseRole,
      skillsConfig: '{"enabledSkillIds":["skill-1"]}',
    });

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByRole('switch', { name: 'daily-review' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: 'daily-review' }));

    await waitFor(() => {
      expect(roleService.updateSkills).toHaveBeenCalledWith('role-1', {
        findSkills: false,
        skillCreator: false,
        enabledSkillIds: ['skill-1'],
      });
    });
  });

  it('自定义 Skill 删除必须先弹窗确认，确认后才执行全局删除并刷新列表', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.delete).mockResolvedValue(undefined);
    vi.mocked(roleService.list).mockResolvedValue([baseRole]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByRole('button', { name: '删除 daily-review' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '删除 daily-review' }));

    expect(skillService.delete).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: '删除自定义 Skill' });
    expect(dialog).toHaveTextContent('确认删除「daily-review」吗？');
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.delete).toHaveBeenCalledWith('skill-1');
    });
    expect(skillService.removeFromRole).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(skillService.listForRole).toHaveBeenCalledTimes(2);
    });
  });

  it('自定义 Skill 描述默认截断两行并通过受限宽度悬浮全文展示', async () => {
    const longSkill = {
      ...customSkill,
      description: '这是一个很长的技能描述，用来验证默认最多展示两行，避免列表卡片占用过多垂直空间，同时鼠标悬浮时仍然可以看到完整描述。',
    };
    vi.mocked(skillService.listForRole).mockResolvedValue([longSkill]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    const card = await screen.findByTestId('custom-skill-card-skill-1');
    const description = screen.getByTestId('custom-skill-description-skill-1');
    const actions = screen.getByTestId('custom-skill-actions-skill-1');

    expect(card).toContainElement(description);
    expect(card).toContainElement(actions);
    expect(description.compareDocumentPosition(actions) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
    expect(description).toHaveClass('line-clamp-2');
    expect(description).not.toHaveAttribute('title');
    expect(screen.getByTestId('custom-skill-tooltip-skill-1')).toHaveClass('max-w-[min(28rem,calc(100vw-3rem))]');
  });

  it('删除自定义 Skill 会执行全局删除并刷新所有角色状态', async () => {
    const refreshedRole = { ...baseRole, skillsConfig: '{"enabledSkillIds":[]}' };
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.delete).mockResolvedValue(undefined);
    vi.mocked(roleService.list).mockResolvedValue([refreshedRole, secondRole]);
    const onUpdateRole = vi.fn();

    render(<SettingsTab role={baseRole} activeRoleCount={2} onUpdateRole={onUpdateRole} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));

    const dialog = await screen.findByRole('dialog', { name: '删除自定义 Skill' });
    expect(dialog).toHaveTextContent('该 Skill 将从所有角色和管家中删除');
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.delete).toHaveBeenCalledWith('skill-1');
    });
    expect(skillService.removeFromRole).not.toHaveBeenCalled();
    expect(roleService.list).toHaveBeenCalled();
    expect(onUpdateRole).toHaveBeenCalledWith(refreshedRole);
    expect(onUpdateRole).toHaveBeenCalledWith(secondRole);
    expect(await screen.findByText('自定义 Skill 已删除')).toBeInTheDocument();
    expect(skillService.notifyScopeUpdated).toHaveBeenCalledWith({ scopeKind: 'all', ownerId: null });
  });

  it('自定义 Skill 删除失败时保留列表且不显示成功提示', async () => {
    vi.mocked(skillService.listForRole).mockResolvedValue([customSkill]);
    vi.mocked(skillService.delete).mockRejectedValue(new Error('delete failed'));

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    expect(await screen.findByText('删除自定义 Skill 失败，请稍后重试')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '删除 daily-review' })).toBeInTheDocument();
    expect(screen.queryByText('自定义 Skill 已删除')).not.toBeInTheDocument();
  });

  it('自定义 Skill 已删除但刷新失败时显示准确状态并移除本地条目', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockRejectedValueOnce(new Error('refresh failed'));
    vi.mocked(skillService.delete).mockResolvedValue(undefined);
    vi.mocked(roleService.list).mockResolvedValue([baseRole]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    expect(await screen.findByText('自定义 Skill 已删除，但列表刷新失败，请重新打开设置页')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '删除 daily-review' })).not.toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: '删除自定义 Skill' })).not.toBeInTheDocument();
  });

  it('自定义 Skill 删除成功提示会自动消失', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.delete).mockResolvedValue(undefined);
    vi.mocked(roleService.list).mockResolvedValue([baseRole]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.delete).toHaveBeenCalledWith('skill-1');
    });
    expect(screen.getByText('自定义 Skill 已删除')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText('自定义 Skill 已删除')).not.toBeInTheDocument();
    }, { timeout: 2500 });
  });

  it('角色导入 Skill 时复用范围可以包含管家', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
      sourcePath: 'C:/UAT/skills/daily-review',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      runtimeReady: true,
      runtimeError: null,
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(roleService.list).mockResolvedValue([
      { ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
    ]);

    render(
      <SettingsTab
        role={baseRole}
        activeRoles={[baseRole, secondRole]}
        activeRoleCount={2}
      />
    );

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    expect(await screen.findByRole('checkbox', { name: '管家' })).not.toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: '管家' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        sourcePath: 'C:/UAT/skills/daily-review',
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['role-1', '__butler__'] },
      });
    });
    expect(skillService.notifyScopeUpdated).toHaveBeenCalledWith({ scopeKind: 'all', ownerId: null });
  });

  it('自定义 Skill 描述不使用原生 title，并提供受限宽度的悬浮全文', async () => {
    const longSkill = {
      ...customSkill,
      description: '这是一个很长的技能描述，用来验证默认最多展示两行，避免列表卡片占用过多垂直空间，同时鼠标悬浮时仍然可以看到完整描述。',
    };
    vi.mocked(skillService.listForRole).mockResolvedValue([longSkill]);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    const description = await screen.findByTestId('custom-skill-description-skill-1');
    expect(description).toHaveClass('line-clamp-2');
    expect(description).not.toHaveAttribute('title');
    expect(screen.getByTestId('custom-skill-tooltip-skill-1')).toHaveClass('max-w-[min(28rem,calc(100vw-3rem))]');
  });

  it('导入已存在 Skill 时仍刷新复用角色配置', async () => {
    const onUpdateRole = vi.fn();
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
      sourcePath: 'C:/UAT/skills/daily-review',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: { kind: 'contentHash', existing: customSkill },
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'duplicate',
      runtimeReady: true,
      runtimeError: null,
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: { kind: 'contentHash', existing: customSkill },
      },
    });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(roleService.list).mockResolvedValue([
      { ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
      { ...secondRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
    ]);

    render(
      <SettingsTab
        role={baseRole}
        activeRoles={[baseRole, secondRole]}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    fireEvent.click(await screen.findByRole('checkbox', { name: '学习者' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(roleService.list).toHaveBeenCalledTimes(1);
    });
    expect(onUpdateRole).toHaveBeenCalledWith({ ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' });
    expect(onUpdateRole).toHaveBeenCalledWith({ ...secondRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' });
    expect(await screen.findByText('Skill 已存在，已更新复用范围并可使用')).toBeInTheDocument();
  });

  it('导入 Skill 复用到其他角色后刷新所有复用角色配置，使复用角色默认启用', async () => {
    const onUpdateRole = vi.fn();
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
      sourcePath: 'C:/UAT/skills/daily-review',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      runtimeReady: true,
      runtimeError: null,
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(roleService.list).mockResolvedValue([
      { ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
      { ...secondRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' },
    ]);

    render(
      <SettingsTab
        role={baseRole}
        activeRoles={[baseRole, secondRole]}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    fireEvent.click(await screen.findByRole('checkbox', { name: '学习者' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        sourcePath: 'C:/UAT/skills/daily-review',
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['role-1', 'role-2'] },
      });
    });
    await waitFor(() => {
      expect(roleService.list).toHaveBeenCalledTimes(1);
    });
    expect(roleService.updateSkills).not.toHaveBeenCalled();
    expect(onUpdateRole).toHaveBeenCalledWith({ ...baseRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' });
    expect(onUpdateRole).toHaveBeenCalledWith({ ...secondRole, skillsConfig: '{"enabledSkillIds":["skill-1"]}' });
  });

  it('未启用 find-skills 时发现 opencode Skill 只显示启用入口且不扫描', async () => {
    const mockedSkillService = skillService as typeof skillService & {
      discoverOpencode: ReturnType<typeof vi.fn>;
    };

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('需要先启用 find-skills 才能发现可用 Skill')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '启用 find-skills' })).toBeInTheDocument();
    expect(mockedSkillService.discoverOpencode).not.toHaveBeenCalled();
  });

  it('发现 opencode Skill 成功但无结果时展示已扫描提示并恢复按钮', async () => {
    const enabledRole: Role = {
      ...baseRole,
      skillsConfig: '{"findSkills":true}',
    };

    render(<SettingsTab role={enabledRole} activeRoleCount={2} />);

    expect(screen.queryByText('已扫描项目级与全局 Skill 目录，未发现 opencode Skill。')).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('已扫描项目级与全局 Skill 目录，未发现 opencode Skill。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '发现 opencode Skill' })).toHaveTextContent('发现');
  });

  it('切换角色时忽略旧角色尚未完成的 opencode 扫描响应', async () => {
    let resolveDiscovery!: (result: { items: []; skipped: { total: number; reasons: string[] } }) => void;
    vi.mocked(skillService.discoverOpencode).mockReturnValueOnce(new Promise(resolve => {
      resolveDiscovery = resolve;
    }));

    const enabledRole: Role = {
      ...baseRole,
      skillsConfig: '{"findSkills":true}',
    };
    const nextRole: Role = {
      ...secondRole,
      skillsConfig: '{"findSkills":true}',
    };
    const { rerender } = render(<SettingsTab role={enabledRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));
    await waitFor(() => expect(skillService.discoverOpencode).toHaveBeenCalledWith('role-1'));

    rerender(<SettingsTab role={nextRole} activeRoleCount={2} />);
    await waitFor(() => expect(screen.getByRole('button', { name: '发现 opencode Skill' })).not.toBeDisabled());

    await act(async () => {
      resolveDiscovery({ items: [], skipped: { total: 0, reasons: [] } });
    });

    expect(screen.queryByText('已扫描项目级与全局 Skill 目录，未发现 opencode Skill。')).not.toBeInTheDocument();
  });

  it('发现 opencode Skill 失败时展示安全的后端校验提示', async () => {
    vi.mocked(skillService.discoverOpencode).mockRejectedValueOnce({
      ValidationError: 'Skill 名称 writer 已被另一来源占用（custom）',
    });

    render(<SettingsTab role={{ ...baseRole, skillsConfig: '{"findSkills":true}' }} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('存在同名但来源不同的 Skill，请检查已导入的 Skill')).toBeInTheDocument();
    expect(screen.queryByText('已扫描项目级与全局 Skill 目录，未发现 opencode Skill。')).not.toBeInTheDocument();
  });

  it('启用 find-skills 后展示 opencode 扫描结果和跳过摘要，并可导入到当前角色', async () => {
    const onUpdateRole = vi.fn();
    const opencodeSkill = {
      name: 'writer',
      description: '写作助手，提供长文本润色、结构调整、标题建议和语气优化能力，描述较长时应只显示两行。',
      sourceLocation: '全局',
      sourcePath: 'C:/Users/Admin/.config/opencode/skills/writer/SKILL.md',
      sourceType: 'opencode' as const,
      contentHash: 'hash-writer',
      alreadyImported: false,
      duplicate: null,
    };
    const opencodeEntry: SkillRegistryEntry = {
      ...customSkill,
      id: 'skill-opencode',
      name: 'writer',
      description: opencodeSkill.description,
      sourceType: 'opencode',
      managedPath: opencodeSkill.sourcePath,
      contentHash: 'hash-writer',
    };
    const enabledRole: Role = {
      ...baseRole,
      skillsConfig: '{"meta":{"findSkills":true,"skillCreator":false},"enabledSkillIds":[]}',
    };
    const refreshedRole: Role = {
      ...enabledRole,
      skillsConfig: '{"meta":{"findSkills":true,"skillCreator":false},"enabledSkillIds":["skill-opencode"]}',
    };
    const removedRole: Role = {
      ...enabledRole,
      skillsConfig: '{"meta":{"findSkills":true,"skillCreator":false},"enabledSkillIds":[]}',
    };
    vi.mocked(skillService.discoverOpencode).mockResolvedValue({
      items: [opencodeSkill],
      skipped: { total: 1, reasons: ['缺少 SKILL.md 的条目已跳过'] },
    });
    vi.mocked(skillService.importOpencode).mockResolvedValue({ status: 'imported', entry: opencodeEntry, synced: true, runtimeReady: true, runtimeError: null });
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([opencodeEntry])
      .mockResolvedValueOnce([]);
    vi.mocked(roleService.list)
      .mockResolvedValueOnce([refreshedRole])
      .mockResolvedValueOnce([removedRole]);

    const { rerender } = render(<SettingsTab role={enabledRole} activeRoleCount={2} onUpdateRole={onUpdateRole} />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('writer')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '发现 opencode Skill' })).toHaveTextContent('发现');
    expect(screen.getByRole('button', { name: '发现 opencode Skill' })).toHaveClass('whitespace-nowrap');
    expect(screen.getByText('全局')).toBeInTheDocument();
    expect(screen.getByText('已跳过 1 个无效 Skill：')).toBeInTheDocument();
    expect(screen.getByText('缺少 SKILL.md 的条目已跳过')).toBeInTheDocument();
    expect(screen.queryByText('已扫描项目级与全局 Skill 目录，未发现 opencode Skill。')).not.toBeInTheDocument();
    expect(screen.getByTestId('opencode-skill-description-hash-writer')).toHaveClass('line-clamp-2');
    fireEvent.click(screen.getByRole('button', { name: /收起/ }));
    expect(screen.queryByText('writer')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /发现 1 个 opencode Skill.*展开/ }));
    expect(screen.getByText('writer')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '导入' }));

    await waitFor(() => {
      expect(skillService.importOpencode).toHaveBeenCalledWith('role-1', {
        sourcePath: opencodeSkill.sourcePath,
        roleScope: { allRoles: false, roleIds: ['role-1'] },
        expectedContentHash: 'hash-writer',
      });
    });
    expect(onUpdateRole).toHaveBeenCalledWith(refreshedRole);
    expect(await screen.findByText('opencode Skill 已导入并可立即使用')).toBeInTheDocument();
    rerender(<SettingsTab role={refreshedRole} activeRoleCount={2} onUpdateRole={onUpdateRole} />);
    expect(screen.getAllByText('writer').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /收起/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '取消导入' }));

    await waitFor(() => {
      expect(skillService.removeFromRole).toHaveBeenCalledWith('skill-opencode', 'role-1');
    });
    expect(skillService.delete).not.toHaveBeenCalled();
    expect(onUpdateRole).toHaveBeenCalledWith(removedRole);
    expect(await screen.findByText('opencode Skill 已取消导入')).toBeInTheDocument();
    rerender(<SettingsTab role={removedRole} activeRoleCount={2} onUpdateRole={onUpdateRole} />);
    expect(screen.getAllByText('writer').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /收起/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '导入' })).toBeInTheDocument();
  });

  it('opencode Skill 已在 registry 但未启用到当前角色时仍显示导入', async () => {
    const opencodeEntry: SkillRegistryEntry = {
      ...customSkill,
      id: 'skill-opencode',
      name: 'writer',
      description: '写作助手',
      sourceType: 'opencode',
      managedPath: 'managed/writer/SKILL.md',
      contentHash: 'hash-writer',
    };
    const enabledRole: Role = {
      ...baseRole,
      skillsConfig: '{"meta":{"findSkills":true,"skillCreator":false},"enabledSkillIds":[]}',
    };
    vi.mocked(skillService.discoverOpencode).mockResolvedValue({
      items: [{
        name: 'writer',
        description: '写作助手',
        sourceLocation: '全局',
        sourcePath: 'C:/Users/Admin/.config/opencode/skills/writer/SKILL.md',
        sourceType: 'opencode',
        contentHash: 'hash-writer',
        alreadyImported: false,
        duplicate: { kind: 'contentHash', existing: opencodeEntry },
      }],
      skipped: { total: 0, reasons: [] },
    });

    render(<SettingsTab role={enabledRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect((await screen.findAllByText('writer')).length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: '导入' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '取消导入' })).not.toBeInTheDocument();
  });

  it('切换主动性级别后持久化并回传更新后的角色', async () => {
    const onUpdateRole = vi.fn();
    const updatedProactivityRole: Role = {
      ...baseRole,
      proactivityLevel: 'proactive',
    };
    vi.mocked(roleService.updateProactivity).mockResolvedValue(updatedProactivityRole);

    render(
      <SettingsTab
        role={baseRole}
        activeRoleCount={2}
        onUpdateRole={onUpdateRole}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: '积极主动' }));

    await waitFor(() => {
      expect(roleService.updateProactivity).toHaveBeenCalledWith('role-1', {
        proactivityLevel: 'proactive',
      });
    });
    expect(onUpdateRole).toHaveBeenCalledWith(updatedProactivityRole);
    expect(await screen.findByText('主动性级别已保存')).toBeInTheDocument();
  });


  it('支持单独折叠角色设置分组并持久化状态', () => {
    const { unmount } = render(<SettingsTab role={baseRole} activeRoleCount={2} />);
    const profileHeader = screen.getByRole('button', { name: '角色信息' });

    expect(profileHeader).toHaveAttribute('aria-expanded', 'true');
    fireEvent.change(screen.getByLabelText('名称'), { target: { value: '未保存的新名称' } });
    fireEvent.click(profileHeader);
    expect(profileHeader).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(profileHeader);
    expect(screen.getByLabelText('名称')).toHaveValue('未保存的新名称');
    fireEvent.click(profileHeader);
    expect(JSON.parse(localStorage.getItem('egosync-role-settings-sections') ?? '{}')).toMatchObject({ profile: false });

    unmount();
    render(<SettingsTab role={baseRole} activeRoleCount={2} />);
    expect(screen.getByRole('button', { name: '角色信息' })).toHaveAttribute('aria-expanded', 'false');
  });

  it('损坏的角色折叠状态会安全回退为全部展开', () => {
    localStorage.setItem('egosync-role-settings-sections', '{broken-json');
    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    ['角色信息', '主动性级别', 'Skill 配置', 'MCP Server配置'].forEach(name => {
      expect(screen.getByRole('button', { name })).toHaveAttribute('aria-expanded', 'true');
    });
  });

  it('支持全部折叠和全部展开角色设置分组', () => {
    render(<SettingsTab role={baseRole} activeRoleCount={2} />);
    const sectionNames = ['角色信息', '主动性级别', 'Skill 配置', 'MCP Server配置'];

    fireEvent.click(screen.getByRole('button', { name: '全部折叠' }));
    sectionNames.forEach(name => expect(screen.getByRole('button', { name })).toHaveAttribute('aria-expanded', 'false'));

    fireEvent.click(screen.getByRole('button', { name: '全部展开' }));
    sectionNames.forEach(name => expect(screen.getByRole('button', { name })).toHaveAttribute('aria-expanded', 'true'));
  });

  it('角色信息独立保存并显示新的 MCP 配置文案', async () => {
    render(<SettingsTab role={baseRole} activeRoleCount={2} />);
    expect(screen.getByRole('button', { name: '保存角色信息' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '保存更改' })).not.toBeInTheDocument();
    expect(screen.getByText('MCP Server配置')).toBeInTheDocument();
    expect(screen.queryByText('外部 MCP 工具')).not.toBeInTheDocument();
    expect(screen.getByText('Skill 配置').compareDocumentPosition(screen.getByText('自定义 Skill')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

});
