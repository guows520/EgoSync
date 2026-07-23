import { invoke } from '@tauri-apps/api/core';
import type { DiscoverOpencodeSkillsResult, ImportCustomSkillInput, ImportCustomSkillResult, ImportOpencodeSkillInput, ImportOpencodeSkillResult, PickCustomSkillDirectoryResult, PreviewCustomSkillInput, SkillImportPreview, SkillRegistryEntry } from '../types/skill';

export const skillService = {
  listRegistry: () => invoke<SkillRegistryEntry[]>('skill_list_registry'),
  listForRole: (roleId: string) => invoke<SkillRegistryEntry[]>('skill_list_for_role', { roleId }),
  listAllRoleSkills: () => invoke<SkillRegistryEntry[]>('skill_list_all_role_skills'),
  listEnabledForScope: (roleId?: string) => invoke<SkillRegistryEntry[]>('skill_list_enabled_for_scope', { roleId: roleId ?? null }),
  pickCustomDirectory: () => invoke<PickCustomSkillDirectoryResult>('skill_pick_custom_directory'),
  previewCustom: (input: PreviewCustomSkillInput) => invoke<SkillImportPreview>('skill_preview_custom', { input }),
  discoverOpencode: (roleId: string) => invoke<DiscoverOpencodeSkillsResult>('skill_discover_opencode', { roleId }),
  importOpencode: (roleId: string, input: ImportOpencodeSkillInput) => invoke<ImportOpencodeSkillResult>('skill_import_opencode', { roleId, input }),
  importCustom: (input: ImportCustomSkillInput) => invoke<ImportCustomSkillResult>('skill_import_custom', { input }),
  removeFromRole: (skillId: string, roleId: string) => invoke<void>('skill_remove_from_role', { skillId, roleId }),
  delete: (skillId: string) => invoke<void>('skill_delete', { skillId }),
};
