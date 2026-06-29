export interface LlmConfig {
  id: string;
  name: string;
  provider: 'openai_compatible' | 'anthropic' | 'minimax';
  baseUrl: string;
  model: string;
  apiKeyRef: string;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateLlmConfigInput {
  name: string;
  provider: 'openai_compatible' | 'anthropic' | 'minimax';
  baseUrl: string;
  model: string;
  apiKey: string;
}

export interface UpdateLlmConfigInput {
  name?: string;
  provider: 'openai_compatible' | 'anthropic' | 'minimax';
  baseUrl?: string;
  model?: string;
  apiKey?: string;
}
