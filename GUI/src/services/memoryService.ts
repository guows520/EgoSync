import { invoke } from '@tauri-apps/api/core';
import type { Memory, MemoryListOptions, MemorySourceMessage } from '../types/memory';

interface MemoryCountOptions {
  roleId?: string | null;
  includeRoleMemories?: boolean;
  category?: MemoryListOptions['category'];
}

export const memoryService = {
  list: (roleId: string | null, options?: MemoryListOptions) =>
    invoke<Memory[]>('memory_list', {
      roleId,
      category: options?.category,
      limit: options?.limit,
      offset: options?.offset,
    }),
  listAll: (options?: MemoryListOptions) =>
    invoke<Memory[]>('memory_list_all', {
      category: options?.category,
      limit: options?.limit,
      offset: options?.offset,
    }),
  count: ({ roleId = null, includeRoleMemories = false, category }: MemoryCountOptions) =>
    invoke<number>('memory_count', { roleId, includeRoleMemories, category }),
  getSourceMessages: (memoryId: string) =>
    invoke<MemorySourceMessage[]>('memory_get_source_messages', { memoryId }),
  delete: (memoryId: string) =>
    invoke<void>('memory_delete', { memoryId }),
};
