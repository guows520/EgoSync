import { useEffect, useMemo, useRef, useState } from 'react';
import { Check, Trash2, AlertCircle, Upload, ChevronDown, ChevronUp } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ROLE_COLORS, ROLE_ICONS, getRoleIconComponent, normalizeColorHex, normalizeIconId } from '../../lib/roleIcons';
import { roleService } from '../../services/roleService';
import { skillService } from '../../services/skillService';
import { mcpService } from '../../services/mcpService';
import type { ProactivityLevel, Role, RoleSkillsConfig } from '../../types/role';
import type { McpServer } from '../../types/mcp';
import type { OpencodeSkillCandidate, SkillImportPreview, SkillRegistryEntry } from '../../types/skill';
import { ProactivityToggle } from './ProactivityToggle';
import { useTauriEvent } from '../../hooks/useTauriEvent';

interface SettingsTabProps {
  role: Role;
  activeRoles?: Role[];
  activeRoleCount?: number;
  onUpdateRole?: (role: Role) => void;
  onArchiveRole?: (id: string) => Promise<void> | void;
  onDeleteRole?: (id: string) => Promise<void> | void;
}

type SkillKey = 'findSkills' | 'skillCreator';

const MIN_ACTIVE_ROLE_MESSAGE = '至少保留一个角色';
const BUTLER_SCOPE_ID = '__butler__';
const MESSAGE_TIMEOUT_MS = 1500;
const ROLE_SECTION_IDS = ['profile', 'proactivity', 'skills', 'mcp'] as const;
type RoleSectionId = typeof ROLE_SECTION_IDS[number];
const ROLE_STORAGE_KEY = 'egosync-role-settings-sections';
const readRoleSections = (): Record<RoleSectionId, boolean> => {
  const defaults = Object.fromEntries(ROLE_SECTION_IDS.map(id => [id, true])) as Record<RoleSectionId, boolean>;
  try {
    const stored = localStorage.getItem(ROLE_STORAGE_KEY);
    return stored ? { ...defaults, ...JSON.parse(stored) } : defaults;
  } catch { return defaults; }
};
const DEFAULT_SKILLS: RoleSkillsConfig = { findSkills: false, skillCreator: false, enabledSkillIds: [] };
const SKILL_OPTIONS: Array<{ key: SkillKey; title: string; source: string; description: string }> = [
  {
    key: 'findSkills',
    title: 'find-skills',
    source: 'Vercel 官方',
    description: '发现并推荐适合当前任务的 Skill。',
  },
  {
    key: 'skillCreator',
    title: 'skill-creator',
    source: 'Anthropic 官方',
    description: '创建或扩展角色需要的新 Skill。',
  },
];

export function SettingsTab({
  role,
  activeRoles,
  onUpdateRole,
}: SettingsTabProps) {
  const [openSections, setOpenSections] = useState<Record<RoleSectionId, boolean>>(readRoleSections);
  const [roleName, setRoleName] = useState(role.name);
  const [roleGoal, setRoleGoal] = useState(role.goal);
  const [rolePersonalityPrompt, setRolePersonalityPrompt] = useState(role.personalityPrompt);
  const [roleIcon, setRoleIcon] = useState(normalizeIconId(role.icon));
  const [roleColor, setRoleColor] = useState(normalizeColorHex(role.color));
  const [saved, setSaved] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [skills, setSkills] = useState<RoleSkillsConfig>(() => parseSkillsConfig(role.skillsConfig));
  const [registrySkills, setRegistrySkills] = useState<SkillRegistryEntry[]>([]);
  const [skillPreview, setSkillPreview] = useState<SkillImportPreview | null>(null);
  const [opencodeSkills, setOpencodeSkills] = useState<OpencodeSkillCandidate[]>([]);
  const [opencodeSkipped, setOpencodeSkipped] = useState<string[]>([]);
  const [isOpencodeExpanded, setIsOpencodeExpanded] = useState(false);
  const [needsFindSkillsPrompt, setNeedsFindSkillsPrompt] = useState(false);
  const [isDiscoveringOpencode, setIsDiscoveringOpencode] = useState(false);
  const [hasDiscoveredOpencode, setHasDiscoveredOpencode] = useState(false);
  const [importingOpencodePath, setImportingOpencodePath] = useState('');
  const [removingOpencodeId, setRemovingOpencodeId] = useState('');
  const [pendingSkillContent, setPendingSkillContent] = useState('');
  const [pendingSkillSourcePath, setPendingSkillSourcePath] = useState('');
  const [reuseAllRoles, setReuseAllRoles] = useState(false);
  const [reuseRoleIds, setReuseRoleIds] = useState<string[]>([role.id]);
  const [isLoadingSkills, setIsLoadingSkills] = useState(false);
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [isImportingSkill, setIsImportingSkill] = useState(false);
  const [proactivityLevel, setProactivityLevel] = useState<ProactivityLevel>(role.proactivityLevel);
  const [pendingSkill, setPendingSkill] = useState<string | null>(null);
  const [deleteSkillTarget, setDeleteSkillTarget] = useState<SkillRegistryEntry | null>(null);
  const [isSavingProactivity, setIsSavingProactivity] = useState(false);
  const [roleMcpServers, setRoleMcpServers] = useState<McpServer[]>([]);
  const [availableMcpServers, setAvailableMcpServers] = useState<McpServer[]>([]);
  const [isLoadingMcpServers, setIsLoadingMcpServers] = useState(false);
  const [isMcpPickerOpen, setIsMcpPickerOpen] = useState(false);
  const [mcpSearch, setMcpSearch] = useState('');
  const [pendingMcpId, setPendingMcpId] = useState<string | null>(null);
  const [settingsSavedMessage, setSettingsSavedMessage] = useState('');
  const [error, setError] = useState('');
  const notifySkillScopeUpdated = async (scopeKind: 'role' | 'butler' | 'all', ownerId: string | null) => {
    try {
      await skillService.notifyScopeUpdated({ scopeKind, ownerId });
    } catch (e) {
      setError(toFriendlyError(e, 'Skill 已更新，但聊天候选刷新失败，请切换页面或稍后重试'));
    }
  };
  const activeRoleIdRef = useRef(role.id);

  useEffect(() => {
    activeRoleIdRef.current = role.id;
  }, [role.id]);

  const filteredAvailableMcpServers = useMemo(() => {
    const query = mcpSearch.trim().toLowerCase();
    if (!query) return availableMcpServers;
    return availableMcpServers.filter(server =>
      server.name.toLowerCase().includes(query) ||
      server.description.toLowerCase().includes(query) ||
      server.commandOrUrl.toLowerCase().includes(query)
    );
  }, [availableMcpServers, mcpSearch]);
  const SelectedIcon = useMemo(() => getRoleIconComponent(roleIcon), [roleIcon]);

  useEffect(() => {
    setRoleName(role.name);
    setRoleGoal(role.goal);
    setRolePersonalityPrompt(role.personalityPrompt);
    setRoleIcon(normalizeIconId(role.icon));
    setRoleColor(normalizeColorHex(role.color));
    setSaved(false);
    setProactivityLevel(role.proactivityLevel);
    setPendingSkill(null);
    setDeleteSkillTarget(null);
    setIsSavingProactivity(false);
    setSettingsSavedMessage('');
    setError('');
    setSkillPreview(null);
    setOpencodeSkills([]);
    setOpencodeSkipped([]);
    setIsOpencodeExpanded(false);
    setNeedsFindSkillsPrompt(false);
    setIsDiscoveringOpencode(false);
    setHasDiscoveredOpencode(false);
    setImportingOpencodePath('');
    setRemovingOpencodeId('');
    setPendingSkillContent('');
    setPendingSkillSourcePath('');
    setReuseAllRoles(false);
    setReuseRoleIds([role.id]);
    setRoleMcpServers([]);
    setAvailableMcpServers([]);
    setIsLoadingMcpServers(false);
    setIsMcpPickerOpen(false);
    setMcpSearch('');
    setPendingMcpId(null);
  }, [role.id]);

  useEffect(() => {
    setRoleName(role.name);
    setRoleGoal(role.goal);
    setRolePersonalityPrompt(role.personalityPrompt);
    setRoleIcon(normalizeIconId(role.icon));
    setRoleColor(normalizeColorHex(role.color));
    setProactivityLevel(role.proactivityLevel);
  }, [role.name, role.goal, role.personalityPrompt, role.icon, role.color, role.proactivityLevel]);

  useEffect(() => {
    setSkills(parseSkillsConfig(role.skillsConfig));
  }, [role.skillsConfig]);

  useTauriEvent<{ ownerId: string }>('skill-registry-updated', (payload) => {
    if (payload.ownerId !== role.id) return;
    void skillService.listForRole(role.id)
      .then(setRegistrySkills)
      .catch(e => setError(toFriendlyError(e, 'Skill 列表刷新失败，请重新打开设置页')));
  }, [role.id]);

  useEffect(() => {
    let cancelled = false;
    setIsLoadingSkills(true);
    skillService.listForRole(role.id)
      .then(items => {
        if (!cancelled) setRegistrySkills(items);
      })
      .catch(e => {
        if (!cancelled) setError(toFriendlyError(e, 'Skill 列表加载失败，请稍后重试'));
      })
      .finally(() => {
        if (!cancelled) setIsLoadingSkills(false);
      });
    return () => {
      cancelled = true;
    };
  }, [role.id]);

  useEffect(() => {
    let cancelled = false;
    setIsLoadingMcpServers(true);
    Promise.all([
      mcpService.listForRole(role.id),
      mcpService.listAvailableForRole(role.id),
    ])
      .then(([bound, available]) => {
        if (!cancelled) {
          setRoleMcpServers(bound);
          setAvailableMcpServers(available);
        }
      })
      .catch(e => {
        if (!cancelled) setError(toFriendlyError(e, 'MCP server 列表加载失败，请稍后重试'));
      })
      .finally(() => {
        if (!cancelled) setIsLoadingMcpServers(false);
      });
    return () => {
      cancelled = true;
    };
  }, [role.id]);

  const refreshMcpServers = async (roleId: string) => {
    const [bound, available] = await Promise.all([
      mcpService.listForRole(roleId),
      mcpService.listAvailableForRole(roleId),
    ]);
    if (activeRoleIdRef.current !== roleId) return;
    setRoleMcpServers(bound);
    setAvailableMcpServers(available);
  };

  const handleSave = async () => {
    const name = roleName.trim();
    if (!name) {
      setError('角色名称不能为空');
      return;
    }

    setIsSaving(true);
    setError('');
    try {
      const updated = await roleService.update(role.id, {
        name,
        icon: roleIcon,
        color: roleColor,
        goal: roleGoal.trim(),
        personalityPrompt: rolePersonalityPrompt.trim(),
      });
      onUpdateRole?.(updated);
      setSaved(true);
      setTimeout(() => setSaved(false), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      setError(toFriendlyError(e, '保存失败，请稍后重试'));
    } finally {
      setIsSaving(false);
    }
  };

  const handleSkillToggle = async (key: SkillKey) => {
    const nextSkills = { ...skills, [key]: !skills[key] };
    await saveSkills(nextSkills, key);
  };

  const handleCustomSkillToggle = async (skillId: string) => {
    const enabled = skills.enabledSkillIds.includes(skillId);
    const nextSkillIds = enabled
      ? skills.enabledSkillIds.filter(id => id !== skillId)
      : [...skills.enabledSkillIds, skillId];
    await saveSkills({ ...skills, enabledSkillIds: nextSkillIds }, skillId);
  };

  const handleCustomSkillDelete = async () => {
    if (!deleteSkillTarget) return;
    const skillId = deleteSkillTarget.id;
    setPendingSkill(skillId);
    setError('');
    setSettingsSavedMessage('');
    try {
      await skillService.delete(skillId);
      await notifySkillScopeUpdated('all', null);
    } catch (e) {
      setError(toFriendlyError(e, '删除自定义 Skill 失败，请稍后重试'));
      setPendingSkill(null);
      return;
    }

    setDeleteSkillTarget(null);
    setRegistrySkills(prev => prev.filter(item => item.id !== skillId));
    setSkills(prev => ({
      ...prev,
      enabledSkillIds: prev.enabledSkillIds.filter(id => id !== skillId),
    }));

    try {
      const [items, refreshedRoles] = await Promise.all([
        skillService.listForRole(role.id),
        roleService.list(),
      ]);
      setRegistrySkills(items);
      refreshedRoles.forEach(item => onUpdateRole?.(item));
      const refreshedCurrentRole = refreshedRoles.find(item => item.id === role.id);
      if (refreshedCurrentRole) {
        setSkills(parseSkillsConfig(refreshedCurrentRole.skillsConfig));
      }
      setSettingsSavedMessage('自定义 Skill 已删除');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      setError(toFriendlyError(e, '自定义 Skill 已删除，但列表刷新失败，请重新打开设置页'));
    } finally {
      setPendingSkill(null);
    }
  };

  const saveSkills = async (nextSkills: RoleSkillsConfig, pendingKey: string): Promise<boolean> => {
    const previousSkills = skills;
    setSkills(nextSkills);
    setPendingSkill(pendingKey);
    setSettingsSavedMessage('');
    setError('');
    try {
      const updated = await roleService.updateSkills(role.id, {
        findSkills: nextSkills.findSkills,
        skillCreator: nextSkills.skillCreator,
        enabledSkillIds: nextSkills.enabledSkillIds,
      });
      onUpdateRole?.(updated);
      setSkills(parseSkillsConfig(updated.skillsConfig));
      await notifySkillScopeUpdated('role', role.id);
      setSettingsSavedMessage('Skill 配置已保存');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
      return true;
    } catch (e) {
      setSkills(previousSkills);
      setError(toFriendlyError(e, 'Skill 配置保存失败，请稍后重试'));
      return false;
    } finally {
      setPendingSkill(null);
    }
  };

  const handleSkillDirectorySelected = async () => {
    setIsPickingDirectory(true);
    setError('');
    setSettingsSavedMessage('');
    setSkillPreview(null);
    try {
      const picked = await skillService.pickCustomDirectory();
      const preview = await skillService.previewCustom({ content: picked.content });
      setPendingSkillContent(picked.content);
      setPendingSkillSourcePath(picked.sourcePath);
      setSkillPreview(preview);
    } catch (e) {
      const text = typeof e === 'string' ? e : (JSON.stringify(e) ?? String(e));
      if (text.includes('未选择 Skill 文件夹')) {
        return;
      }
      setPendingSkillContent('');
      setPendingSkillSourcePath('');
      setError(toFriendlyError(e, '未能从所选 Skill 文件夹读取 SKILL.md'));
    } finally {
      setIsPickingDirectory(false);
    }
  };

  const handleConfirmImport = async (overwriteExisting = false) => {
    if (!pendingSkillContent) return;
    setIsImportingSkill(true);
    setError('');
    setSettingsSavedMessage('');
    try {
      const result = await skillService.importCustom({
        content: pendingSkillContent,
        sourcePath: pendingSkillSourcePath,
        overwriteExisting,
        roleScope: { allRoles: reuseAllRoles, roleIds: reuseAllRoles ? [] : reuseRoleIds },
      });
      const items = await skillService.listForRole(role.id);
      setRegistrySkills(items);
      await notifySkillScopeUpdated('all', null);
      if (result.entry) {
        const refreshedRoles = await roleService.list();
        refreshedRoles.forEach(item => onUpdateRole?.(item));
        const refreshedCurrentRole = refreshedRoles.find(item => item.id === role.id);
        if (refreshedCurrentRole) {
          setSkills(parseSkillsConfig(refreshedCurrentRole.skillsConfig));
        }
      }
      setSkillPreview(result.status === 'duplicate' ? result.preview : null);
      setPendingSkillContent(result.status === 'duplicate' ? pendingSkillContent : '');
      setPendingSkillSourcePath(result.status === 'duplicate' ? pendingSkillSourcePath : '');
      if (!result.runtimeReady) {
        setSettingsSavedMessage('自定义 Skill 已导入，但运行时刷新失败；重启应用后生效');
      } else {
        setSettingsSavedMessage(result.status === 'duplicate' ? 'Skill 已存在，已更新复用范围并可使用' : '自定义 Skill 已导入并可立即使用');
      }
    } catch (e) {
      setError(toFriendlyError(e, '导入自定义 Skill 失败，请稍后重试'));
    } finally {
      setIsImportingSkill(false);
    }
  };

  const handleDiscoverOpencode = async () => {
    const roleId = role.id;
    setError('');
    setSettingsSavedMessage('');
    setOpencodeSkills([]);
    setOpencodeSkipped([]);
    setHasDiscoveredOpencode(false);
    setNeedsFindSkillsPrompt(false);
    if (!skills.findSkills) {
      setNeedsFindSkillsPrompt(true);
      return;
    }
    setIsDiscoveringOpencode(true);
    try {
      const result = await skillService.discoverOpencode(roleId);
      if (activeRoleIdRef.current !== roleId) return;
      setHasDiscoveredOpencode(true);
      setOpencodeSkills(result.items);
      setIsOpencodeExpanded(result.items.length > 0);
      // 跳过摘要由独立的 opencodeSkipped 区块承载（见下方 UI），不复用
      // settingsSavedMessage，避免随后导入操作的结果提示把跳过信息覆盖掉。
      setOpencodeSkipped(result.skipped.reasons);
    } catch (e) {
      if (activeRoleIdRef.current === roleId) {
        setError(toFriendlyError(e, '发现 opencode Skill 失败，请稍后重试'));
      }
    } finally {
      if (activeRoleIdRef.current === roleId) setIsDiscoveringOpencode(false);
    }
  };

  const handleEnableFindSkills = async () => {
    await saveSkills({ ...skills, findSkills: true }, 'findSkills');
  };

  const handleImportOpencode = async (candidate: OpencodeSkillCandidate) => {
    // 与发现流程保持一致：未启用 find-skills 时给出相同引导，而非直接抛后端报错。
    if (!skills.findSkills) {
      setNeedsFindSkillsPrompt(true);
      return;
    }
    setImportingOpencodePath(candidate.sourcePath);
    setError('');
    setSettingsSavedMessage('');
    try {
      const result = await skillService.importOpencode(role.id, {
        sourcePath: candidate.sourcePath,
        roleScope: { allRoles: false, roleIds: [role.id] },
        // 回传发现时的 hash，后端据此拒绝已被替换的源文件（TOCTOU 一致性）。
        expectedContentHash: candidate.contentHash,
      });
      const items = await skillService.listForRole(role.id);
      setRegistrySkills(items);
      await notifySkillScopeUpdated('all', null);
      if (result.entry) {
        const refreshedRoles = await roleService.list();
        refreshedRoles.forEach(item => onUpdateRole?.(item));
        const refreshedCurrentRole = refreshedRoles.find(item => item.id === role.id);
        if (refreshedCurrentRole) {
          setSkills(parseSkillsConfig(refreshedCurrentRole.skillsConfig));
        }
      }
      const duplicate = result.entry ? { kind: 'contentHash' as const, existing: result.entry } : candidate.duplicate;
      setOpencodeSkills(prev => prev.map(item => item.sourcePath === candidate.sourcePath ? { ...item, alreadyImported: true, duplicate } : item));
      // 同步失败时不能谎称"已启用"：registry 已登记但 opencode agent 未生效，
      // 如实提示用户稍后会自动重试同步。
      if (result.status === 'duplicate') {
        setSettingsSavedMessage('Skill 已存在，已更新复用范围');
      } else if (result.synced && result.runtimeReady) {
        setSettingsSavedMessage('opencode Skill 已导入并可立即使用');
      } else if (result.synced) {
        setSettingsSavedMessage('opencode Skill 已导入，但运行时刷新失败；重启应用后生效');
      } else {
        setSettingsSavedMessage('opencode Skill 已导入，但同步到 agent 暂时失败，将在下次同步时自动生效');
      }
    } catch (e) {
      setError(toFriendlyError(e, '导入 opencode Skill 失败，请稍后重试'));
    } finally {
      setImportingOpencodePath('');
    }
  };

  const handleRemoveOpencode = async (candidate: OpencodeSkillCandidate) => {
    const skillId = candidate.duplicate?.existing.id;
    if (!skillId) return;
    setRemovingOpencodeId(skillId);
    setError('');
    setSettingsSavedMessage('');
    try {
      await skillService.removeFromRole(skillId, role.id);
      await notifySkillScopeUpdated('role', role.id);
      const items = await skillService.listForRole(role.id);
      setRegistrySkills(items);
      const refreshedRoles = await roleService.list();
      refreshedRoles.forEach(item => onUpdateRole?.(item));
      const refreshedCurrentRole = refreshedRoles.find(item => item.id === role.id);
      if (refreshedCurrentRole) {
        setSkills(parseSkillsConfig(refreshedCurrentRole.skillsConfig));
      } else {
        setSkills(prev => ({
          ...prev,
          enabledSkillIds: prev.enabledSkillIds.filter(id => id !== skillId),
        }));
      }
      setOpencodeSkills(prev => prev.map(item => item.sourcePath === candidate.sourcePath ? { ...item, alreadyImported: false } : item));
      setSettingsSavedMessage('opencode Skill 已取消导入');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      setError(toFriendlyError(e, '取消导入 opencode Skill 失败，请稍后重试'));
    } finally {
      setRemovingOpencodeId('');
    }
  };

  const handleAddMcpToRole = async (serverId: string) => {
    const roleId = role.id;
    setPendingMcpId(serverId);
    setError('');
    setSettingsSavedMessage('');
    try {
      await mcpService.addToRole(roleId, serverId);
      await refreshMcpServers(roleId);
      if (activeRoleIdRef.current !== roleId) return;
      setMcpSearch('');
      setIsMcpPickerOpen(false);
      setSettingsSavedMessage('MCP server 已添加');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      if (activeRoleIdRef.current === roleId) setError(toFriendlyError(e, 'MCP server 添加失败，请稍后重试'));
    } finally {
      if (activeRoleIdRef.current === roleId) setPendingMcpId(null);
    }
  };

  const handleRemoveMcpFromRole = async (serverId: string) => {
    const roleId = role.id;
    setPendingMcpId(serverId);
    setError('');
    setSettingsSavedMessage('');
    try {
      await mcpService.removeFromRole(roleId, serverId);
      await refreshMcpServers(roleId);
      if (activeRoleIdRef.current !== roleId) return;
      setSettingsSavedMessage('MCP server 已移除');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      if (activeRoleIdRef.current === roleId) setError(toFriendlyError(e, 'MCP server 移除失败，请稍后重试'));
    } finally {
      if (activeRoleIdRef.current === roleId) setPendingMcpId(null);
    }
  };

  const handleProactivityChange = async (nextLevel: ProactivityLevel) => {
    if (nextLevel === proactivityLevel) return;
    const previousLevel = proactivityLevel;
    setProactivityLevel(nextLevel);
    setIsSavingProactivity(true);
    setSettingsSavedMessage('');
    setError('');
    try {
      const updated = await roleService.updateProactivity(role.id, { proactivityLevel: nextLevel });
      onUpdateRole?.(updated);
      setProactivityLevel(updated.proactivityLevel);
      setSettingsSavedMessage('主动性级别已保存');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      setProactivityLevel(previousLevel);
      setError(toFriendlyError(e, '主动性级别保存失败，请稍后重试'));
    } finally {
      setIsSavingProactivity(false);
    }
  };

  useEffect(() => {
    localStorage.setItem(ROLE_STORAGE_KEY, JSON.stringify(openSections));
  }, [openSections]);

  const setAllSections = (open: boolean) => setOpenSections(Object.fromEntries(ROLE_SECTION_IDS.map(id => [id, open])) as Record<RoleSectionId, boolean>);
  const toggleSection = (id: RoleSectionId) => setOpenSections(prev => ({ ...prev, [id]: !prev[id] }));
  const sectionClass = (id: RoleSectionId, base = '') => cn(base, !openSections[id] && '[&>*:not(:first-child)]:hidden');
  const sectionHeader = (id: RoleSectionId, title: string) => (
    <button type="button" aria-expanded={openSections[id]} onClick={() => toggleSection(id)} className="mb-3 flex w-full items-center justify-between rounded-lg text-left text-[14px] font-semibold text-slate-800 outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/40 dark:text-slate-100">
      <span>{title}</span>{openSections[id] ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
    </button>
  );

  return (
    <div className="space-y-8">
      <div className="flex justify-end gap-2">
        <button type="button" onClick={() => setAllSections(true)} className="rounded-lg border border-slate-200 px-3 py-1.5 text-[12px] font-medium text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800">全部展开</button>
        <button type="button" onClick={() => setAllSections(false)} className="rounded-lg border border-slate-200 px-3 py-1.5 text-[12px] font-medium text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800">全部折叠</button>
      </div>
      <div className={sectionClass('profile')}>
        {sectionHeader('profile', '角色信息')}
        <div className="space-y-4">
          <div>
            <label htmlFor="role-name" className="block text-[13px] font-medium text-slate-600 dark:text-slate-300 mb-1.5">名称</label>
            <input id="role-name" type="text" value={roleName} onChange={e => setRoleName(e.target.value)} className="w-full bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-600 dark:text-slate-300 mb-2">图标</label>
            <div className="flex items-center gap-3 mb-3">
              <div className="w-11 h-11 rounded-xl flex items-center justify-center text-white" style={{ backgroundColor: roleColor }}>
                <SelectedIcon size={22} strokeWidth={2} />
              </div>
              <span className="text-[13px] text-slate-500 dark:text-slate-400">选择一个稳定的角色标识</span>
            </div>
            <div className="grid grid-cols-6 gap-2">
              {ROLE_ICONS.map(option => {
                const Icon = option.component;
                return (
                  <button
                    key={option.id}
                    type="button"
                    title={option.label}
                    onClick={() => setRoleIcon(option.id)}
                    className={cn(
                      'w-10 h-10 rounded-lg border flex items-center justify-center transition-all',
                      roleIcon === option.id ? 'border-indigo-500 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600' : 'border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-500 dark:text-slate-400 hover:border-indigo-200 dark:hover:border-indigo-600 hover:text-indigo-500'
                    )}
                  >
                    <Icon size={18} strokeWidth={2} />
                  </button>
                );
              })}
            </div>
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-600 dark:text-slate-300 mb-2">颜色</label>
            <div className="grid grid-cols-4 gap-2">
              {ROLE_COLORS.map(option => (
                <button
                  key={option.hex}
                  type="button"
                  onClick={() => setRoleColor(option.hex)}
                  className={cn(
                    'flex items-center gap-2 rounded-lg border px-3 py-2 text-[12px] font-medium transition-all',
                    roleColor === option.hex ? 'border-indigo-500 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700' : 'border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:border-indigo-200 dark:hover:border-indigo-600'
                  )}
                >
                  <span className="w-4 h-4 rounded-full" style={{ backgroundColor: option.hex }} />
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label htmlFor="role-goal" className="block text-[13px] font-medium text-slate-600 dark:text-slate-300 mb-1.5">目标</label>
            <textarea id="role-goal" value={roleGoal} onChange={e => setRoleGoal(e.target.value)} placeholder="这个角色要达成的核心目标..." rows={2} className="w-full bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
          </div>

          <div>
            <label htmlFor="role-personality" className="block text-[13px] font-medium text-slate-600 dark:text-slate-300 mb-1.5">角色个性描述</label>
            <textarea
              id="role-personality"
              value={rolePersonalityPrompt}
              onChange={e => setRolePersonalityPrompt(e.target.value)}
              placeholder="描述这个角色下次回复时应采用的语气、判断方式和表达习惯..."
              rows={4}
              className="w-full bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none"
            />
            <div className="mt-2 rounded-lg bg-slate-50 dark:bg-slate-800 border border-slate-100 dark:border-slate-700 px-3 py-2 text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">
              <div>模板参考：</div>
              <div>产品经理：简洁专业，偏结构化表达，先判断优先级再给建议。</div>
              <div>家庭：温暖关怀，偏情感支持，先回应感受再给建议。</div>
              <div>学习者：好奇探索，偏启发式提问，鼓励持续尝试。</div>
            </div>
          </div>
          <button type="button" onClick={handleSave} disabled={isSaving} className={cn('mt-4 inline-flex items-center gap-2 rounded-lg border px-3.5 py-2 text-[12.5px] font-medium transition-colors disabled:opacity-60', saved ? 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300' : 'border-slate-200 text-slate-600 hover:border-indigo-200 hover:bg-indigo-50 hover:text-indigo-600 dark:border-slate-700 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-900/30')}>
            {saved ? <><Check size={16} /> 已保存</> : isSaving ? '保存中...' : '保存角色信息'}
          </button>
        </div>
      </div>

      <div className={sectionClass('proactivity', 'pt-6 border-t border-slate-200/80 dark:border-slate-700/80')}>
        {sectionHeader('proactivity', '主动性级别')}
        <ProactivityToggle
          level={proactivityLevel}
          onChange={handleProactivityChange}
          disabled={isSavingProactivity}
        />
        <p className="text-[12px] text-slate-400 dark:text-slate-500 mt-2.5 leading-relaxed">V1 仅保存角色主动性级别；实际主动建议会在后续主动循环中生效。</p>
      </div>

      <div className={sectionClass('skills', 'pt-6 border-t border-slate-200/80 dark:border-slate-700/80')}>
        {sectionHeader('skills', 'Skill 配置')}
        <div className="space-y-3">
          {SKILL_OPTIONS.map(option => {
            const enabled = skills[option.key];
            return (
              <div key={option.key} className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 shadow-sm">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="flex items-center gap-2.5">
                      <span className={cn('w-2.5 h-2.5 rounded-full', enabled ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]' : 'bg-slate-300')} />
                      <span className="text-[14.5px] font-medium text-slate-800 dark:text-slate-100">{option.title}</span>
                      <span className="text-[11px] font-medium text-slate-500 dark:text-slate-400 bg-slate-100 dark:bg-slate-700 px-2 py-0.5 rounded-full border border-slate-200 dark:border-slate-700">{option.source}</span>
                    </div>
                    <p className="mt-2 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">{option.description}</p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={enabled}
                    disabled={pendingSkill !== null}
                    onClick={() => handleSkillToggle(option.key)}
                    className={cn(
                      'relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-60',
                      enabled ? 'bg-indigo-600' : 'bg-slate-300',
                    )}
                  >
                    <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white dark:bg-slate-800 shadow transition-transform', enabled ? 'translate-x-5' : 'translate-x-0')} />
                    <span className="sr-only">{option.title}</span>
                  </button>
                </div>
              </div>
            );
          })}

          <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 shadow-sm">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <div className="text-[14.5px] font-medium text-slate-800 dark:text-slate-100">opencode 生态 Skill</div>
                <p className="mt-1 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">扫描本机 opencode 项目级与全局 Skill 目录，不下载远程内容。</p>
              </div>
              <button
                type="button"
                title="发现 opencode Skill"
                aria-label="发现 opencode Skill"
                onClick={handleDiscoverOpencode}
                disabled={isDiscoveringOpencode}
                className="inline-flex min-w-[3.5rem] shrink-0 items-center justify-center whitespace-nowrap rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-[12px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isDiscoveringOpencode ? '发现中...' : '发现'}
              </button>
            </div>
            {!skills.findSkills && needsFindSkillsPrompt && (
              <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-[12.5px] text-amber-700">
                <div>需要先启用 find-skills 才能发现可用 Skill</div>
                <button
                  type="button"
                  onClick={handleEnableFindSkills}
                  disabled={pendingSkill !== null}
                  className="mt-2 rounded-md bg-amber-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  启用 find-skills
                </button>
              </div>
            )}
            {opencodeSkipped.length > 0 && (
              <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-[12.5px] text-amber-700">
                <div>已跳过 {opencodeSkipped.length} 个无效 Skill：</div>
                <ul className="mt-1 list-disc space-y-0.5 pl-4">
                  {opencodeSkipped.slice(0, 5).map((reason, index) => (
                    <li key={`${reason}-${index}`} className="break-words">{reason}</li>
                  ))}
                </ul>
                {opencodeSkipped.length > 5 && (
                  <div className="mt-1 text-amber-600">…另有 {opencodeSkipped.length - 5} 项已跳过</div>
                )}
              </div>
            )}
            {hasDiscoveredOpencode && opencodeSkills.length === 0 && opencodeSkipped.length === 0 && (
              <div className="mt-3 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-[12.5px] text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                已扫描项目级与全局 Skill 目录，未发现 opencode Skill。
              </div>
            )}
            {opencodeSkills.length > 0 && (
              <div className="mt-3 space-y-2">
                <button
                  type="button"
                  onClick={() => setIsOpencodeExpanded(prev => !prev)}
                  className="flex w-full items-center justify-between rounded-lg border border-slate-100 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2 text-[12.5px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700"
                >
                  <span>发现 {opencodeSkills.length} 个 opencode Skill</span>
                  <span className="inline-flex items-center gap-1">
                    {isOpencodeExpanded ? '收起' : '展开'}
                    {isOpencodeExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                  </span>
                </button>
                {isOpencodeExpanded && opencodeSkills.map(candidate => {
                  const importedSkillId = candidate.duplicate?.existing.id;
                  const isRemoving = importedSkillId === removingOpencodeId;
                  return (
                    <div key={candidate.sourcePath} className="rounded-lg border border-slate-100 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-3">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="break-words text-[13.5px] font-medium text-slate-800 dark:text-slate-100">{candidate.name}</span>
                            <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">opencode</span>
                            <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">{candidate.sourceLocation}</span>
                            {candidate.alreadyImported && <span className="rounded-full border border-emerald-100 bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-700">已导入</span>}
                          </div>
                          <div className="group relative mt-1.5">
                            <p data-testid={`opencode-skill-description-${candidate.contentHash}`} className="line-clamp-2 break-words text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">{candidate.description}</p>
                            <div data-testid={`opencode-skill-tooltip-${candidate.contentHash}`} className="pointer-events-none absolute left-0 top-full z-20 mt-1 hidden max-w-[min(28rem,calc(100vw-3rem))] rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-[12px] leading-relaxed text-slate-600 dark:text-slate-300 shadow-xl group-hover:block">
                              {candidate.description}
                            </div>
                          </div>
                        </div>
                        {candidate.alreadyImported ? (
                          <button
                            type="button"
                            onClick={() => handleRemoveOpencode(candidate)}
                            disabled={!importedSkillId || removingOpencodeId !== '' || importingOpencodePath !== ''}
                            className="shrink-0 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-1.5 text-[12px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60"
                          >
                            {isRemoving ? '取消中...' : '取消导入'}
                          </button>
                        ) : (
                          <button
                            type="button"
                            onClick={() => handleImportOpencode(candidate)}
                            disabled={importingOpencodePath !== '' || removingOpencodeId !== ''}
                            className="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
                          >
                            {importingOpencodePath === candidate.sourcePath ? '导入中...' : '导入'}
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 shadow-sm">
            <div className="mb-4 space-y-3">
              <div className="min-w-0">
                <div className="text-[14.5px] font-medium text-slate-800 dark:text-slate-100">自定义 Skill</div>
                <p className="mt-1 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">选择包含 SKILL.md 的文件夹后，可按角色启用。</p>
              </div>
              <button
                type="button"
                onClick={handleSkillDirectorySelected}
                disabled={isImportingSkill || isPickingDirectory}
                className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-indigo-200 bg-indigo-50 dark:bg-indigo-900/30 px-3 py-2 text-[12px] font-medium text-indigo-700 hover:bg-indigo-100 disabled:cursor-not-allowed disabled:opacity-60 sm:w-auto"
              >
                <Upload size={14} /> {isPickingDirectory ? '选择中...' : '选择skill文件夹'}
              </button>
            </div>

            {skillPreview && (
              <div className="mb-3 rounded-lg border border-indigo-100 dark:border-indigo-700 bg-indigo-50 dark:bg-indigo-900/30 px-3 py-3 text-[12.5px] text-slate-600 dark:text-slate-300">
                <div className="font-semibold text-slate-800 dark:text-slate-100">预览：{skillPreview.name}</div>
                <div className="mt-1">{skillPreview.description}</div>
                {skillPreview.duplicate && (
                  <div className="mt-2 rounded-md border border-amber-200 bg-amber-50 px-2.5 py-2 text-amber-700">
                    {skillPreview.duplicate.kind === 'contentHash' ? '相同内容的 Skill 已存在。' : '同名 Skill 已存在。'}可取消或覆盖元数据。
                  </div>
                )}
                <div className="mt-3 rounded-md border border-indigo-100 dark:border-indigo-700 bg-white/70 dark:bg-slate-800/70 px-3 py-2">
                  <div className="mb-2 text-[12px] font-medium text-slate-700 dark:text-slate-300">复用到以下角色</div>
                  <label className="flex items-center gap-2 text-[12.5px] text-slate-600 dark:text-slate-300">
                    <input
                      type="checkbox"
                      checked={reuseAllRoles}
                      onChange={e => setReuseAllRoles(e.target.checked)}
                      className="h-3.5 w-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                    />
                    全部角色
                  </label>
                  {!reuseAllRoles && (
                    <div className="mt-2 space-y-1.5">
                      {(activeRoles ?? [role]).map(item => (
                        <label key={item.id} className="flex items-center gap-2 text-[12.5px] text-slate-600 dark:text-slate-300">
                          <input
                            type="checkbox"
                            checked={reuseRoleIds.includes(item.id)}
                            onChange={e => {
                              setReuseRoleIds(prev => e.target.checked
                                ? [...prev.filter(id => id !== item.id), item.id]
                                : prev.filter(id => id !== item.id));
                            }}
                            className="h-3.5 w-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                          />
                          {item.name}
                        </label>
                      ))}
                      <label className="flex items-center gap-2 text-[12.5px] text-slate-600 dark:text-slate-300">
                        <input
                          type="checkbox"
                          checked={reuseRoleIds.includes(BUTLER_SCOPE_ID)}
                          onChange={e => {
                            setReuseRoleIds(prev => e.target.checked
                              ? [...prev.filter(id => id !== BUTLER_SCOPE_ID), BUTLER_SCOPE_ID]
                              : prev.filter(id => id !== BUTLER_SCOPE_ID));
                          }}
                          className="h-3.5 w-3.5 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                        />
                        管家
                      </label>
                    </div>
                  )}
                </div>
                <div className="mt-3 flex justify-end gap-2">
                  <button type="button" onClick={() => { setSkillPreview(null); setPendingSkillContent(''); }} className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-1.5 text-[12px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700">取消</button>
                  <button type="button" onClick={() => handleConfirmImport(false)} disabled={isImportingSkill} className="rounded-lg bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60">确认导入</button>
                  {skillPreview.duplicate && (
                    <button type="button" onClick={() => handleConfirmImport(true)} disabled={isImportingSkill} className="rounded-lg bg-amber-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-60">覆盖元数据</button>
                  )}
                </div>
              </div>
            )}

            <div className="space-y-2">
              {isLoadingSkills && <div className="text-[12.5px] text-slate-400 dark:text-slate-500">正在加载自定义 Skill...</div>}
              {!isLoadingSkills && registrySkills.length === 0 && <div className="text-[12.5px] text-slate-400 dark:text-slate-500">暂无自定义 Skill。</div>}
              {registrySkills.map(item => {
                const enabled = skills.enabledSkillIds.includes(item.id);
                return (
                  <div key={item.id} data-testid={`custom-skill-card-${item.id}`} className="rounded-lg border border-slate-100 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-3">
                    <div className="flex items-start justify-between gap-4">
                      <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 flex-wrap items-center gap-2">
                          <span className={cn('w-2.5 h-2.5 shrink-0 rounded-full', enabled ? 'bg-emerald-500' : 'bg-slate-300')} />
                          <span className="min-w-0 break-words text-[13.5px] font-medium text-slate-800 dark:text-slate-100">{item.name}</span>
                          <span className="shrink-0 text-[11px] font-medium text-slate-500 dark:text-slate-400 bg-white dark:bg-slate-800 px-2 py-0.5 rounded-full border border-slate-200 dark:border-slate-700">自定义</span>
                        </div>
                      </div>
                      <div data-testid={`custom-skill-actions-${item.id}`} className="flex shrink-0 items-center gap-2">
                        <button
                          type="button"
                          aria-label={`删除 ${item.name}`}
                          disabled={pendingSkill !== null}
                          onClick={() => setDeleteSkillTarget(item)}
                          className="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-red-100 bg-red-50 text-red-500 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          <Trash2 size={14} />
                        </button>
                        <button
                          type="button"
                          role="switch"
                          aria-checked={enabled}
                          disabled={pendingSkill !== null}
                          onClick={() => handleCustomSkillToggle(item.id)}
                          className={cn(
                            'relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-60',
                            enabled ? 'bg-indigo-600' : 'bg-slate-300',
                          )}
                        >
                          <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white dark:bg-slate-800 shadow transition-transform', enabled ? 'translate-x-5' : 'translate-x-0')} />
                          <span className="sr-only">{item.name}</span>
                        </button>
                      </div>
                    </div>
                    <div className="group relative mt-1.5">
                      <p data-testid={`custom-skill-description-${item.id}`} className="line-clamp-2 break-words text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">{item.description}</p>
                      <div data-testid={`custom-skill-tooltip-${item.id}`} className="pointer-events-none absolute left-0 top-full z-20 mt-1 hidden max-w-[min(28rem,calc(100vw-3rem))] rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-[12px] leading-relaxed text-slate-600 dark:text-slate-300 shadow-xl group-hover:block">
                        {item.description}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
      <div className={sectionClass('mcp', 'pt-6 border-t border-slate-200/80 dark:border-slate-700/80')}>
        {sectionHeader('mcp', 'MCP Server配置')}
        <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-4 shadow-sm">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className="text-[14.5px] font-medium text-slate-800 dark:text-slate-100">当前角色可用 MCP server</div>
              <p className="mt-1 text-[12.5px] leading-relaxed text-slate-500 dark:text-slate-400">这里只展示已为当前角色启用的 MCP；全局未绑定的 server 不会出现在角色能力中。</p>
            </div>
            <button
              type="button"
              onClick={() => setIsMcpPickerOpen(prev => !prev)}
              disabled={isLoadingMcpServers}
              className="inline-flex shrink-0 items-center justify-center rounded-lg border border-indigo-200 bg-indigo-50 dark:bg-indigo-900/30 px-3 py-2 text-[12px] font-medium text-indigo-700 hover:bg-indigo-100 disabled:cursor-not-allowed disabled:opacity-60"
            >
              添加 MCP server
            </button>
          </div>

          <div className="mt-3 space-y-2">
            {isLoadingMcpServers && <div className="text-[12.5px] text-slate-400 dark:text-slate-500">正在加载 MCP server...</div>}
            {!isLoadingMcpServers && roleMcpServers.length === 0 && <div className="text-[12.5px] text-slate-400 dark:text-slate-500">当前角色暂无 MCP server。</div>}
            {roleMcpServers.map(server => (
              <div key={server.id} className="rounded-lg border border-slate-100 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className={cn('w-2.5 h-2.5 rounded-full', server.enabled ? 'bg-emerald-500' : 'bg-slate-300')} />
                      <span className="break-words text-[13.5px] font-medium text-slate-800 dark:text-slate-100">{server.name}</span>
                      <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">{mcpServerTypeLabel(server.serverType)}</span>
                      {!server.enabled && <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">已停用</span>}
                    </div>
                    <p className="mt-1.5 break-words text-[12px] leading-relaxed text-slate-500 dark:text-slate-400">{server.description || server.commandOrUrl}</p>
                  </div>
                  <button
                    type="button"
                    aria-label={`移除 ${server.name}`}
                    onClick={() => handleRemoveMcpFromRole(server.id)}
                    disabled={pendingMcpId !== null}
                    className="shrink-0 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-1.5 text-[12px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    {pendingMcpId === server.id ? '移除中...' : '移除'}
                  </button>
                </div>
              </div>
            ))}
          </div>

          {isMcpPickerOpen && (
            <div className="mt-3 rounded-lg border border-indigo-100 dark:border-indigo-700 bg-indigo-50 dark:bg-indigo-900/30 px-3 py-3">
              <input
                type="text"
                value={mcpSearch}
                onChange={e => setMcpSearch(e.target.value)}
                placeholder="搜索 MCP server"
                className="w-full rounded-lg border border-indigo-100 dark:border-indigo-700 bg-white dark:bg-slate-800 px-3 py-2 text-[13px] outline-none focus:border-indigo-300 focus:ring-2 focus:ring-indigo-500/20"
              />
              <div className="mt-2 space-y-2">
                {filteredAvailableMcpServers.length === 0 ? (
                  <div className="text-[12.5px] text-slate-500 dark:text-slate-400">没有可添加的 MCP server</div>
                ) : filteredAvailableMcpServers.map(server => (
                  <div key={server.id} className="flex items-start justify-between gap-3 rounded-lg border border-indigo-100 dark:border-indigo-700 bg-white dark:bg-slate-800 px-3 py-2">
                    <div className="min-w-0">
                      <div className="break-words text-[13.5px] font-medium text-slate-800 dark:text-slate-100">{server.name}</div>
                      <div className="mt-1 break-words text-[12px] text-slate-500 dark:text-slate-400">{server.description || server.commandOrUrl}</div>
                    </div>
                    <button
                      type="button"
                      aria-label={`添加 ${server.name}`}
                      onClick={() => handleAddMcpToRole(server.id)}
                      disabled={pendingMcpId !== null}
                      className="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
                    >
                      {pendingMcpId === server.id ? '添加中...' : '添加'}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>


      <div className="pt-6 border-t border-slate-200/80 dark:border-slate-700/80">
        {settingsSavedMessage && (
          <div className="mb-3 rounded-lg border border-emerald-100 bg-emerald-50 px-3 py-2 text-[13px] text-emerald-700 flex items-center gap-2">
            <Check size={14} /> {settingsSavedMessage}
          </div>
        )}
        {error && (
          <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> {error}
          </div>
        )}
      </div>

      {deleteSkillTarget && (
        <div className="fixed inset-0 z-[220] flex items-center justify-center bg-slate-900/20 backdrop-blur-sm">
          <div role="dialog" aria-modal="true" aria-label="删除自定义 Skill" className="w-[380px] rounded-2xl bg-white dark:bg-slate-800 p-6 shadow-2xl">
            <h3 className="mb-3 text-[16px] font-semibold text-slate-800 dark:text-slate-100">删除自定义 Skill</h3>
            <p className="mb-5 text-[14px] leading-relaxed text-slate-600 dark:text-slate-300">
              确认删除「{deleteSkillTarget.name}」吗？该 Skill 将从所有角色和管家中删除。
            </p>
            <div className="flex justify-end gap-3">
              <button type="button" onClick={() => setDeleteSkillTarget(null)} disabled={pendingSkill !== null} className="rounded-lg border border-slate-200 dark:border-slate-700 px-5 py-2.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60">取消</button>
              <button
                type="button"
                onClick={handleCustomSkillDelete}
                disabled={pendingSkill !== null}
                className="rounded-lg bg-red-600 px-5 py-2.5 text-[13px] font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {pendingSkill === deleteSkillTarget.id ? '删除中...' : '确认删除'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function parseSkillsConfig(raw: string): RoleSkillsConfig {
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const meta = typeof parsed.meta === 'object' && parsed.meta !== null ? parsed.meta as Record<string, unknown> : {};
    const enabledSkillIds = Array.isArray(parsed.enabledSkillIds)
      ? parsed.enabledSkillIds.filter((id): id is string => typeof id === 'string' && id.trim().length > 0)
      : [];
    return {
      findSkills: meta.findSkills === true || parsed['find-skills'] === true || parsed.findSkills === true,
      skillCreator: meta.skillCreator === true || parsed['skill-creator'] === true || parsed.skillCreator === true,
      enabledSkillIds,
    };
  } catch {
    return DEFAULT_SKILLS;
  }
}

function mcpServerTypeLabel(value: string) {
  if (value === 'streamable_http') return 'Streamable HTTP';
  if (value === 'stdio' || value === 'command') return 'stdio';
  return 'SSE';
}

function toFriendlyError(error: unknown, fallback: string) {
  // P8: error 可能为 undefined/null（Promise 以 undefined 拒绝时 JSON.stringify 返回 undefined），
  // 直接 .includes 会二次抛错，先兜底。
  if (error == null) return fallback;
  const validationError = typeof error === 'object'
    && 'ValidationError' in error
    && typeof error.ValidationError === 'string'
    ? error.ValidationError
    : null;
  const text = validationError ?? (typeof error === 'string' ? error : (JSON.stringify(error) ?? String(error)));
  if (text.includes('需要先启用 find-skills')) {
    return '需要先启用 find-skills 才能发现可用 Skill';
  }
  if (text.includes('已被另一来源占用')) {
    return '存在同名但来源不同的 Skill，请检查已导入的 Skill';
  }
  if (text.includes(MIN_ACTIVE_ROLE_MESSAGE)) return MIN_ACTIVE_ROLE_MESSAGE;
  if (text.includes('角色名称不能为空')) return '角色名称不能为空';
  // P9: 不回显后端原始错误文本（可能含文件系统路径等内部细节），统一映射为固定友好文案。
  if (text.includes('frontmatter') || text.includes('SKILL.md')) {
    return 'SKILL.md 解析失败，请检查 frontmatter 中的 name 和 description';
  }
  return fallback;
}
