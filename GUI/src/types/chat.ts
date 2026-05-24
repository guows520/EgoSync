export interface Conversation {
  id: string;
  roleId: string | null;
  title: string;
  startedAt: string;
  updatedAt: string;
}

export interface TitleUpdatedPayload {
  conversationId: string;
  title: string;
}

export interface ChatMessage {
  id: string;
  conversationId: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thinkingContent: string;
  isComplete: boolean;
  createdAt: string;
  /** Story 2.3: 管家委派审计 JSON 字符串；非委派消息为 null。 */
  routingMetadata: string | null;
}

export interface ChatRequest {
  conversationId?: string;
  roleId?: string;
  content: string;
  onboardingStep?: number;
}

export interface StreamPayload {
  conversationId: string;
  token: string;
  done: boolean;
  thinking: boolean;
  /** Story 2.3: 后端切换到新 assistant 气泡时携带其 id；普通单段流式不带。 */
  messageId?: string | null;
}
