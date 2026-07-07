import { invoke } from '@tauri-apps/api/core';
import type { LlmConfig, CreateLlmConfigInput, UpdateLlmConfigInput } from '../types/settings';

export const llmConfigService = {
  list: () => invoke<LlmConfig[]>('llm_config_list'),
  create: (input: CreateLlmConfigInput) => invoke<LlmConfig>('llm_config_create', { input }),
  update: (id: string, input: UpdateLlmConfigInput) => invoke<LlmConfig>('llm_config_update', { id, input }),
  delete: (id: string) => invoke<void>('llm_config_delete', { id }),
  setDefault: (id: string) => invoke<void>('llm_config_set_default', { id }),
  testConnection: (id: string) => invoke<void>('llm_config_test_connection', { id }),
  listModels: (id: string) => invoke<string[]>('llm_config_list_models', { id }),
  listModelsByParams: (provider: string, baseUrl: string, apiKey: string) =>
    invoke<string[]>('llm_config_list_models_by_params', { provider, baseUrl, apiKey }),
};
