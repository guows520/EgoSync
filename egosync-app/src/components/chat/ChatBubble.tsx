import { useState, useEffect, useLayoutEffect, useRef, Fragment, memo, type AnchorHTMLAttributes, type ReactNode } from 'react';
import ReactMarkdown, { defaultUrlTransform } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { cn } from '../../lib/utils';
import { Home, ChevronRight, type LucideIcon } from 'lucide-react';
import type { ChatMessage, ExecutionTraceBlock, StreamPayload } from '../../types/chat';

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
  executionTraceBlocks?: ExecutionTraceBlock[];
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

type ExecutionActionType = Extract<ExecutionTraceBlock, { type: 'action' }>['actionType'];

function actionTypeLabel(type: ExecutionActionType) {
  switch (type) {
    case 'shell': return 'Shell';
    case 'read': return null;
    case 'edit': return 'Edit';
    case 'write': return 'Write';
    case 'skill': return 'Skill';
    case 'explore': return 'Explore';
    default: return 'Tool';
  }
}

function ExecutionActionDetails({ block }: { block: Extract<ExecutionTraceBlock, { type: 'action' }> }) {
  const details = block.details ?? [];
  const command = details.find(detail => detail.label === 'Command')?.value;
  const outputs = details.filter(detail => detail.label === 'Output' || detail.label === 'Error');
  const otherDetails = details.filter(detail => detail.label !== 'Command' && detail.label !== 'Output' && detail.label !== 'Error');

  return (
    <div className="space-y-2 border-t border-slate-200 p-3 text-[11px] leading-relaxed dark:border-slate-700">
      {command && (
        <pre className="max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-slate-100 p-2 text-slate-600 dark:bg-slate-950/60 dark:text-slate-300">
          {`$ ${command}`}
        </pre>
      )}
      {outputs.map(detail => (
        <pre
          key={`${detail.label}-${detail.value}`}
          className={cn(
            'max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-slate-100 p-2 text-slate-600 dark:bg-slate-950/60 dark:text-slate-300',
            detail.tone === 'error' && 'bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-300',
          )}
        >
          {detail.value}
        </pre>
      ))}
      {!command && outputs.length === 0 && otherDetails.map(detail => (
        <pre
          key={`${detail.label}-${detail.value}`}
          className={cn(
            'max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-slate-100 p-2 text-slate-600 dark:bg-slate-950/60 dark:text-slate-300',
            detail.tone === 'error' && 'bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-300',
          )}
        >
          {detail.value}
        </pre>
      ))}
    </div>
  );
}

function ThinkingTraceBlock({ block }: { block: Extract<ExecutionTraceBlock, { type: 'thinking' }> }) {
  const [expanded, setExpanded] = useState(block.isActive);
  const viewportRef = useRef<HTMLDivElement>(null);
  const hasContent = block.content.trim().length > 0;

  useEffect(() => {
    setExpanded(block.isActive);
  }, [block.isActive]);

  useLayoutEffect(() => {
    if (!block.isActive || !expanded) return;
    const viewport = viewportRef.current;
    if (viewport) viewport.scrollTop = viewport.scrollHeight;
  }, [block.content, block.isActive, expanded]);

  return (
    <div className="rounded-lg border border-slate-200/70 bg-white/60 dark:border-slate-700/70 dark:bg-slate-900/30">
      <button
        type="button"
        disabled={!hasContent}
        onClick={() => setExpanded(prev => !prev)}
        aria-expanded={hasContent && expanded}
        className={cn(
          'flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-slate-500 dark:text-slate-400',
          hasContent && 'hover:bg-slate-50 dark:hover:bg-slate-800/60',
        )}
      >
        <ChevronRight size={12} className={cn('shrink-0 transition-transform', expanded && 'rotate-90')} />
        <span>{block.isActive && (block.elapsedSeconds === null || block.elapsedSeconds === 0)
          ? 'Think 思考中'
          : typeof block.elapsedSeconds === 'number'
            ? `Think 思考了${block.elapsedSeconds}秒`
            : 'Think 思考时长未知'}</span>
      </button>
      {hasContent && expanded && (
        <div
          data-testid="thinking-content"
          className={cn(
            'border-t border-slate-200/70 px-3 py-2 text-xs leading-6 text-slate-500 dark:border-slate-700/70 dark:text-slate-400',
            !block.isActive && 'max-h-64 overflow-y-auto whitespace-pre-wrap',
          )}
        >
          {block.isActive ? (
            <div
              ref={viewportRef}
              data-testid="thinking-viewport"
              className="h-[4.5rem] overflow-hidden whitespace-pre-wrap"
            >
              {block.content}
            </div>
          ) : block.content}
        </div>
      )}
    </div>
  );
}

function ExecutionActionCard({ block }: { block: Extract<ExecutionTraceBlock, { type: 'action' }> }) {
  const [expanded, setExpanded] = useState(false);
  const hasDetails = Boolean(block.details?.length);
  const label = actionTypeLabel(block.actionType);
  return (
    <div className="rounded-lg border border-slate-200 bg-white/70 dark:border-slate-700 dark:bg-slate-900/40">
      <button
        type="button"
        onClick={() => setExpanded(prev => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-slate-600 hover:bg-slate-50 dark:text-slate-300 dark:hover:bg-slate-800/60"
      >
        <ChevronRight size={12} className={cn('shrink-0 transition-transform', expanded && 'rotate-90')} />
        {label && <span className="shrink-0 font-semibold">{label}</span>}
        <span className="min-w-0 truncate">{block.title}</span>
      </button>
      {block.previewLines && block.previewLines.length > 0 && (
        <div className="space-y-1 px-3 pb-2 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
          {block.previewLines.map((line, index) => (
            <div key={`${line}-${index}`}>{line}</div>
          ))}
        </div>
      )}
      {expanded && hasDetails && <ExecutionActionDetails block={block} />}
      {expanded && !hasDetails && block.detail && (
        <pre className="max-h-64 overflow-auto border-t border-slate-200 p-3 text-[11px] leading-relaxed text-slate-500 dark:border-slate-700 dark:text-slate-400">
          {block.detail}
        </pre>
      )}
    </div>
  );
}

export function ExecutionTrace({ blocks, defaultExpanded }: { blocks: ExecutionTraceBlock[]; defaultExpanded: boolean }) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  return (
    <div className="w-full max-w-[85%]">
      <button
        type="button"
        onClick={() => setExpanded(prev => !prev)}
        className="flex items-center gap-1.5 py-1 text-xs text-slate-400 transition-colors hover:text-slate-500 dark:hover:text-slate-300"
      >
        <ChevronRight size={12} className={cn('transition-transform', expanded && 'rotate-90')} />
        <span>执行过程</span>
      </button>
      {expanded && (
        <div className="mt-1 space-y-2 rounded-xl border border-slate-200/60 bg-slate-50 p-3 shadow-sm dark:border-slate-700/60 dark:bg-slate-800/50">
          {blocks.map(block => {
            if (block.type === 'thinking') return <ThinkingTraceBlock key={block.id} block={block} />;
            if (block.type === 'narration') {
              return (
                <p key={block.id} className="text-xs leading-relaxed text-slate-500 whitespace-pre-wrap dark:text-slate-400">
                  {block.content}
                </p>
              );
            }
            return <ExecutionActionCard key={block.id} block={block} />;
          })}
        </div>
      )}
    </div>
  );
}

export function ChatBubble({
  message,
  isStreaming,
  isThinkingPhase,
  streamStatus,
  executionTraceBlocks = [],
  assistantName,
  assistantIcon,
  assistantColor,
  onMemoryReferenceClick,
}: ChatBubbleProps) {
  const isUser = message.role === 'user';
  const statusText = streamStatus?.statusText;
  const traceBlocks: ExecutionTraceBlock[] = [];

  if (statusText && !isUser) {
    traceBlocks.push({
      id: 'stream-status',
      type: 'action',
      actionType: 'tool',
      title: statusText,
      status: 'running',
    });
  }
  traceBlocks.push(...executionTraceBlocks);
  if (
    !isUser
    && message.thinkingContent.trim().length > 0
    && !traceBlocks.some(block => block.type === 'thinking')
  ) {
    traceBlocks.unshift({
      id: `message-thinking-${message.id}`,
      type: 'thinking',
      content: message.thinkingContent,
      elapsedSeconds: null,
      isActive: false,
    });
  }

  const hasExecutionTrace = !isUser && traceBlocks.length > 0;
  const executionTraceDefaultExpanded = Boolean(isStreaming && hasExecutionTrace);

  const AssistantIcon = assistantIcon ?? Home;
  const displayName = assistantName ?? '管家';

  return (
    <div className={cn(
      "flex flex-col gap-1.5",
      isUser ? "items-end" : "items-start w-full"
    )}>
      {hasExecutionTrace && (
        <ExecutionTrace blocks={traceBlocks} defaultExpanded={executionTraceDefaultExpanded} />
      )}
      <div className={cn(
        "rounded-2xl px-5 py-3 max-w-[85%] text-[14.5px] leading-[1.7] shadow-sm",
        !isUser && "w-full",
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
                  remarkPlugins={[remarkGfm]}
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
                    table: ({ children }) => (
                      <div className="my-4 max-w-full overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
                        <table className="m-0 w-full min-w-max border-collapse text-left">{children}</table>
                      </div>
                    ),
                    thead: ({ children }) => <thead className="bg-slate-100 dark:bg-slate-800">{children}</thead>,
                    tr: ({ children }) => <tr className="border-b border-slate-200 last:border-b-0 dark:border-slate-700">{children}</tr>,
                    th: ({ children }) => <th className="whitespace-nowrap px-3 py-2 font-semibold text-slate-700 dark:text-slate-200">{children}</th>,
                    td: ({ children }) => <td className="px-3 py-2 align-top text-slate-600 dark:text-slate-300">{children}</td>,
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
