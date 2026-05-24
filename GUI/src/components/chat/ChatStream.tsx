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
  // Story 2.3 P1b: 流式期间可能产生多个气泡（管家委派两段式：先「稍等，我让 X 看一下」，
  // 后「来自 X 的反馈…」）。按后端 messageId 分桶，messageId 缺省（普通单段）落到 id=null 桶。
  const [streamBubbles, setStreamBubbles] = useState<{ id: string | null; content: string }[]>([]);
  const [thinkingContent, setThinkingContent] = useState('');
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
    setStreamBubbles([]);
    setThinkingContent('');
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
      setStreamBubbles([]);
      setThinkingContent('');
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
  }, [messages, streamBubbles, thinkingContent]);

  const handleStreamEvent = useCallback((payload: StreamPayload) => {
    if (conversation && payload.conversationId !== conversation.id) return;

    const bucketKey = payload.messageId ?? null;

    if (payload.done) {
      // 委派路径下 done 会发两次：第一段（带 messageId）与最终段（带 followup messageId）。
      // 中间 done（有 messageId 且桶里仍有其他活跃气泡）仅刷新历史，不结束 streaming——
      // 否则后续 follow-up token 因 isStreaming=false 而不渲染。
        setThinkingContent('');
      // 清桶逻辑封装——先算出 remaining，但延迟到 getHistory 返回后执行，
      // 避免清桶→历史未到位之间的渲染间隙导致气泡闪烁。
      const applyBucketClear = () => {
        setStreamBubbles(prev => {
          const remaining = prev.filter(b => b.id !== bucketKey && b.id !== null);
          if (remaining.length === 0) {
            setIsStreaming(false);
          }
          return remaining;
        });
      };
      if (conversation) {
        chatService.getHistory(conversation.id).then(history => {
          setMessages(history);
          applyBucketClear();
        }).catch(console.error);
      } else {
        applyBucketClear();
      }
    } else if (payload.thinking) {
      setThinkingContent(prev => prev + payload.token);
    } else {
      setIsStreaming(true);
      setStreamBubbles(prev => {
        const idx = prev.findIndex(b => b.id === bucketKey);
        if (idx === -1) {
          // 新气泡（包括 messageId=null 默认桶第一次出现、或后端发空 token 唤醒第二个气泡）
          return [...prev, { id: bucketKey, content: payload.token }];
        }
        const next = prev.slice();
        next[idx] = { ...next[idx], content: next[idx].content + payload.token };
        return next;
      });
    }
  }, [conversation]);

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
      setStreamBubbles([]);
      setThinkingContent('');
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
    setStreamBubbles([]);
    setThinkingContent('');

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

  // Story 2.3 P1b: 流式期间渲染所有活跃气泡。空 content + isStreaming 时 ChatBubble 自动显示弹跳点。
  // thinking 仅挂在首个气泡上（与改动前单气泡时同等语义），避免在每段都重复 thinking 标签。
  const streamingMessages: ChatMessage[] = isStreaming
    ? streamBubbles.map((b) => ({
        id: b.id ? `__streaming__${b.id}` : `__streaming__first__`,
        conversationId: conversation?.id ?? '',
        role: 'assistant',
        content: b.content,
        thinkingContent: '',
        isComplete: false,
        createdAt: new Date().toISOString(),
        routingMetadata: null,
      }))
    : [];
  // 边界情况：isStreaming=true 但还没收到任何 token（首气泡尚未建立）也要显示弹跳点占位。
  if (isStreaming && streamingMessages.length === 0) {
    streamingMessages.push({
      id: '__streaming__first__',
      conversationId: conversation?.id ?? '',
      role: 'assistant',
      content: '',
      thinkingContent: '',
      isComplete: false,
      createdAt: new Date().toISOString(),
      routingMetadata: null,
    });
  }

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
            .filter(m => m.role !== 'system' && (m.isComplete || m.content))
            .map(msg => (
              <ChatBubble
                key={msg.id}
                message={msg}
                assistantName={role?.name}
                assistantIcon={assistantIcon}
                assistantColor={assistantColor}
              />
            ))}
          {streamingMessages.map((m, idx) => (
            <ChatBubble
              key={m.id}
              message={m}
              isStreaming
              streamingThinking={idx === 0 && thinkingContent ? thinkingContent : undefined}
              isThinkingPhase={idx === 0 && thinkingContent.length > 0 && streamBubbles.length === 0}
              assistantName={role?.name}
              assistantIcon={assistantIcon}
              assistantColor={assistantColor}
            />
          ))}
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
