import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import type { DiscoverOpencodeSkillsResult, ImportCustomSkillInput, ImportCustomSkillResult, ImportOpencodeSkillInput, ImportOpencodeSkillResult, PickCustomSkillDirectoryResult, PreviewCustomSkillInput, SkillImportPreview, SkillRegistryEntry, SelectableSkill, SkillScopeUpdatedPayload } from '../types/skill';

export const skillService = {
  listRegistry: () => invoke<SkillRegistryEntry[]>('skill_list_registry'),
  listForRole: (roleId: string) => invoke<SkillRegistryEntry[]>('skill_list_for_role', { roleId }),
  listAllRoleSkills: () => invoke<SkillRegistryEntry[]>('skill_list_all_role_skills'),
  listSelectableForScope: (roleId?: string) => invoke<SelectableSkill[]>('skill_list_selectable_for_scope', { roleId: roleId ?? null }),
  notifyScopeUpdated: (payload: SkillScopeUpdatedPayload) => emit('skill-scope-updated', payload),
  pickCustomDirectory: () => invoke<PickCustomSkillDirectoryResult>('skill_pick_custom_directory'),
  previewCustom: (input: PreviewCustomSkillInput) => invoke<SkillImportPreview>('skill_preview_custom', { input }),
  discoverOpencode: (roleId: string) => invoke<DiscoverOpencodeSkillsResult>('skill_discover_opencode', { roleId }),
  importOpencode: (roleId: string, input: ImportOpencodeSkillInput) => invoke<ImportOpencodeSkillResult>('skill_import_opencode', { roleId, input }),
  importCustom: (input: ImportCustomSkillInput) => invoke<ImportCustomSkillResult>('skill_import_custom', { input }),
  removeFromRole: (skillId: string, roleId: string) => invoke<void>('skill_remove_from_role', { skillId, roleId }),
  delete: (skillId: string) => invoke<void>('skill_delete', { skillId }),
};
