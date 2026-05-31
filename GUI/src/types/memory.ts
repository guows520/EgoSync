export interface Memory {
  id: string;
  roleId: string | null;
  category: 'preference' | 'task_status' | 'cognition_update' | 'fact';
  content: string;
  sourceConversationId: string;
  sourceMessageIds: string;
  createdAt: string;
}
