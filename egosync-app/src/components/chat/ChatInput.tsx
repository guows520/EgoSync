import { useState } from 'react';
import { Play, Square } from 'lucide-react';
import { cn } from '../../lib/utils';

interface ChatInputProps {
  onSend: (content: string) => void;
  onStop?: () => void;
  isStreaming?: boolean;
  disabled?: boolean;
  placeholder?: string;
  useRoleAccent?: boolean;
}

export function ChatInput({ onSend, onStop, isStreaming, disabled, placeholder, useRoleAccent }: ChatInputProps) {
  const [input, setInput] = useState('');

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setInput('');
  };

  return (
    <div className="relative w-full">
      <input
        type="text"
        value={input}
        onChange={e => setInput(e.target.value)}
        onKeyDown={e => e.key === 'Enter' && !e.shiftKey && handleSend()}
        placeholder={placeholder ?? "跟管家说点什么，比如：帮我安排一个会议..."}
        disabled={disabled}
        className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-500/20 transition-colors shadow-sm disabled:opacity-60 disabled:cursor-not-allowed"
      />
      {isStreaming ? (
        <button
          onClick={onStop}
          aria-label="停止"
          className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700"
        >
          <Square size={14} fill="currentColor" />
        </button>
      ) : (
        <button
          onClick={handleSend}
          disabled={disabled || !input.trim()}
          aria-label="发送"
          className={cn(
            'absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white rounded-lg transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed',
            useRoleAccent ? 'hover:brightness-110' : 'bg-slate-800 hover:bg-slate-700',
          )}
          style={useRoleAccent ? { backgroundColor: 'var(--role-accent)' } : undefined}
        >
          <Play size={16} className="ml-0.5" fill="currentColor" />
        </button>
      )}
    </div>
  );
}
