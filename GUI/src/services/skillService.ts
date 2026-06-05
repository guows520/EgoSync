import { invoke } from '@tauri-apps/api/core';
import type { ImportCustomSkillInput, ImportCustomSkillResult, PickCustomSkillDirectoryResult, PreviewCustomSkillInput, SkillImportPreview, SkillRegistryEntry } from '../types/skill';

export const skillService = {
  listRegistry: () => invoke<SkillRegistryEntry[]>('skill_list_registry'),
  listForRole: (roleId: string) => invoke<SkillRegistryEntry[]>('skill_list_for_role', { roleId }),
  listAllRoleSkills: () => invoke<SkillRegistryEntry[]>('skill_list_all_role_skills'),
  pickCustomDirectory: () => invoke<PickCustomSkillDirectoryResult>('skill_pick_custom_directory'),
  previewCustom: (input: PreviewCustomSkillInput) => invoke<SkillImportPreview>('skill_preview_custom', { input }),
  importCustom: (input: ImportCustomSkillInput) => invoke<ImportCustomSkillResult>('skill_import_custom', { input }),
  removeFromRole: (skillId: string, roleId: string) => invoke<void>('skill_remove_from_role', { skillId, roleId }),
  delete: (skillId: string) => invoke<void>('skill_delete', { skillId }),
};
