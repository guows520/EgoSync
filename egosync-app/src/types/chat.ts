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
  workingDirectory?: string;
  onboardingStep?: number;
  /** Story 10.1: 用户通过 `@Skill` 显式指定本轮任务使用的 统一 Skill key（registry/meta）。 */
  selectedSkillId?: string;
}

export interface MessageProcessEvent {
  id: string;
  conversationId: string;
  messageId: string;
  opencodeSessionId: string;
  eventType: string;
  toolName: string | null;
  status: string | null;
  summary: string;
  rawJson: string;
  workingDirectory: string | null;
  createdAt: string;
}

export interface ExecutionTraceDetail {
  label: string;
  value: string;
  tone?: 'normal' | 'error';
}

export type ExecutionTraceBlock =
  | {
      id: string;
      type: 'thinking';
      content: string;
      elapsedSeconds: number | null;
      isActive: boolean;
    }
  | {
      id: string;
      type: 'narration';
      content: string;
    }
  | {
      id: string;
      type: 'action';
      actionType: 'tool' | 'shell' | 'read' | 'edit' | 'write' | 'skill' | 'explore';
      title: string;
      status?: string | null;
      details?: ExecutionTraceDetail[];
      previewLines?: string[];
      detail?: string;
    };

export interface StreamPayload {
  conversationId: string;
  token: string;
  done: boolean;
  thinking: boolean;
  /** Story 2.3: 后端切换到新 assistant 气泡时携带其 id；普通单段流式不带。 */
  messageId?: string | null;
  phase?: 'thinking' | 'tool' | 'process' | 'answering' | 'done' | 'error';
  statusText?: string;
  toolName?: string;
  processEvent?: MessageProcessEvent;
}

export interface SourceNavigationTarget {
  conversationId: string;
  messageId: string;
  roleId: string | null;
}
