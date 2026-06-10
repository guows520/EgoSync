import { act, render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { ButlerSettingsContent } from './ButlerSettingsContent';
import { appService } from '../../services/appService';
import { roleService } from '../../services/roleService';
import { skillService } from '../../services/skillService';
import type { Role } from '../../types/role';
import type { SkillRegistryEntry } from '../../types/skill';

vi.mock('../../services/appService', () => ({
  appService: {
    getButlerSkills: vi.fn(),
    updateButlerSkills: vi.fn(),
  },
}));

vi.mock('../../services/roleService', () => ({
  roleService: {
    list: vi.fn(),
  },
}));

vi.mock('../../services/skillService', () => ({
  skillService: {
    listAllRoleSkills: vi.fn(),
    pickCustomDirectory: vi.fn(),
    previewCustom: vi.fn(),
    importCustom: vi.fn(),
    discoverOpencode: vi.fn(),
    importOpencode: vi.fn(),
    delete: vi.fn(),
    removeFromRole: vi.fn(),
  },
}));

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

const baseRole: Role = {
  id: 'role-1',
  name: '父亲',
  icon: 'baby',
  color: '#EF4444',
  goal: '陪伴孩子成长',
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
  goal: '保持学习节奏',
};

describe('ButlerSettingsContent', () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: [],
    });
    vi.mocked(skillService.listAllRoleSkills).mockResolvedValue([]);
    vi.mocked(skillService.discoverOpencode).mockResolvedValue({ items: [], skipped: { total: 0, reasons: [] } });
    vi.mocked(skillService.importOpencode).mockResolvedValue({ status: 'imported', entry: null, synced: true });
    vi.mocked(roleService.list).mockResolvedValue([baseRole, secondRole]);
  });

  it('展示并持久化管家 Skill 配置', async () => {
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: true,
      enabledSkillIds: [],
    });

    render(<ButlerSettingsContent />);

    expect(await screen.findByText('Skill 配置')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'find-skills' })).toBeChecked();
    expect(screen.getByRole('switch', { name: 'skill-creator' })).not.toBeChecked();
    expect(screen.queryByText('保存中...')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('switch', { name: 'skill-creator' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: true,
        enabledSkillIds: [],
      });
    });
    expect(await screen.findByText('Skill 配置已保存')).toBeInTheDocument();
    expect(screen.queryByText('保存中...')).not.toBeInTheDocument();
  });

  it('管家可以管理全部角色范围的自定义 Skill', async () => {
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });
    vi.mocked(skillService.listAllRoleSkills).mockResolvedValue([customSkill]);
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: [],
    });

    render(<ButlerSettingsContent />);

    expect(await screen.findByText('自定义 Skill')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'daily-review' })).toBeChecked();
    expect(screen.getByTestId('custom-skill-description-skill-1')).toHaveTextContent('日复盘助手');

    fireEvent.click(screen.getByRole('switch', { name: 'daily-review' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: false,
        enabledSkillIds: [],
      });
    });
  });

  it('管家选择 Skill 文件夹时可选择复用角色范围', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listAllRoleSkills)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });

    render(<ButlerSettingsContent activeRoles={[baseRole, secondRole]} />);

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    expect(await screen.findByText('预览：daily-review')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: '全部角色' })).toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: '全部角色' }));
    expect(screen.getByRole('checkbox', { name: '父亲' })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: '学习者' })).not.toBeChecked();

    fireEvent.click(screen.getByRole('checkbox', { name: '父亲' }));
    fireEvent.click(screen.getByRole('checkbox', { name: '学习者' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['__butler__', 'role-1', 'role-2'] },
      });
    });
    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: false,
        enabledSkillIds: ['skill-1'],
      });
    });
  });

  it('管家自定义 Skill 删除必须先弹窗确认，确认后只从管家移除', async () => {
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });
    vi.mocked(skillService.listAllRoleSkills).mockResolvedValue([customSkill]);
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: [],
    });

    render(<ButlerSettingsContent />);

    expect(await screen.findByRole('button', { name: '删除 daily-review' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '删除 daily-review' }));

    expect(skillService.delete).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: '删除自定义 Skill' });
    expect(dialog).toHaveTextContent('确认删除「daily-review」吗？');
    expect(dialog).not.toHaveTextContent('所有角色和管家');
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: false,
        enabledSkillIds: [],
      });
    });
    expect(skillService.delete).not.toHaveBeenCalled();
  });

  it('管家删除成功提示会自动消失', async () => {
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });
    vi.mocked(skillService.listAllRoleSkills).mockResolvedValue([customSkill]);
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: [],
    });

    render(<ButlerSettingsContent />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: false,
        enabledSkillIds: [],
      });
    });
    expect(screen.getByText('自定义 Skill 已删除')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText('自定义 Skill 已删除')).not.toBeInTheDocument();
    }, { timeout: 2500 });
  });

  it('管家自定义 Skill 描述默认截断两行并通过受限宽度悬浮全文展示', async () => {
    const longSkill = {
      ...customSkill,
      description: '这是一个很长的技能描述，用来验证管家设置页也默认最多展示两行，避免列表卡片占用过多垂直空间，同时鼠标悬浮时仍然可以看到完整描述。',
    };
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });
    vi.mocked(skillService.listAllRoleSkills).mockResolvedValue([longSkill]);

    render(<ButlerSettingsContent />);

    const description = await screen.findByTestId('custom-skill-description-skill-1');
    expect(description).toHaveClass('line-clamp-2');
    expect(description).not.toHaveAttribute('title');
    expect(screen.getByTestId('custom-skill-tooltip-skill-1')).toHaveClass('max-w-[min(28rem,calc(100vw-3rem))]');
  });

  it('管家启用 find-skills 后展示 opencode 扫描结果并可导入和取消导入', async () => {
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
      managedPath: 'managed/writer/SKILL.md',
      contentHash: 'hash-writer',
    };
    vi.mocked(skillService.discoverOpencode).mockResolvedValue({
      items: [opencodeSkill],
      skipped: { total: 1, reasons: ['缺少 SKILL.md 的条目已跳过'] },
    });
    vi.mocked(skillService.importOpencode).mockResolvedValue({ status: 'imported', entry: opencodeEntry, synced: true });
    vi.mocked(skillService.listAllRoleSkills)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([opencodeEntry])
      .mockResolvedValueOnce([]);

    render(<ButlerSettingsContent />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('writer')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '发现 opencode Skill' })).toHaveTextContent('发现');
    expect(screen.getByRole('button', { name: '发现 opencode Skill' })).toHaveClass('whitespace-nowrap');
    expect(screen.getByText('全局')).toBeInTheDocument();
    expect(screen.getByText('已跳过 1 个无效 Skill：')).toBeInTheDocument();
    expect(screen.getByText('缺少 SKILL.md 的条目已跳过')).toBeInTheDocument();
    expect(screen.getByTestId('opencode-skill-description-hash-writer')).toHaveClass('line-clamp-2');
    fireEvent.click(screen.getByRole('button', { name: /收起/ }));
    expect(screen.queryByText('writer')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /展开/ }));
    expect(screen.getByText('writer')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '导入' }));

    await waitFor(() => {
      expect(skillService.importOpencode).toHaveBeenCalledWith('__butler__', {
        sourcePath: opencodeSkill.sourcePath,
        roleScope: { allRoles: false, roleIds: ['__butler__'] },
        expectedContentHash: 'hash-writer',
      });
    });
    expect(await screen.findByText('opencode Skill 已导入并启用')).toBeInTheDocument();
    expect(screen.getAllByText('writer').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /收起/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '取消导入' }));

    await waitFor(() => {
      expect(skillService.removeFromRole).toHaveBeenCalledWith('skill-opencode', '__butler__');
    });
    expect(await screen.findByText('opencode Skill 已取消导入')).toBeInTheDocument();
    expect(screen.getAllByText('writer').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /收起/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '导入' })).toBeInTheDocument();
  });

  it('管家未启用 find-skills 时发现 opencode Skill 只显示启用入口且不扫描', async () => {
    vi.mocked(appService.getButlerSkills).mockResolvedValue({
      findSkills: false,
      skillCreator: false,
      enabledSkillIds: [],
    });
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: [],
    });

    render(<ButlerSettingsContent />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('需要先启用 find-skills 才能发现可用 Skill')).toBeInTheDocument();
    expect(skillService.discoverOpencode).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: '启用 find-skills' }));

    await waitFor(() => {
      expect(appService.updateButlerSkills).toHaveBeenCalledWith({
        findSkills: true,
        skillCreator: false,
        enabledSkillIds: [],
      });
    });
  });

  it('opencode Skill 已在 registry 但管家未启用时仍显示导入', async () => {
    const opencodeEntry: SkillRegistryEntry = {
      ...customSkill,
      id: 'skill-opencode',
      name: 'writer',
      description: '写作助手',
      sourceType: 'opencode',
      managedPath: 'managed/writer/SKILL.md',
      contentHash: 'hash-writer',
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

    render(<ButlerSettingsContent />);

    fireEvent.click(await screen.findByRole('button', { name: '发现 opencode Skill' }));

    expect(await screen.findByText('writer')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '导入' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '取消导入' })).not.toBeInTheDocument();
  });

  it('管家 Skill 配置加载完成前不渲染默认打开的开关', async () => {
    let resolveSkills: (value: { findSkills: boolean; skillCreator: boolean; enabledSkillIds: string[] }) => void = () => {};
    vi.mocked(appService.getButlerSkills).mockImplementation(() => new Promise(resolve => {
      resolveSkills = resolve;
    }));

    render(<ButlerSettingsContent />);

    expect(screen.queryByRole('switch', { name: 'find-skills' })).not.toBeInTheDocument();
    expect(screen.getByText('正在加载 Skill 配置...')).toBeInTheDocument();

    await act(async () => {
      resolveSkills({ findSkills: false, skillCreator: false, enabledSkillIds: [] });
    });

    expect(await screen.findByRole('switch', { name: 'find-skills' })).not.toBeChecked();
  });

  it('管家导入 Skill 的复用范围包含管家选项', async () => {
    vi.mocked(skillService.pickCustomDirectory).mockResolvedValue({
      content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: null,
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'imported',
      entry: customSkill,
      preview: {
        name: 'daily-review',
        description: '日复盘助手',
        contentHash: 'hash-1',
        duplicate: null,
      },
    });
    vi.mocked(skillService.listAllRoleSkills)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([customSkill]);
    vi.mocked(appService.updateButlerSkills).mockResolvedValue({
      findSkills: true,
      skillCreator: false,
      enabledSkillIds: ['skill-1'],
    });

    render(<ButlerSettingsContent activeRoles={[baseRole, secondRole]} />);

    fireEvent.click(await screen.findByRole('button', { name: '选择skill文件夹' }));
    fireEvent.click(await screen.findByRole('checkbox', { name: '全部角色' }));
    expect(screen.getByRole('checkbox', { name: '管家' })).toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: '父亲' }));
    fireEvent.click(screen.getByRole('button', { name: '确认导入' }));

    await waitFor(() => {
      expect(skillService.importCustom).toHaveBeenCalledWith({
        content: '---\nname: daily-review\ndescription: 日复盘助手\n---\n',
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['__butler__', 'role-1'] },
      });
    });
  });
});
