export interface TaskDecompositionItem {
  title: string;
  deadline: string | null;
}

export interface TaskDecompositionProposal {
  id: string;
  roleId: string;
  sourceConversationId: string;
  taskSummary: string;
  items: TaskDecompositionItem[];
  status: 'pending' | 'accepted' | 'kept_single';
  createdAt: string;
  resolvedAt: string | null;
  roleName: string;
  roleIcon: string;
  roleColor: string;
}
