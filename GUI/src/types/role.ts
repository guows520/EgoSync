export interface Role {
  id: string;
  name: string;
  icon: string;
  color: string;
  goal: string;
  personalityPrompt: string;
  status: 'active' | 'archived';
  energy: number;
  skillsConfig: string;
  proactivityLevel: 'passive' | 'moderate' | 'proactive';
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateRoleInput {
  name: string;
  icon?: string;
  color?: string;
  goal?: string;
}

export interface UpdateRoleInput {
  name?: string;
  icon?: string;
  color?: string;
  goal?: string;
}