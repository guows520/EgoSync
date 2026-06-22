export interface Suggestion {
  id: string;
  roleId: string;
  title: string;
  content: string;
  priority: 'high' | 'medium' | 'low';
  status: 'pending' | 'confirmed' | 'rejected';
  rejectionReason: string | null;
  convertedTaskId: string | null;
  createdAt: string;
}

export interface SuggestionWithRole extends Suggestion {
  roleName: string;
  roleIcon: string;
  roleColor: string;
}
