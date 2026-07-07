import { Bell, X, ChevronDown, ChevronUp } from 'lucide-react';
import { useState } from 'react';
import { cn } from '../../lib/utils';
import type { NotificationWithRole } from '../../types/notification';

const levelConfig = {
  whisper: { label: '耳语', cls: 'bg-slate-100 dark:bg-slate-700 text-slate-500 dark:text-slate-400 border-slate-200 dark:border-slate-600' },
  tap: { label: '轻触', cls: 'bg-blue-50 text-blue-600 border-blue-100' },
  knock: { label: '敲门', cls: 'bg-red-50 text-red-600 border-red-100 animate-pulse' },
} as const;

function formatRelativeTime(iso: string): string {
  const now = Date.now();
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return iso;
  const diff = Math.max(0, now - then);
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  return `${days} 天前`;
}

interface NotificationPanelProps {
  onClose: () => void;
  notifications: NotificationWithRole[];
  isLoading: boolean;
  markAsRead: (id: string) => void;
}

export function NotificationPanel({ onClose, notifications, isLoading, markAsRead }: NotificationPanelProps) {
  const [readExpanded, setReadExpanded] = useState(false);

  const unread = notifications.filter(n => !n.isRead);
  const read = notifications.filter(n => n.isRead);

  return (
    <div
      className="fixed top-0 right-0 bottom-0 w-[380px] z-50 animate-in slide-in-from-right duration-300"
      role="complementary"
      aria-label="通知中心"
    >
      <div className="h-full bg-white dark:bg-slate-900 shadow-2xl border-l border-slate-200 dark:border-slate-700 flex flex-col">
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-slate-700 shrink-0">
          <h2 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2"><Bell size={18} /> 通知中心</h2>
          <button onClick={onClose} className="p-1.5 text-slate-400 dark:text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"><X size={18} /></button>
        </div>
        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {isLoading && (
            <div className="text-center text-[13px] text-slate-400 dark:text-slate-500 py-8">加载中...</div>
          )}
          {!isLoading && notifications.length === 0 && (
            <div className="text-center text-[13px] text-slate-400 dark:text-slate-500 py-12">暂时没有新通知</div>
          )}
          {unread.map(n => {
            const lc = levelConfig[n.level] ?? levelConfig.whisper;
            return (
              <div
                key={n.id}
                role="article"
                aria-label={`${n.roleName} - ${lc.label} 通知`}
                onClick={() => markAsRead(n.id)}
                className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-4 shadow-sm hover:shadow-md hover:-translate-y-px transition-all duration-200 cursor-pointer motion-reduce:transition-none"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: n.roleColor }}></span>
                    <span className="text-[13px] font-semibold text-slate-700 dark:text-slate-300">{n.roleName}</span>
                  </div>
                  <span className={cn("px-2 py-0.5 rounded text-[10px] font-bold border motion-reduce:animate-none", lc.cls)}>{lc.label}</span>
                </div>
                <p className="text-[13px] text-slate-600 dark:text-slate-300 leading-relaxed">{n.content}</p>
                <p className="text-[11px] text-slate-400 dark:text-slate-500 mt-2">{formatRelativeTime(n.createdAt)}</p>
              </div>
            );
          })}
          {read.length > 0 && (
            <div className="pt-2">
              <button
                onClick={() => setReadExpanded(v => !v)}
                className="w-full flex items-center justify-center gap-1.5 py-2 text-[12px] text-slate-400 dark:text-slate-500 hover:text-slate-600 dark:hover:text-slate-300 transition-colors"
              >
                {readExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                已读 ({read.length})
              </button>
              {readExpanded && (
                <div className="space-y-1.5 mt-2">
                  {read.map(n => {
                    const lc = levelConfig[n.level] ?? levelConfig.whisper;
                    return (
                      <div
                        key={n.id}
                        role="article"
                        aria-label={`${n.roleName} - ${lc.label} 已读通知`}
                        className="bg-slate-50 dark:bg-slate-800 border border-slate-100 dark:border-slate-700 rounded-lg px-3 py-2 opacity-60"
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-1.5">
                            <span className="w-2 h-2 rounded-full" style={{ backgroundColor: n.roleColor }}></span>
                            <span className="text-[12px] font-medium text-slate-500 dark:text-slate-400">{n.roleName}</span>
                          </div>
                          <span className={cn("px-1.5 py-0.5 rounded text-[9px] font-bold border", lc.cls)}>{lc.label}</span>
                        </div>
                        <p className="text-[12px] text-slate-400 dark:text-slate-500 leading-snug mt-1 line-clamp-2">{n.content}</p>
                        <p className="text-[10px] text-slate-300 dark:text-slate-600 mt-1">{formatRelativeTime(n.createdAt)}</p>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
