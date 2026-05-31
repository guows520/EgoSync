import { invoke } from '@tauri-apps/api/core';
import type { Memory } from '../types/memory';

export const memoryService = {
  list: (roleId: string | null) => invoke<Memory[]>('memory_list', { roleId }),
  listAll: () => invoke<Memory[]>('memory_list_all'),
};
