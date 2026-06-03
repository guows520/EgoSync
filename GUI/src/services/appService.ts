import { invoke } from '@tauri-apps/api/core';
import type { RoleSkillsConfig, UpdateRoleSkillsInput } from '../types/role';

export const appService = {
  isFirstLaunch: () => invoke<boolean>('app_is_first_launch'),
  completeOnboarding: () => invoke<void>('app_complete_onboarding'),
  isLlmConfigured: () => invoke<boolean>('app_is_llm_configured'),
  getButlerSkills: () => invoke<RoleSkillsConfig>('app_get_butler_skills'),
  updateButlerSkills: (input: UpdateRoleSkillsInput) => invoke<RoleSkillsConfig>('app_update_butler_skills', { input }),
};