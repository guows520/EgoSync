import { invoke } from '@tauri-apps/api/core';
import type { Suggestion, SuggestionWithRole } from '../types/suggestion';

export const suggestionService = {
  listPending: (conversationId: string) => invoke<SuggestionWithRole[]>('suggestion_list_pending', { conversationId }),
  confirm: (id: string) => invoke<Suggestion>('suggestion_confirm', { id }),
  reject: (id: string, reason: string) =>
    invoke<Suggestion>('suggestion_reject', { id, reason }),
};
