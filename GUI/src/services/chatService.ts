import { invoke } from '@tauri-apps/api/core';
import type { Conversation, ChatMessage, ChatRequest } from '../types/chat';

export const chatService = {
  sendMessage: (request: ChatRequest) => invoke<ChatMessage>('chat_send_message', { request }),
  getHistory: (conversationId: string) => invoke<ChatMessage[]>('chat_get_history', { conversationId }),
  getButlerConversation: () => invoke<Conversation>('chat_get_butler_conversation'),
  getRoleConversation: (roleId: string) =>
    invoke<Conversation>('chat_get_role_conversation', { roleId }),
  listConversations: (roleId?: string) =>
    invoke<Conversation[]>('chat_list_conversations', { roleId: roleId ?? null }),
  newConversation: (oldConversationId?: string, roleId?: string) =>
    invoke<Conversation>('chat_new_conversation', {
      oldConversationId: oldConversationId ?? null,
      roleId: roleId ?? null,
    }),
  stopStreaming: (conversationId: string) =>
    invoke<void>('chat_stop_streaming', { conversationId }),
  deleteConversation: (conversationId: string) =>
    invoke<void>('chat_delete_conversation', { conversationId }),
};
