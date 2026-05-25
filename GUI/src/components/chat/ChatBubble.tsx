import { useState, memo } from 'react';
import ReactMarkdown from 'react-markdown';
import { cn } from '../../lib/utils';
import { Home, ChevronRight, type LucideIcon } from 'lucide-react';
import type { ChatMessage } from '../../types/chat';

const BounceDots = memo(function BounceDots() {
  return (
    <div className="flex gap-1.5 items-center h-5">
      <span className="w-2 h-2 bg-slate-400 rounded-full animate-bounce-forever" style={{ animationDelay: '0ms' }} />
      <span className="w-2 h-2 bg-slate-400 rounded-full animate-bounce-forever" style={{ animationDelay: '160ms' }} />
      <span className="w-2 h-2 bg-slate-400 rounded-full animate-bounce-forever" style={{ animationDelay: '320ms' }} />
    </div>
  );
});

interface ChatBubbleProps {
  message: ChatMessage;
  isStreaming?: boolean;
  streamingThinking?: string;
  isThinkingPhase?: boolean;
  /** 助手气泡显示的角色名。未传时回退到「管家」。 */
  assistantName?: string;
  /** 助手气泡左上角图标。未传时回退到 Home。 */
  assistantIcon?: LucideIcon;
  /** 助手图标背景色（hex 或 css color），未传时使用默认 slate/indigo。 */
  assistantColor?: string;
}

export function ChatBubble({
  message,
  isStreaming,
  streamingThinking,
  isThinkingPhase,
  assistantName,
  assistantIcon,
  assistantColor,
}: ChatBubbleProps) {
  const isUser = message.role === 'user';
  const [thinkingExpanded, setThinkingExpanded] = useState(false);

  const thinkingText = streamingThinking || message.thinkingContent;
  const hasThinking = !isUser && thinkingText && thinkingText.length > 0;

  const AssistantIcon = assistantIcon ?? Home;
  const displayName = assistantName ?? '管家';

  return (
    <div className={cn(
      "flex flex-col gap-1.5",
      !isStreaming && "animate-in slide-in-from-bottom-2",
      isUser ? "items-end" : "items-start"
    )}>
      {hasThinking && (
        <div className="max-w-[85%]">
          {isThinkingPhase ? (
            <div className="bg-slate-50 dark:bg-slate-800/50 border border-slate-200/60 dark:border-slate-700/60 rounded-xl p-3 shadow-sm">
              <div className="flex items-center gap-2 mb-1.5">
                <span className="text-xs font-medium text-amber-500">思考中...</span>
                <span className="w-1.5 h-1.5 bg-amber-400 rounded-full animate-pulse" />
              </div>
              <div className="max-h-10 overflow-hidden flex flex-col justify-end">
                <p className="text-xs text-slate-400 dark:text-slate-500 leading-relaxed whitespace-pre-wrap">
                  {thinkingText}
                </p>
              </div>
            </div>
          ) : (
            <button
              onClick={() => setThinkingExpanded(prev => !prev)}
              className="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-500 dark:hover:text-slate-300 transition-colors py-1"
            >
              <ChevronRight size={12} className={cn("transition-transform", thinkingExpanded && "rotate-90")} />
              <span>思考过程</span>
            </button>
          )}
          {thinkingExpanded && !isThinkingPhase && (
            <div className="bg-slate-50 dark:bg-slate-800/50 border border-slate-200/60 dark:border-slate-700/60 rounded-xl p-3 mt-1 shadow-sm">
              <p className="text-xs text-slate-400 dark:text-slate-500 leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto">
                {thinkingText}
              </p>
            </div>
          )}
        </div>
      )}
      <div className={cn(
        "rounded-2xl px-5 py-3 max-w-[85%] text-[14.5px] leading-[1.7] shadow-sm",
        isUser
          ? "bg-slate-800 dark:bg-indigo-600 text-white rounded-tr-sm"
          : "bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 rounded-tl-sm text-slate-700 dark:text-slate-200"
      )}>
        {!isUser && (
          <div className="flex items-center gap-2 mb-2">
            <div
              className={cn(
                "w-5 h-5 rounded-md text-white flex items-center justify-center",
                !assistantColor && "bg-slate-800 dark:bg-indigo-600",
              )}
              style={assistantColor ? { backgroundColor: assistantColor } : undefined}
            >
              <AssistantIcon size={12} />
            </div>
            <span className="text-xs text-slate-400 font-medium">{displayName}</span>
          </div>
        )}
        {(isStreaming || isThinkingPhase) && !message.content ? (
          <BounceDots />
        ) : (
          <>
            {isUser ? (
              <span>{message.content}</span>
            ) : (
              <div className="prose prose-sm prose-slate dark:prose-invert max-w-none">
                <ReactMarkdown>{message.content}</ReactMarkdown>
              </div>
            )}
            {isStreaming && (
              <span className="inline-block w-0.5 h-4 bg-slate-600 dark:bg-slate-300 ml-0.5 animate-pulse" />
            )}
          </>
        )}
      </div>
    </div>
  );
}
