import { useState, Fragment, memo, type AnchorHTMLAttributes, type ReactNode } from 'react';
import ReactMarkdown, { defaultUrlTransform } from 'react-markdown';
import { cn } from '../../lib/utils';
import { Home, ChevronRight, type LucideIcon } from 'lucide-react';
import type { ChatMessage, StreamPayload } from '../../types/chat';

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
  streamStatus?: Pick<StreamPayload, 'phase' | 'statusText' | 'toolName'> | null;
  /** 助手气泡显示的角色名。未传时回退到「管家」。 */
  assistantName?: string;
  /** 助手气泡左上角图标。未传时回退到 Home。 */
  assistantIcon?: LucideIcon;
  /** 助手图标背景色（hex 或 css color），未传时使用默认 slate/indigo。 */
  assistantColor?: string;
  onMemoryReferenceClick?: (memoryId: string) => void;
}

const memoryReferencePattern = /\[记忆#([^\]]+)\]/g;
const memoryLinkScheme = 'egosync-memory://';

function memoryButton(label: ReactNode, memoryId: string, key?: string | number, accessibleLabel = memoryId, className?: string, onMemoryReferenceClick?: (memoryId: string) => void) {
  return (
    <button
      key={key}
      type="button"
      aria-label={`打开记忆 ${accessibleLabel}`}
      onClick={() => onMemoryReferenceClick?.(memoryId)}
      className={cn(
        'inline-flex rounded px-1 font-medium text-indigo-600 underline decoration-indigo-300 underline-offset-2 hover:text-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-400 focus:ring-offset-1 dark:text-indigo-300 dark:decoration-indigo-500',
        className,
      )}
    >
      {label}
    </button>
  );
}

function labelToAccessibleText(label: ReactNode): string {
  if (typeof label === 'string' || typeof label === 'number') return String(label);
  if (Array.isArray(label)) return label.map(labelToAccessibleText).join('');
  return '';
}

function MemoryAnchor({ href, children, onMemoryReferenceClick }: AnchorHTMLAttributes<HTMLAnchorElement> & { onMemoryReferenceClick?: (memoryId: string) => void }) {
  if (href?.startsWith(memoryLinkScheme)) {
    const memoryId = decodeURIComponent(href.slice(memoryLinkScheme.length));
    return memoryButton(children, memoryId, undefined, labelToAccessibleText(children), undefined, onMemoryReferenceClick);
  }

  return <a href={href}>{children}</a>;
}

function renderMemoryReferences(node: ReactNode, onMemoryReferenceClick?: (memoryId: string) => void): ReactNode {
  if (!onMemoryReferenceClick) return node;
  if (typeof node === 'string') {
    const parts: ReactNode[] = [];
    let lastIndex = 0;
    for (const match of node.matchAll(memoryReferencePattern)) {
      const index = match.index ?? 0;
      if (index > lastIndex) parts.push(node.slice(lastIndex, index));
      const label = match[0];
      const memoryId = match[1];
      parts.push(memoryButton(label, memoryId, `${memoryId}-${index}`, memoryId, undefined, onMemoryReferenceClick));
      lastIndex = index + label.length;
    }
    if (lastIndex === 0) return node;
    if (lastIndex < node.length) parts.push(node.slice(lastIndex));
    return parts;
  }
  if (Array.isArray(node)) {
    return node.map((child, index) => (
      <Fragment key={index}>{renderMemoryReferences(child, onMemoryReferenceClick)}</Fragment>
    ));
  }
  return node;
}

export function ChatBubble({
  message,
  isStreaming,
  streamingThinking,
  isThinkingPhase,
  streamStatus,
  assistantName,
  assistantIcon,
  assistantColor,
  onMemoryReferenceClick,
}: ChatBubbleProps) {
  const [thinkingExpanded, setThinkingExpanded] = useState(false);
  const isUser = message.role === 'user';
  const thinkingText = !isUser ? (streamingThinking || message.thinkingContent) : '';
  const hasThinking = Boolean(thinkingText && thinkingText.length > 0);
  const statusText = streamStatus?.statusText;

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
              type="button"
              onClick={() => setThinkingExpanded(prev => !prev)}
              className="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-500 dark:hover:text-slate-300 transition-colors py-1"
            >
              <ChevronRight size={12} className={cn('transition-transform', thinkingExpanded && 'rotate-90')} />
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
      {statusText && !isUser && (
        <div className="max-w-[85%] rounded-xl border border-slate-200/60 bg-slate-50 px-3 py-2 text-xs text-slate-500 shadow-sm dark:border-slate-700/60 dark:bg-slate-800/50 dark:text-slate-400">
          {statusText}
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
                <ReactMarkdown
                  urlTransform={value => value.startsWith(memoryLinkScheme) ? value : defaultUrlTransform(value)}
                  components={{
                    a: ({ href, children }) => (
                      <MemoryAnchor href={href} onMemoryReferenceClick={onMemoryReferenceClick}>
                        {children}
                      </MemoryAnchor>
                    ),
                    p: ({ children }) => <p>{renderMemoryReferences(children, onMemoryReferenceClick)}</p>,
                    li: ({ children }) => <li>{renderMemoryReferences(children, onMemoryReferenceClick)}</li>,
                    strong: ({ children }) => <strong>{renderMemoryReferences(children, onMemoryReferenceClick)}</strong>,
                    em: ({ children }) => <em>{renderMemoryReferences(children, onMemoryReferenceClick)}</em>,
                  }}
                >
                  {message.content}
                </ReactMarkdown>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
