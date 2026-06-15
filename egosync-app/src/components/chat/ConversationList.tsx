import { Trash2 } from 'lucide-react';
import { cn } from '../../lib/utils';
import type { Conversation } from '../../types/chat';

interface ConversationListProps {
  conversations: Conversation[];
  currentId: string;
  onSelect: (conv: Conversation) => void;
  onDelete: (convId: string) => void;
  onClose: () => void;
}

function formatRelativeTime(dateStr: string): string {
  const now = Date.now();
  const date = new Date(dateStr).getTime();
  if (isNaN(date)) return '';
  const diff = now - date;
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes}分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前`;
  const d = new Date(dateStr);
  return `${d.getMonth() + 1}月${d.getDate()}日 ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

export function ConversationList({ conversations, currentId, onSelect, onDelete, onClose }: ConversationListProps) {
  return (
    <div className="absolute right-0 top-full mt-1 w-72 max-h-80 overflow-y-auto bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl shadow-lg z-50">
      {conversations.length === 0 ? (
        <div className="p-4 text-center text-sm text-slate-400">暂无历史对话</div>
      ) : (
        conversations.map(conv => (
          <div
            key={conv.id}
            className={cn(
              "group flex items-center border-b border-slate-100 dark:border-slate-700/50 last:border-0 hover:bg-slate-50 dark:hover:bg-slate-700/50 transition-colors",
              conv.id === currentId && "bg-slate-50 dark:bg-slate-700/50"
            )}
          >
            <button
              onClick={() => { onSelect(conv); onClose(); }}
              className="flex-1 text-left px-4 py-2.5 min-w-0"
            >
              <div className="text-sm text-slate-700 dark:text-slate-200 truncate">
                {conv.title || '新对话'}
              </div>
              <div className="text-xs text-slate-400 mt-0.5">
                {formatRelativeTime(conv.updatedAt)}
              </div>
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDelete(conv.id);
              }}
              className="opacity-0 group-hover:opacity-100 p-2 mr-2 text-slate-400 hover:text-red-500 dark:hover:text-red-400 transition-all rounded-md hover:bg-red-50 dark:hover:bg-red-900/20"
              title="删除对话"
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))
      )}
    </div>
  );
}
