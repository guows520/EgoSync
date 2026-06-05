import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { SettingsTab } from './SettingsTab';
import { roleService } from '../../services/roleService';
import { skillService } from '../../services/skillService';
import type { Role } from '../../types/role';
import type { SkillRegistryEntry } from '../../types/skill';

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
    listRegistry: vi.fn(),
    listForRole: vi.fn(),
    pickCustomDirectory: vi.fn(),
    previewCustom: vi.fn(),
    importCustom: vi.fn(),
    delete: vi.fn(),
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

describe('SettingsTab role CRUD actions', () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    vi.mocked(skillService.listRegistry).mockResolvedValue([]);
    vi.mocked(skillService.listForRole).mockResolvedValue([]);
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
    fireEvent.click(screen.getByRole('button', { name: '保存更改' }));

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
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['role-1'] },
      });
    });
    await waitFor(() => {
      expect(roleService.list).toHaveBeenCalledTimes(1);
    });
    expect(roleService.updateSkills).not.toHaveBeenCalled();
    expect(await screen.findByText('自定义 Skill 已导入并启用')).toBeInTheDocument();

    Object.defineProperty(window, 'showDirectoryPicker', {
      configurable: true,
      value: originalPicker,
    });
  });

  it('导入 Skill 时勾选全部角色会持久化全部角色范围', async () => {
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
        overwriteExisting: false,
        roleScope: { allRoles: true, roleIds: [] },
      });
    });
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

  it('自定义 Skill 删除必须先弹窗确认，确认后才从当前角色移除并刷新当前角色列表', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.removeFromRole).mockResolvedValue(undefined);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    expect(await screen.findByRole('button', { name: '删除 daily-review' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '删除 daily-review' }));

    expect(skillService.delete).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: '删除自定义 Skill' });
    expect(dialog).toHaveTextContent('确认删除「daily-review」吗？');
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.removeFromRole).toHaveBeenCalledWith('skill-1', 'role-1');
    });
    expect(skillService.delete).not.toHaveBeenCalled();
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

  it('从角色删除自定义 Skill 只移除当前角色，不调用全局删除', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.removeFromRole).mockResolvedValue(undefined);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));

    expect(skillService.delete).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: '删除自定义 Skill' });
    expect(dialog).toHaveTextContent('确认删除「daily-review」吗？');
    expect(dialog).not.toHaveTextContent('所有角色和管家');
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.removeFromRole).toHaveBeenCalledWith('skill-1', 'role-1');
    });
    expect(skillService.delete).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(screen.getByText('自定义 Skill 已删除')).toBeInTheDocument();
    });
  });

  it('自定义 Skill 删除成功提示会自动消失', async () => {
    vi.mocked(skillService.listForRole)
      .mockResolvedValueOnce([customSkill])
      .mockResolvedValueOnce([]);
    vi.mocked(skillService.removeFromRole).mockResolvedValue(undefined);

    render(<SettingsTab role={baseRole} activeRoleCount={2} />);

    fireEvent.click(await screen.findByRole('button', { name: '删除 daily-review' }));
    fireEvent.click(screen.getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(skillService.removeFromRole).toHaveBeenCalledWith('skill-1', 'role-1');
    });
    expect(screen.getByText('自定义 Skill 已删除')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText('自定义 Skill 已删除')).not.toBeInTheDocument();
    }, { timeout: 2500 });
  });

  it('角色导入 Skill 时复用范围可以包含管家', async () => {
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
        overwriteExisting: false,
        roleScope: { allRoles: false, roleIds: ['role-1', '__butler__'] },
      });
    });
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
    });
    vi.mocked(skillService.previewCustom).mockResolvedValue({
      name: 'daily-review',
      description: '日复盘助手',
      contentHash: 'hash-1',
      duplicate: { kind: 'contentHash', existing: customSkill },
    });
    vi.mocked(skillService.importCustom).mockResolvedValue({
      status: 'duplicate',
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
    expect(await screen.findByText('Skill 已存在，已更新复用范围')).toBeInTheDocument();
  });

  it('导入 Skill 复用到其他角色后刷新所有复用角色配置，使复用角色默认启用', async () => {
    const onUpdateRole = vi.fn();
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

  it('永久删除必须输入角色名确认', () => {
    const onDeleteRole = vi.fn();

    render(
      <SettingsTab
        role={baseRole}
        activeRoleCount={2}
        onDeleteRole={onDeleteRole}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /永久删除/ }));
    const confirmButton = screen.getByRole('button', { name: '永久删除' });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText('产品经理'), { target: { value: '错误名称' } });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText('产品经理'), { target: { value: '产品经理' } });
    expect(confirmButton).not.toBeDisabled();
  });

  it('仅剩一个 active 角色时禁用危险操作', () => {
    render(<SettingsTab role={baseRole} activeRoleCount={1} />);

    expect(screen.getByText('至少保留一个角色')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /归档角色/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /永久删除/ })).toBeDisabled();
  });
});
