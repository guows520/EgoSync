import { invoke } from '@tauri-apps/api/core';

export const appService = {
  isFirstLaunch: () => invoke<boolean>('app_is_first_launch'),
  completeOnboarding: () => invoke<void>('app_complete_onboarding'),
  isLlmConfigured: () => invoke<boolean>('app_is_llm_configured'),
};