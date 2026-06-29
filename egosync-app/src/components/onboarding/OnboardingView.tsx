import { useState, useEffect, useRef, useCallback } from 'react';
import { Home, Play } from 'lucide-react';
import { chatService } from '../../services/chatService';
import { appService } from '../../services/appService';
import { roleService } from '../../services/roleService';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { ChatBubble } from '../chat/ChatBubble';
import { RoleConfirmModal, type RoleProposal } from './RoleConfirmModal';
import type { ChatMessage, StreamPayload } from '../../types/chat';

interface RoleProposedPayload {
  conversationId: string;
  name: string;
  icon: string | null;
  color: string | null;
  goal: string | null;
}

interface OnboardingViewProps {
  onComplete: () => void;
  onOpenSettings: () => void;
}

export function OnboardingView({ onComplete, onOpenSettings }: OnboardingViewProps) {
  const [step, setStep] = useState(1);
  const [input, setInput] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamContent, setStreamContent] = useState('');
  const [thinkingContent, setThinkingContent] = useState('');
  const [streamStatus, setStreamStatus] = useState<Pick<StreamPayload, 'phase' | 'statusText' | 'toolName'> | null>(null);
  const [isThinking, setIsThinking] = useState(false);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [llmReady, setLlmReady] = useState<boolean | null>(null);
  const [proposal, setProposal] = useState<RoleProposal | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const thinkingContentRef = useRef('');
  const initRef = useRef(false);
  const proposalHandledRef = useRef(false);

  // Scroll to bottom on new messages
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamContent, thinkingContent]);

  // Listen for LLM stream events
  useTauriEvent<StreamPayload>('llm:stream', useCallback((payload: StreamPayload) => {
    if (conversationId && payload.conversationId !== conversationId) return;

    if (payload.phase === 'tool' && payload.statusText) {
      setIsStreaming(true);
      setStreamStatus({ phase: payload.phase, statusText: payload.statusText, toolName: payload.toolName });
    }

    if (payload.done) {
      const currentThinkingContent = thinkingContentRef.current;
      setIsStreaming(false);
      setIsThinking(false);
      setStreamStatus(null);
      setStreamContent(prev => {
        if (prev.trim() || currentThinkingContent.trim()) {
          const assistantMsg: ChatMessage = {
            id: `onboard-${Date.now()}`,
            conversationId: conversationId ?? '',
            role: 'assistant',
            content: prev.trim(),
            thinkingContent: currentThinkingContent,
            isComplete: true,
            createdAt: new Date().toISOString(),
            routingMetadata: null,
          };
          setMessages(msgs => [...msgs, assistantMsg]);
        }
        return '';
      });
      thinkingContentRef.current = '';
      setThinkingContent('');
    } else if (payload.thinking) {
      setIsStreaming(true);
      setIsThinking(true);
      const nextThinkingContent = thinkingContentRef.current + payload.token;
      thinkingContentRef.current = nextThinkingContent;
      setThinkingContent(nextThinkingContent);
    } else {
      if (payload.phase === 'tool' && !payload.token) return;
      if (isThinking) setIsThinking(false);
      setStreamStatus(null);
      setStreamContent(prev => prev + payload.token);
    }
  }, [conversationId, isThinking]), [conversationId]);

  // Listen for role:proposed event from backend tool execution
  // 后端 LLM 调用 create_role 工具时只发提议，不写库；前端展示 modal 让用户确认/编辑
  useTauriEvent<RoleProposedPayload>('role:proposed', useCallback((payload: RoleProposedPayload) => {
    if (proposalHandledRef.current) return; // 一次 onboarding 只处理一次
    console.info('收到角色提议:', payload.name, payload.icon, payload.color);
    setProposal({
      name: payload.name,
      icon: payload.icon,
      color: payload.color,
      goal: payload.goal,
    });
    setModalOpen(true);
  }, []), []);

  const handleProposalConfirm = useCallback(
    async (values: { name: string; icon: string; color: string; goal: string }) => {
      if (creating) return;
      setCreating(true);
      try {
        const role = await roleService.create({
          name: values.name,
          icon: values.icon,
          color: values.color,
          goal: values.goal || undefined,
        });
        console.info('角色创建成功:', role.name, role.id);
        proposalHandledRef.current = true;
        setModalOpen(false);
        setProposal(null);
        await appService.completeOnboarding();
        setTimeout(() => onComplete(), 800);
      } catch (e) {
        console.error('Failed to create role from proposal:', e);
        setCreating(false);
      }
    },
    [creating, onComplete],
  );

  const handleProposalCancel = useCallback(() => {
    if (creating) return;
    setModalOpen(false);
    setProposal(null);
    // 不标记 proposalHandledRef，允许 LLM 后续再次提议
  }, [creating]);

  // Initialize: check LLM config and start onboarding
  useEffect(() => {
    if (initRef.current) return;
    initRef.current = true;

    appService.isLlmConfigured().then(configured => {
      setLlmReady(configured);
      if (configured) {
        startOnboarding();
      }
    }).catch(() => {
      setLlmReady(false);
    });
  }, []);

  const startOnboarding = async () => {
    try {
      const conv = await chatService.newConversation();
      setConversationId(conv.id);
      setIsStreaming(true);
      await chatService.sendMessage({
        conversationId: conv.id,
        content: '__onboarding_start__',
        onboardingStep: 1,
      });
    } catch (e) {
      console.error('Failed to start onboarding:', e);
      const fallbackMsg: ChatMessage = {
        id: `onboard-fallback-${Date.now()}`,
        conversationId: '',
        role: 'assistant',
        content: '你好，我是你的数字管家。很高兴为你服务！聊聊你最近在忙什么？',
        thinkingContent: '',
        isComplete: true,
        createdAt: new Date().toISOString(),
        routingMetadata: null,
      };
      setMessages([fallbackMsg]);
    }
  };

  const handleSend = async () => {
    if (!input.trim() || isStreaming || !conversationId) return;
    const text = input.trim();
    setInput('');

    const userMsg: ChatMessage = {
      id: `onboard-user-${Date.now()}`,
      conversationId: conversationId,
      role: 'user',
      content: text,
      thinkingContent: '',
      isComplete: true,
      createdAt: new Date().toISOString(),
      routingMetadata: null,
    };
    setMessages(prev => [...prev, userMsg]);

    const nextStep = Math.min(step + 1, 5);
    setStep(nextStep);
    setIsStreaming(true);
    thinkingContentRef.current = '';
    setThinkingContent('');
    setStreamStatus(null);
    setIsThinking(false);

    try {
      await chatService.sendMessage({
        conversationId,
        content: text,
        onboardingStep: nextStep,
      });
    } catch (e) {
      console.error('Failed to send message:', e);
      setIsStreaming(false);
      const errorMsg: ChatMessage = {
        id: `onboard-err-${Date.now()}`,
        conversationId: conversationId,
        role: 'assistant',
        content: '抱歉，我现在无法回应，请检查一下模型配置。',
        thinkingContent: '',
        isComplete: true,
        createdAt: new Date().toISOString(),
        routingMetadata: null,
      };
      setMessages(prev => [...prev, errorMsg]);
    }
  };

  const handleConfigComplete = () => {
    setLlmReady(true);
    startOnboarding();
  };

  // Show settings prompt if LLM not configured
  if (llmReady === false) {
    return (
      <div className="h-full flex flex-col items-center justify-center bg-white/40 dark:bg-slate-900/40">
        <div className="max-w-md text-center space-y-6 p-8">
          <div className="w-16 h-16 rounded-2xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-sm">
            <Home size={32} strokeWidth={2} />
          </div>
          <h2 className="text-xl font-semibold text-slate-800 dark:text-slate-100">欢迎使用 EgoSync</h2>
          <p className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            在开始之前，需要先配置一个 AI 模型。点击下方按钮前往设置。
          </p>
          <button
            onClick={() => {
              onOpenSettings();
              // Poll for config completion
              const interval = setInterval(() => {
                appService.isLlmConfigured().then(configured => {
                  if (configured) {
                    clearInterval(interval);
                    handleConfigComplete();
                  }
                }).catch(() => {});
              }, 2000);
            }}
            className="px-6 py-3 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity"
          >
            配置 AI 模型
          </button>
        </div>
      </div>
    );
  }

  // Loading state
  if (llmReady === null) {
    return (
      <div className="h-full flex items-center justify-center bg-white/40 dark:bg-slate-900/40">
        <div className="text-slate-400 text-[14px]">正在准备...</div>
      </div>
    );
  }

  const placeholders = [
    '输入你的名字或昵称...',
    '比如：我是一个产品经理，最近在忙新产品上线...',
    '聊聊你最关注的方面...',
    '确认，或者告诉我你想调整...',
    '好的，创建吧！',
  ];

  return (
    <div className="h-full flex flex-col relative bg-white/40 dark:bg-slate-900/40">
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-8 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">数字管家</h2>
        </div>
      </header>

      {/* Chat Area */}
      <div className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="max-w-3xl mx-auto space-y-3">
          {messages
            .filter(m => m.role !== 'system')
            .map(msg => (
              <ChatBubble key={msg.id} message={msg} />
            ))}
          {/* Streaming bubble */}
          {isStreaming && (
            <ChatBubble
              message={{
                id: '__onboarding_streaming__',
                conversationId: conversationId ?? '',
                role: 'assistant',
                content: streamContent,
                thinkingContent: '',
                isComplete: false,
                createdAt: new Date().toISOString(),
                routingMetadata: null,
              }}
              isStreaming
              streamingThinking={thinkingContent || undefined}
              isThinkingPhase={isThinking}
              streamStatus={streamStatus}
            />
          )}
          <div ref={chatEndRef} />
        </div>
      </div>

      {/* Input Area */}
      <div className="p-5 bg-white/70 dark:bg-slate-800/70 backdrop-blur-md border-t border-slate-200/60 dark:border-slate-700/60 shrink-0">
        <div className="relative max-w-3xl mx-auto">
          <input
            type="text"
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSend()}
            disabled={isStreaming}
            placeholder={placeholders[Math.min(step, placeholders.length - 1)]}
            className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-500/20 transition-colors shadow-sm disabled:opacity-50"
          />
          <button
            onClick={handleSend}
            disabled={isStreaming || !input.trim()}
            className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700 disabled:opacity-40"
          >
            <Play size={16} className="ml-0.5" fill="currentColor" />
          </button>
        </div>
      </div>

      {/* Role proposal confirmation modal */}
      <RoleConfirmModal
        open={modalOpen}
        proposal={proposal}
        onConfirm={handleProposalConfirm}
        onCancel={handleProposalCancel}
        busy={creating}
      />
    </div>
  );
}