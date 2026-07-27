export type LlmProviderType = 'openai_compatible' | 'anthropic' | 'minimax' | 'zhipu' | 'deepseek' | 'kimi' | 'bailian';

export type NetworkLocation = 'internal' | 'external';

export interface LlmConfig {
  id: string;
  name: string;
  provider: LlmProviderType;
  baseUrl: string;
  model: string;
  apiKeyRef: string;
  isDefault: boolean;
  networkLocation: NetworkLocation;
  createdAt: string;
  updatedAt: string;
}

export interface CreateLlmConfigInput {
  name: string;
  provider: LlmProviderType;
  baseUrl: string;
  model: string;
  apiKey: string;
  networkLocation: NetworkLocation;
}

export interface UpdateLlmConfigInput {
  name?: string;
  provider: LlmProviderType;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
  networkLocation?: NetworkLocation;
}
