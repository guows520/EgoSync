import { useState, useRef, useEffect } from 'react';
import { Plus, History } from 'lucide-react';
import { ConversationList } from './ConversationList';
import type { Conversation } from '../../types/chat';

interface ChatHeaderProps {
  conversations: Conversation[];
  currentConversationId: string;
  onNewConversation: () => void;
  onSelectConversation: (conv: Conversation) => void;
  onDeleteConversation: (convId: string) => void;
}

export function ChatHeader({
  conversations,
  currentConversationId,
  onNewConversation,
  onSelectConversation,
  onDeleteConversation,
}: ChatHeaderProps) {
  const [showHistory, setShowHistory] = useState(false);
  const historyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (historyRef.current && !historyRef.current.contains(e.target as Node)) {
        setShowHistory(false);
      }
    };
    if (showHistory) document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [showHistory]);

  return (
    <div className="flex items-center justify-between px-8 py-2 bg-white/50 dark:bg-slate-800/50 backdrop-blur-sm shrink-0">
      <button
        onClick={onNewConversation}
        className="flex items-center gap-1.5 text-xs text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 transition-colors px-2.5 py-1.5 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700/50"
      >
        <Plus size={14} />
        <span>新对话</span>
      </button>

      <div ref={historyRef} className="relative">
        <button
          onClick={() => setShowHistory(prev => !prev)}
          className="flex items-center gap-1.5 text-xs text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 transition-colors px-2.5 py-1.5 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700/50"
        >
          <History size={14} />
          <span>历史对话</span>
        </button>
        {showHistory && (
          <ConversationList
            conversations={conversations}
            currentId={currentConversationId}
            onSelect={onSelectConversation}
            onDelete={onDeleteConversation}
            onClose={() => setShowHistory(false)}
          />
        )}
      </div>
    </div>
  );
}
