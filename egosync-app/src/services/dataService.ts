import { invoke } from '@tauri-apps/api/core';

export type ExportFormat = 'sqlite' | 'json' | 'markdown';

export interface ExportResult {
  files: string[];
  sqlitePath: string | null;
  jsonPath: string | null;
  markdownPath: string | null;
}

export const dataService = {
  dataExport: (formats: ExportFormat[]) =>
    invoke<ExportResult>('data_export', { formats }),
  dataDestroy: () => invoke<void>('data_destroy'),
};
