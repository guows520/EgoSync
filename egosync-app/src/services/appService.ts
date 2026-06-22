import { invoke } from '@tauri-apps/api/core';
import type { ButlerSkillsConfig, UpdateRoleSkillsInput } from '../types/role';

export const appService = {
  isFirstLaunch: () => invoke<boolean>('app_is_first_launch'),
  completeOnboarding: () => invoke<void>('app_complete_onboarding'),
  isLlmConfigured: () => invoke<boolean>('app_is_llm_configured'),
  getButlerSkills: () => invoke<ButlerSkillsConfig>('app_get_butler_skills'),
  updateButlerSkills: (input: UpdateRoleSkillsInput) => invoke<ButlerSkillsConfig>('app_update_butler_skills', { input }),
  getSetting: (key: string) => invoke<string | null>('app_get_setting', { key }),
  setSetting: (key: string, value: string) => invoke<void>('app_set_setting', { key, value }),
};
