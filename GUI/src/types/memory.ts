export type MemoryCategory = 'preference' | 'task_status' | 'cognition_update' | 'fact';

export interface Memory {
  id: string;
  roleId: string | null;
  category: MemoryCategory;
  content: string;
  sourceConversationId: string;
  sourceMessageIds: string;
  createdAt: string;
}

export interface MemorySourceMessage {
  id: string;
  conversationId: string;
  role: 'user' | 'assistant' | string;
  content: string;
  createdAt: string;
  isSource: boolean;
}

export interface MemoryListOptions {
  category?: MemoryCategory;
  limit?: number;
  offset?: number;
}
