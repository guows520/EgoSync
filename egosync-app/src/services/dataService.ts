import { invoke } from '@tauri-apps/api/core';

export type ExportFormat = 'sqlite' | 'json' | 'markdown';

export interface ExportResult {
  files: string[];
  sqlitePath: string | null;
  jsonPath: string | null;
  markdownPath: string | null;
}

export interface ImportResult {
  rolesCount: number;
  tasksCount: number;
  memoriesCount: number;
  conversationsCount: number;
  messagesCount: number;
}

export const dataService = {
  dataExport: (formats: ExportFormat[]) =>
    invoke<ExportResult>('data_export', { formats }),
  dataDestroy: () => invoke<void>('data_destroy'),
  pickImportFile: () => invoke<string | null>('pick_import_file'),
  dataImport: (filePath: string) =>
    invoke<ImportResult>('data_import', { filePath }),
};
