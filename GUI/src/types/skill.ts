export interface SkillRoleScope {
  allRoles: boolean;
  roleIds: string[];
}

export interface SkillRegistryEntry {
  id: string;
  name: string;
  description: string;
  sourceType: 'custom';
  managedPath: string;
  contentHash: string;
  createdAt: string;
  updatedAt: string;
}

export interface SkillDuplicateInfo {
  kind: 'contentHash' | 'name';
  existing: SkillRegistryEntry;
}

export interface SkillImportPreview {
  name: string;
  description: string;
  contentHash: string;
  duplicate: SkillDuplicateInfo | null;
}

export interface PickCustomSkillDirectoryResult {
  content: string;
}

export interface PreviewCustomSkillInput {
  content?: string;
}

export interface ImportCustomSkillInput {
  content?: string;
  overwriteExisting?: boolean;
  roleScope?: SkillRoleScope;
}

export interface ImportCustomSkillResult {
  status: 'imported' | 'duplicate';
  entry: SkillRegistryEntry | null;
  preview: SkillImportPreview;
}
