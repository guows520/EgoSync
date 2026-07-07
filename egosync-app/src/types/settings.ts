export type LlmProviderType = 'openai_compatible' | 'anthropic' | 'minimax' | 'zhipu' | 'deepseek' | 'kimi' | 'bailian';

export interface LlmConfig {
  id: string;
  name: string;
  provider: LlmProviderType;
  baseUrl: string;
  model: string;
  apiKeyRef: string;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateLlmConfigInput {
  name: string;
  provider: LlmProviderType;
  baseUrl: string;
  model: string;
  apiKey: string;
}

export interface UpdateLlmConfigInput {
  name?: string;
  provider: LlmProviderType;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
}
