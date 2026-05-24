import { useState, useEffect, useRef, useCallback } from 'react';
import { chatService } from '../../services/chatService';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { ChatBubble } from './ChatBubble';
import { ChatInput } from './ChatInput';
import { ChatHeader } from './ChatHeader';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import type { ChatMessage, StreamPayload, Conversation, TitleUpdatedPayload } from '../../types/chat';
import type { Role } from '../../types/role';

interface ChatStreamProps {
  /** 角色视图传入对应 Role；管家/onboarding 视图传 null。 */
  role?: Role | null;
}

export function ChatStream({ role }: ChatStreamProps) {
  const roleId = role?.id ?? null;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamContent, setStreamContent] = useState('');
  const [thinkingContent, setThinkingContent] = useState('');
  const [isThinking, setIsThinking] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const loadConversations = useCallback(async () => {
    try {
      const list = await chatService.listConversations(roleId ?? undefined);
      setConversations(list);
    } catch (e) {
      console.error('加载对话列表失败:', e);
    }
  }, [roleId]);

  const switchToConversation = useCallback(async (conv: Conversation) => {
    setConversation(conv);
    setMessages([]);
    setStreamContent('');
    setThinkingContent('');
    setIsThinking(false);
    try {
      const history = await chatService.getHistory(conv.id);
      setMessages(history);
    } catch (e) {
      console.error('加载对话历史失败:', e);
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      // 切换 role/butler 时立即清空当前消息，避免上一会话的内容短暂泄漏到新视图
      setConversation(null);
      setMessages([]);
      setStreamContent('');
      setThinkingContent('');
      setIsThinking(false);
      try {
        const conv = roleId
          ? await chatService.getRoleConversation(roleId)
          : await chatService.getButlerConversation();
        setConversation(conv);
        const history = await chatService.getHistory(conv.id);
        setMessages(history);
        await loadConversations();
      } catch (e) {
        console.error('加载对话失败:', e);
      }
    };
    init();
  }, [loadConversations, roleId]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, streamContent, thinkingContent]);

  const handleStreamEvent = useCallback((payload: StreamPayload) => {
    if (conversation && payload.conversationId !== conversation.id) return;

    if (payload.done) {
      setIsStreaming(false);
      setIsThinking(false);
      setStreamContent('');
      setThinkingContent('');
      if (conversation) {
        chatService.getHistory(conversation.id).then(setMessages).catch(console.error);
      }
    } else if (payload.thinking) {
      setIsThinking(true);
      setThinkingContent(prev => prev + payload.token);
    } else {
      if (isThinking) setIsThinking(false);
      setStreamContent(prev => prev + payload.token);
    }
  }, [conversation, isThinking]);

  useTauriEvent<StreamPayload>('llm:stream', handleStreamEvent, [conversation?.id]);

  const handleTitleUpdated = useCallback((payload: TitleUpdatedPayload) => {
    setConversations(prev =>
      prev.map(c => c.id === payload.conversationId ? { ...c, title: payload.title } : c)
    );
  }, []);

  useTauriEvent<TitleUpdatedPayload>('conversation:title-updated', handleTitleUpdated, []);

  const handleDeleteConversation = async (convId: string) => {
    if (isStreaming) return;
    try {
      await chatService.deleteConversation(convId);
      if (conversation?.id === convId) {
        const newConv = roleId
          ? await chatService.getRoleConversation(roleId)
          : await chatService.getButlerConversation();
        setConversation(newConv);
        const history = await chatService.getHistory(newConv.id);
        setMessages(history);
      }
      await loadConversations();
    } catch (e) {
      console.error('删除对话失败:', e);
    }
  };

  const handleNewConversation = async () => {
    if (isStreaming) return;
    try {
      const newConv = await chatService.newConversation(conversation?.id, roleId ?? undefined);
      setConversation(newConv);
      setMessages([]);
      setStreamContent('');
      setThinkingContent('');
      setIsThinking(false);
      await loadConversations();
    } catch (e) {
      console.error('创建新对话失败:', e);
    }
  };

  const handleStop = async () => {
    if (!conversation || !isStreaming) return;
    try {
      await chatService.stopStreaming(conversation.id);
    } catch (e) {
      console.error('停止流式回复失败:', e);
    }
  };

  const handleSend = async (content: string) => {
    if (!conversation) return;

    setIsStreaming(true);
    setStreamContent('');
    setThinkingContent('');
    setIsThinking(false);

    try {
      const userMsg = await chatService.sendMessage({
        conversationId: conversation.id,
        roleId: roleId ?? undefined,
        content,
      });

      if (userMsg.role === 'assistant') {
        setMessages(prev => [...prev, userMsg]);
        setIsStreaming(false);
        return;
      }

      setMessages(prev => [...prev, userMsg]);
    } catch (e) {
      console.error('发送消息失败:', e);
      setIsStreaming(false);
    }
  };

  // 角色视图：助手气泡显示角色名/图标/色温；管家视图保留默认 (Home + 管家)。
  const assistantIcon = role ? getRoleIconComponent(role.icon) : undefined;
  const assistantColor = role ? normalizeColorHex(role.color) : undefined;

  const streamingMessage: ChatMessage | null = isStreaming
    ? {
        id: '__streaming__',
        conversationId: conversation?.id ?? '',
        role: 'assistant',
        content: streamContent,
        thinkingContent: '',
        isComplete: false,
        createdAt: new Date().toISOString(),
        routingMetadata: null,
      }
    : null;

  return (
    <div className="flex flex-col h-full">
      <ChatHeader
        conversations={conversations}
        currentConversationId={conversation?.id ?? ''}
        onNewConversation={handleNewConversation}
        onSelectConversation={switchToConversation}
        onDeleteConversation={handleDeleteConversation}
      />
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="mx-auto max-w-3xl space-y-3">
          {messages
            .filter(m => m.role !== 'system')
            .map(msg => (
              <ChatBubble
                key={msg.id}
                message={msg}
                assistantName={role?.name}
                assistantIcon={assistantIcon}
                assistantColor={assistantColor}
              />
            ))}
          {streamingMessage && (
            <ChatBubble
              message={streamingMessage}
              isStreaming
              streamingThinking={thinkingContent || undefined}
              isThinkingPhase={isThinking}
              assistantName={role?.name}
              assistantIcon={assistantIcon}
              assistantColor={assistantColor}
            />
          )}
        </div>
      </div>

      <div className="p-5 bg-white/70 dark:bg-slate-800/70 backdrop-blur-md border-t border-slate-200/60 dark:border-slate-700/60 shrink-0">
        <div className="mx-auto max-w-3xl">
          <ChatInput
            onSend={handleSend}
            onStop={handleStop}
            isStreaming={isStreaming}
            disabled={isStreaming}
            useRoleAccent={Boolean(role)}
            placeholder={role ? `跟 ${role.name} 说点什么...` : undefined}
          />
        </div>
      </div>
    </div>
  );
}
