export interface SkillRoleScope {
  allRoles: boolean;
  roleIds: string[];
}

export interface SkillRegistryEntry {
  id: string;
  name: string;
  description: string;
  sourceType: 'custom' | 'opencode';
  managedPath: string;
  contentHash: string;
  createdAt: string;
  updatedAt: string;
}


export type SelectableSkillKey = `registry:${string}` | 'meta:find-skills' | 'meta:skill-creator';

export interface SelectableSkill {
  key: SelectableSkillKey;
  name: string;
  description: string;
  kind: 'registry' | 'meta';
  sourceType: 'custom' | 'opencode' | 'meta';
}

export interface SkillScopeUpdatedPayload {
  scopeKind: 'role' | 'butler' | 'all';
  ownerId: string | null;
}

export interface SkillDuplicateInfo {
  kind: 'contentHash' | 'name';
  existing: SkillRegistryEntry;
}

export interface OpencodeSkillCandidate {
  name: string;
  description: string;
  sourceLocation: string;
  sourcePath: string;
  sourceType: 'opencode';
  contentHash: string;
  alreadyImported: boolean;
  duplicate: SkillDuplicateInfo | null;
}

export interface OpencodeSkillSkippedSummary {
  total: number;
  reasons: string[];
}

export interface DiscoverOpencodeSkillsResult {
  items: OpencodeSkillCandidate[];
  skipped: OpencodeSkillSkippedSummary;
}

export interface ImportOpencodeSkillInput {
  sourcePath: string;
  roleScope?: SkillRoleScope;
  /** 发现时展示的 content_hash，导入时回传供后端做 TOCTOU 一致性校验。 */
  expectedContentHash?: string;
}

export interface ImportOpencodeSkillResult {
  status: 'imported' | 'duplicate';
  entry: SkillRegistryEntry | null;
  /** registry 写入后是否已成功同步到 opencode agent。false 时不应声称"已启用"。 */
  synced: boolean;
  runtimeReady: boolean;
  runtimeError: string | null;
}

export interface SkillImportPreview {
  name: string;
  description: string;
  contentHash: string;
  duplicate: SkillDuplicateInfo | null;
}

export interface PickCustomSkillDirectoryResult {
  content: string;
  sourcePath: string;
}

export interface PreviewCustomSkillInput {
  content?: string;
}

export interface ImportCustomSkillInput {
  content?: string;
  sourcePath?: string;
  overwriteExisting?: boolean;
  roleScope?: SkillRoleScope;
}

export interface ImportCustomSkillResult {
  status: 'imported' | 'duplicate';
  entry: SkillRegistryEntry | null;
  preview: SkillImportPreview;
  runtimeReady: boolean;
  runtimeError: string | null;
}
