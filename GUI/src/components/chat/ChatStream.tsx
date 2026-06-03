import { useState, useEffect, useRef, useCallback } from 'react';
import { chatService } from '../../services/chatService';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { ChatBubble } from './ChatBubble';
import { ChatInput } from './ChatInput';
import { ChatHeader } from './ChatHeader';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import type { ChatMessage, StreamPayload, Conversation, TitleUpdatedPayload, SourceNavigationTarget } from '../../types/chat';
import type { Role } from '../../types/role';

interface ChatStreamProps {
  /** 角色视图传入对应 Role；管家/onboarding 视图传 null。 */
  role?: Role | null;
  onMemoryReferenceClick?: (memoryId: string) => void;
  sourceNavigationTarget?: SourceNavigationTarget | null;
  onSourceNavigationHandled?: () => void;
}

type StreamBubbleState = { id: string | null; content: string };
type StreamStatus = Pick<StreamPayload, 'phase' | 'statusText' | 'toolName'>;

function completedAssistantMessagesFromBubbles(
  bubbles: Array<StreamBubbleState & { generation?: number }>,
  conversationId: string,
  fallbackGeneration: number,
  thinkingContent: string,
): ChatMessage[] {
  const createdAt = new Date().toISOString();
  const hasThinking = thinkingContent.trim().length > 0;
  const completedMessages = bubbles
    .filter(b => b.content.trim().length > 0)
    .map((b, idx) => ({
      id: b.id ? `__completed__${b.id}` : `__completed__${b.generation ?? fallbackGeneration}_${idx}`,
      conversationId,
      role: 'assistant' as const,
      content: b.content,
      thinkingContent: idx === 0 && hasThinking ? thinkingContent : '',
      isComplete: true,
      createdAt,
      routingMetadata: null,
    }));

  if (completedMessages.length === 0 && hasThinking) {
    return [{
      id: `__completed__thinking__${fallbackGeneration}`,
      conversationId,
      role: 'assistant',
      content: '',
      thinkingContent,
      isComplete: true,
      createdAt,
      routingMetadata: null,
    }];
  }

  return completedMessages;
}

function isLikelyPersistedLocalUser(historyMsg: ChatMessage, localMsg: ChatMessage) {
  if (!localMsg.id.startsWith('__local_user__') || historyMsg.role !== 'user' || localMsg.content !== historyMsg.content) {
    return false;
  }

  const historyTime = Date.parse(historyMsg.createdAt);
  const localTime = Date.parse(localMsg.createdAt);
  return Number.isFinite(historyTime) && Number.isFinite(localTime) && historyTime >= localTime - 60_000;
}

function isLocalCompletedAssistant(message: ChatMessage) {
  return message.role === 'assistant' && message.id.startsWith('__completed__');
}

function hasPersistedCompletedAssistantId(historyMsg: ChatMessage, localMsg: ChatMessage) {
  return isLocalCompletedAssistant(localMsg)
    && historyMsg.role === 'assistant'
    && localMsg.id === `__completed__${historyMsg.id}`;
}

function previousUserContent(messages: ChatMessage[], index: number) {
  for (let i = index - 1; i >= 0; i -= 1) {
    if (messages[i].role === 'user') return messages[i].content;
  }
  return null;
}

function isLikelyPersistedCompletedAssistant(
  history: ChatMessage[],
  historyIndex: number,
  current: ChatMessage[],
  localMsg: ChatMessage,
) {
  const historyMsg = history[historyIndex];
  if (!localMsg.id.startsWith('__completed__') || historyMsg.role !== 'assistant') {
    return false;
  }

  if (hasPersistedCompletedAssistantId(historyMsg, localMsg)) {
    return true;
  }

  if (localMsg.content.trim() !== historyMsg.content.trim()) {
    return false;
  }

  const localIndex = current.findIndex(m => m.id === localMsg.id);
  if (localIndex === -1) return false;

  const historyUserContent = previousUserContent(history, historyIndex);
  const localUserContent = previousUserContent(current, localIndex);
  return historyUserContent !== null
    && localUserContent !== null
    && historyUserContent === localUserContent;
}

function appendLocalMessages(current: ChatMessage[], localMessages: ChatMessage[]) {
  const next = current.slice();
  for (const message of localMessages) {
    const candidateCurrent = [...next, message];
    const exists = next.some((existing, index) =>
      existing.id === message.id
      || (!existing.id.startsWith('__completed__')
        && isLikelyPersistedCompletedAssistant(next, index, candidateCurrent, message))
    );
    if (!exists) {
      next.push(message);
    }
  }
  return next;
}

function mergeHistoryWithLocalMessages(history: ChatMessage[], current: ChatMessage[]) {
  if (history.length === 0) return current;

  const remainingCurrent = current.slice();
  const mergedHistory = history.map((historyMsg, historyIndex) => {
    const sameIdIndex = remainingCurrent.findIndex(m => m.id === historyMsg.id);
    if (sameIdIndex !== -1) {
      remainingCurrent.splice(sameIdIndex, 1);
      return historyMsg;
    }

    const localUserIndex = remainingCurrent.findIndex(m => isLikelyPersistedLocalUser(historyMsg, m));
    if (localUserIndex !== -1) {
      remainingCurrent.splice(localUserIndex, 1);
      return historyMsg;
    }

    const completedAssistantIndex = remainingCurrent.findIndex(m =>
      isLikelyPersistedCompletedAssistant(history, historyIndex, current, m)
    );
    if (completedAssistantIndex !== -1) {
      remainingCurrent.splice(completedAssistantIndex, 1);
      return historyMsg;
    }

    const sameMessageIndex = remainingCurrent.findIndex(m =>
      m.role === historyMsg.role
      && m.content === historyMsg.content
      && m.createdAt === historyMsg.createdAt
    );
    if (sameMessageIndex !== -1) {
      remainingCurrent.splice(sameMessageIndex, 1);
    }

    return historyMsg;
  });

  return [...mergedHistory, ...remainingCurrent];
}

function centerMessageInScrollContainer(container: HTMLDivElement | null, target: HTMLDivElement) {
  target.scrollIntoView({ behavior: 'smooth', block: 'center' });
  if (!container) return;
  const top = target.offsetTop - (container.clientHeight / 2) + (target.clientHeight / 2);
  container.scrollTop = Math.max(0, top);
}

export function ChatStream({
  role,
  onMemoryReferenceClick,
  sourceNavigationTarget,
  onSourceNavigationHandled,
}: ChatStreamProps) {
  const roleId = role?.id ?? null;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isInputLocked, setIsInputLocked] = useState(false);
  // Story 2.3 P1b: 流式期间可能产生多个气泡（管家委派两段式：先「稍等，我让 X 看一下」，
  // 后「来自 X 的反馈…」）。按后端 messageId 分桶，messageId 缺省（普通单段）落到 id=null 桶。
  const [streamBubbles, setStreamBubbles] = useState<StreamBubbleState[]>([]);
  const streamBubblesRef = useRef<StreamBubbleState[]>([]);
  const streamGenerationRef = useRef(0);
  const conversationLoadGenerationRef = useRef(0);
  const localMessageSequenceRef = useRef(0);
  const conversationIdRef = useRef<string | null>(null);
  const [thinkingContent, setThinkingContent] = useState('');
  const thinkingContentRef = useRef('');
  const [streamStatus, setStreamStatus] = useState<StreamStatus | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const messageRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const highlightTimeout = useRef<number | null>(null);
  const sourceScrollSuppressTimeout = useRef<number | null>(null);
  const suppressAutoScrollRef = useRef(false);
  const [highlightedMessageId, setHighlightedMessageId] = useState<string | null>(null);
  const [sourceNavigationNotice, setSourceNavigationNotice] = useState<string | null>(null);
  const [pendingScrollMessageId, setPendingScrollMessageId] = useState<string | null>(null);

  const updateStreamBubbles = useCallback((updater: (prev: StreamBubbleState[]) => StreamBubbleState[]) => {
    const next = updater(streamBubblesRef.current);
    streamBubblesRef.current = next;
    setStreamBubbles(next);
  }, []);

  useEffect(() => {
    return () => {
      if (highlightTimeout.current !== null) {
        window.clearTimeout(highlightTimeout.current);
      }
      if (sourceScrollSuppressTimeout.current !== null) {
        window.clearTimeout(sourceScrollSuppressTimeout.current);
      }
    };
  }, []);

  useEffect(() => {
    conversationIdRef.current = conversation?.id ?? null;
  }, [conversation?.id]);

  const resetStreamingState = useCallback(() => {
    streamGenerationRef.current += 1;
    updateStreamBubbles(() => []);
    setIsStreaming(false);
    setIsInputLocked(false);
    thinkingContentRef.current = '';
    setThinkingContent('');
    setStreamStatus(null);
  }, [updateStreamBubbles]);

  const loadConversations = useCallback(async () => {
    try {
      const list = await chatService.listConversations(roleId ?? undefined);
      setConversations(list);
    } catch (e) {
      console.error('加载对话列表失败:', e);
    }
  }, [roleId]);

  const switchToConversation = useCallback(async (conv: Conversation) => {
    const loadGeneration = ++conversationLoadGenerationRef.current;
    conversationIdRef.current = conv.id;
    setConversation(conv);
    setMessages([]);
    resetStreamingState();
    try {
      const history = await chatService.getHistory(conv.id);
      if (conversationIdRef.current !== conv.id || conversationLoadGenerationRef.current !== loadGeneration) return;
      setMessages(history);
    } catch (e) {
      console.error('加载对话历史失败:', e);
    }
  }, [resetStreamingState]);

  useEffect(() => {
    const init = async () => {
      const loadGeneration = ++conversationLoadGenerationRef.current;
      // 切换 role/butler 时立即清空当前消息，避免上一会话的内容短暂泄漏到新视图
      setConversation(null);
      conversationIdRef.current = null;
      setMessages([]);
      resetStreamingState();
        try {
        const conv = roleId
          ? await chatService.getRoleConversation(roleId)
          : await chatService.getButlerConversation();
        if (conversationLoadGenerationRef.current !== loadGeneration) return;
        conversationIdRef.current = conv.id;
        setConversation(conv);
        const history = await chatService.getHistory(conv.id);
        if (conversationIdRef.current !== conv.id || conversationLoadGenerationRef.current !== loadGeneration) return;
        setMessages(prev => mergeHistoryWithLocalMessages(history, prev));
        await loadConversations();
      } catch (e) {
        console.error('加载对话失败:', e);
      }
    };
    init();
  }, [loadConversations, resetStreamingState, roleId]);

  useEffect(() => {
    if (!sourceNavigationTarget) return;
    const loadGeneration = ++conversationLoadGenerationRef.current;
    const target = sourceNavigationTarget;

    async function navigateToSource() {
      try {
        const conv = await chatService.getConversation(target.conversationId);
        if (conversationLoadGenerationRef.current !== loadGeneration) return;
        if (!conv) {
          setSourceNavigationNotice('来源信息已删除，无法跳转');
          onSourceNavigationHandled?.();
          return;
        }

        if ((roleId ?? null) !== (conv.roleId ?? null)) {
          setSourceNavigationNotice('来源对话不属于当前视图，无法在这里打开');
          onSourceNavigationHandled?.();
          return;
        }

        const history = await chatService.getHistory(conv.id);
        if (conversationLoadGenerationRef.current !== loadGeneration) return;
        const exists = history.some(message => message.id === target.messageId);
        if (!exists) {
          setSourceNavigationNotice('来源信息已删除，无法跳转');
          onSourceNavigationHandled?.();
          return;
        }

        conversationIdRef.current = conv.id;
        setConversation(conv);
        setMessages(history);
        updateStreamBubbles(() => []);
        setIsStreaming(false);
        setIsInputLocked(false);
        thinkingContentRef.current = '';
        setThinkingContent('');
        setSourceNavigationNotice(null);
        setHighlightedMessageId(target.messageId);
        setPendingScrollMessageId(target.messageId);
        if (highlightTimeout.current !== null) {
          window.clearTimeout(highlightTimeout.current);
        }
        highlightTimeout.current = window.setTimeout(() => {
          setHighlightedMessageId(current => (current === target.messageId ? null : current));
          highlightTimeout.current = null;
        }, 3000);
        await loadConversations();
      } catch (e) {
        console.error('跳转来源对话失败:', e);
        if (conversationLoadGenerationRef.current !== loadGeneration) return;
        setSourceNavigationNotice('来源对话暂时无法打开');
        onSourceNavigationHandled?.();
      }
    }

    navigateToSource();
  }, [loadConversations, onSourceNavigationHandled, resetStreamingState, roleId, sourceNavigationTarget]);

  useEffect(() => {
    if (!pendingScrollMessageId) return;
    const target = messageRefs.current[pendingScrollMessageId];
    if (!target) return;
    suppressAutoScrollRef.current = true;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        centerMessageInScrollContainer(scrollRef.current, target);
        setPendingScrollMessageId(null);
        onSourceNavigationHandled?.();
        if (sourceScrollSuppressTimeout.current !== null) {
          window.clearTimeout(sourceScrollSuppressTimeout.current);
        }
        sourceScrollSuppressTimeout.current = window.setTimeout(() => {
          suppressAutoScrollRef.current = false;
          sourceScrollSuppressTimeout.current = null;
        }, 1200);
      });
    });
  }, [messages, onSourceNavigationHandled, pendingScrollMessageId]);

  useEffect(() => {
    if (pendingScrollMessageId || suppressAutoScrollRef.current) return;
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, pendingScrollMessageId, streamBubbles, streamStatus, thinkingContent]);

  const handleStreamEvent = useCallback((payload: StreamPayload) => {
    if (conversation && payload.conversationId !== conversation.id) return;
    if (!conversation) return;

    const bucketKey = payload.messageId ?? null;

    if (payload.phase === 'tool' && payload.statusText) {
      setIsStreaming(true);
      setIsInputLocked(true);
      setStreamStatus({ phase: payload.phase, statusText: payload.statusText, toolName: payload.toolName });
    }

    if (payload.done) {
      // 委派路径下 done 会发两次：第一段（带 messageId）与最终段（带 followup messageId）。
      // 中间 done（有 messageId 且桶里仍有其他活跃气泡）仅刷新历史，不结束 streaming——
      // 否则后续 follow-up token 因 isStreaming=false 而不渲染。
      const currentThinkingContent = thinkingContentRef.current;
      const currentBubbles = streamBubblesRef.current;
      const doneMatchesActiveBucket = currentBubbles.some(b => b.id === bucketKey);
      const isDelegationSegmentDone = bucketKey !== null
        && currentBubbles.some(b => b.id === null)
        && !doneMatchesActiveBucket;
      const remainingAfterDone = currentBubbles.filter(b => b.id !== bucketKey && b.id !== null);
      const isFinalDone = !isDelegationSegmentDone && remainingAfterDone.length === 0;
      const streamGeneration = streamGenerationRef.current;
      if (isFinalDone) {
        setIsInputLocked(false);
        setStreamStatus(null);
        thinkingContentRef.current = '';
        setThinkingContent('');
        const completedMessages = completedAssistantMessagesFromBubbles(
          currentBubbles,
          conversation.id,
          streamGeneration,
          currentThinkingContent,
        );
        if (completedMessages.length > 0) {
          setMessages(prev => appendLocalMessages(prev, completedMessages));
        }
        updateStreamBubbles(() => []);
        setIsStreaming(false);
      }
      const applyBucketClear = () => {
        if (isDelegationSegmentDone || isFinalDone) return;
        if (streamGenerationRef.current !== streamGeneration) return;
        updateStreamBubbles(prev => {
          const remaining = prev.filter(b => b.id !== bucketKey && b.id !== null);
          if (remaining.length === 0) {
            setIsStreaming(false);
          }
          return remaining;
        });
      };
      if (conversation) {
        const conversationId = conversation.id;
        chatService.getHistory(conversationId).then(history => {
          if (conversationIdRef.current !== conversationId) return;
          if (isDelegationSegmentDone) {
            if (streamGenerationRef.current !== streamGeneration || !streamBubblesRef.current.some(b => b.id === null)) return;
            setMessages(prev => mergeHistoryWithLocalMessages(history, prev));
            const persistedContents = new Set(
              history
                .filter(m => m.role === 'assistant')
                .map(m => m.content.trim())
                .filter(Boolean),
            );
            updateStreamBubbles(prev => prev.filter(b => b.id !== null || !persistedContents.has(b.content.trim())));
            setIsStreaming(true);
            return;
          }
          setMessages(prev => mergeHistoryWithLocalMessages(history, prev));
          applyBucketClear();
        }).catch(error => {
          console.error(error);
          if (conversationIdRef.current !== conversationId) return;
          applyBucketClear();
        });
      } else {
        applyBucketClear();
      }
    } else if (payload.thinking) {
      setIsStreaming(true);
      setIsInputLocked(true);
      const nextThinkingContent = thinkingContentRef.current + payload.token;
      thinkingContentRef.current = nextThinkingContent;
      setThinkingContent(nextThinkingContent);
    } else {
      if (payload.phase === 'tool' && !payload.token) return;
      setStreamStatus(null);
      setIsStreaming(true);
      setIsInputLocked(true);
      updateStreamBubbles(prev => {
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
      conversationIdRef.current = newConv.id;
      setConversation(newConv);
      setMessages([]);
      resetStreamingState();
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
    setIsInputLocked(true);
    streamGenerationRef.current += 1;
    const sendGeneration = streamGenerationRef.current;
    const localUserMessageId = `__local_user__${sendGeneration}_${localMessageSequenceRef.current++}`;
    setMessages(prev => [
      ...prev.filter(m => !isLocalCompletedAssistant(m)),
      {
        id: localUserMessageId,
        conversationId: conversation.id,
        role: 'user',
        content,
        thinkingContent: '',
        isComplete: true,
        createdAt: new Date().toISOString(),
        routingMetadata: null,
      },
    ]);
    updateStreamBubbles(() => []);
    thinkingContentRef.current = '';
    setThinkingContent('');
    setStreamStatus(null);

    try {
      const userMsg = await chatService.sendMessage({
        conversationId: conversation.id,
        roleId: roleId ?? undefined,
        content,
      });

      if (userMsg.role === 'assistant') {
        if (streamGenerationRef.current !== sendGeneration) return;
        setMessages(prev => [...prev.filter(m => m.id !== localUserMessageId), userMsg]);
        setIsStreaming(false);
        setIsInputLocked(false);
        return;
      }

      setMessages(prev => prev.map(m => m.id === localUserMessageId ? userMsg : m));
    } catch (e) {
      console.error('发送消息失败:', e);
      setMessages(prev => prev.filter(m => m.id !== localUserMessageId));
      if (streamGenerationRef.current === sendGeneration) {
        setIsStreaming(false);
        setIsInputLocked(false);
      }
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
      <div ref={scrollRef} data-testid="chat-scroll-container" className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="mx-auto max-w-3xl space-y-3">
          {sourceNavigationNotice && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-[13px] text-amber-700">
              {sourceNavigationNotice}
            </div>
          )}
          {messages
            .filter(m =>
              m.role !== 'system'
              && (m.role !== 'assistant' || m.content.trim().length > 0 || m.thinkingContent.trim().length > 0)
              && Boolean(m.isComplete || m.content || m.thinkingContent)
            )
            .map(msg => (
              <div
                key={msg.id}
                data-testid={`chat-message-${msg.id}`}
                ref={node => {
                  messageRefs.current[msg.id] = node;
                }}
                className={highlightedMessageId === msg.id ? 'rounded-xl ring-2 ring-indigo-400 ring-offset-4 ring-offset-white' : undefined}
              >
                <ChatBubble
                  message={msg}
                  assistantName={role?.name}
                  assistantIcon={assistantIcon}
                  assistantColor={assistantColor}
                  onMemoryReferenceClick={onMemoryReferenceClick}
                />
              </div>
            ))}
          {streamingMessages.map((m, idx) => (
            <ChatBubble
              key={m.id}
              message={m}
              isStreaming
              streamingThinking={idx === 0 && thinkingContent ? thinkingContent : undefined}
              isThinkingPhase={idx === 0 && thinkingContent.length > 0 && streamBubbles.length === 0}
              streamStatus={idx === 0 ? streamStatus : null}
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
            isStreaming={isInputLocked}
            disabled={isInputLocked}
            useRoleAccent={Boolean(role)}
            placeholder={role ? `跟 ${role.name} 说点什么...` : undefined}
          />
        </div>
      </div>
    </div>
  );
}
