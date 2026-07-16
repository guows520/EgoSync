import { invoke } from '@tauri-apps/api/core';
import type { TaskDecompositionProposal } from '../types/taskDecomposition';

export const taskDecompositionService = {
  listPending: (conversationId: string) =>
    invoke<TaskDecompositionProposal[]>('task_decomposition_list_pending', { conversationId }),
  accept: (id: string) =>
    invoke<TaskDecompositionProposal>('task_decomposition_accept', { id }),
  keepSingle: (id: string) =>
    invoke<TaskDecompositionProposal>('task_decomposition_keep_single', { id }),
};
