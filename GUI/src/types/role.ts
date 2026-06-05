export type ProactivityLevel = 'passive' | 'moderate' | 'proactive';

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
  proactivityLevel: ProactivityLevel;
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
  personalityPrompt?: string;
}

export interface UpdateRoleSkillsInput {
  findSkills: boolean;
  skillCreator: boolean;
  enabledSkillIds?: string[];
}

export interface RoleSkillsConfig {
  findSkills: boolean;
  skillCreator: boolean;
  enabledSkillIds: string[];
}

export interface ButlerSkillsConfig {
  findSkills: boolean;
  skillCreator: boolean;
  enabledSkillIds: string[];
}

export interface UpdateRoleProactivityInput {
  proactivityLevel: ProactivityLevel;
}
