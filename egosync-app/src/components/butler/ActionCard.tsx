import { useState, useEffect, useCallback } from 'react';
import { Check, X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent } from '../../lib/roleIcons';
import type { SuggestionWithRole } from '../../types/suggestion';
import { REJECT_REASONS, type RejectReason } from '../../hooks/useSuggestions';

interface ActionCardProps {
  suggestion: SuggestionWithRole;
  onConfirm: (id: string) => void;
  onReject: (id: string, reason: string) => void;
  onDismiss: (id: string) => void;
  /** 确认按钮文案（默认「确认」） */
  confirmLabel?: string;
  /** 拒绝按钮文案（默认「拒绝」） */
  rejectLabel?: string;
  /** 设置后点击拒绝直接以该原因拒绝，不展示原因选择器（用于敲门通知「稍后」） */
  directRejectReason?: string;
  /** 隐藏正文描述（敲门通知标题即内容，无需正文） */
  hideDescription?: boolean;
}

type CardStatus = 'pending' | 'confirming' | 'confirmed' | 'rejecting' | 'rejected';

const PRIORITY_LABELS: Record<string, string> = {
  high: '高优',
  medium: '中优',
  low: '低优',
};

const PRIORITY_COLORS: Record<string, string> = {
  high: 'bg-red-50 text-red-600 dark:bg-red-950/50 dark:text-red-300',
  medium: 'bg-amber-50 text-amber-600 dark:bg-amber-950/50 dark:text-amber-300',
  low: 'bg-slate-100 text-slate-500 dark:bg-slate-700 dark:text-slate-300',
};

function formatRelativeTime(iso: string): string {
  const now = Date.now();
  const created = Date.parse(iso);
  if (!Number.isFinite(created)) return '';
  const diffMs = now - created;
  const diffMin = Math.floor(diffMs / 60_000);
  if (diffMin < 1) return '刚刚';
  if (diffMin < 60) return `${diffMin} 分钟前`;
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour} 小时前`;
  const diffDay = Math.floor(diffHour / 24);
  if (diffDay < 7) return `${diffDay} 天前`;
  return new Date(created).toLocaleDateString();
}

export function ActionCard({ suggestion, onConfirm, onReject, onDismiss, confirmLabel = '确认', rejectLabel = '拒绝', directRejectReason, hideDescription = false }: ActionCardProps) {
  const [status, setStatus] = useState<CardStatus>('pending');
  const [showRejectionReasons, setShowRejectionReasons] = useState(false);
  const [selectedReason, setSelectedReason] = useState<RejectReason | null>(null);
  const [removed, setRemoved] = useState(false);
  const [showOtherInput, setShowOtherInput] = useState(false);
  const [otherReason, setOtherReason] = useState('');

  const handleConfirm = useCallback(() => {
    if (status !== 'pending') return;
    setStatus('confirming');
    Promise.resolve(onConfirm(suggestion.id)).then(() => {
      setStatus('confirmed');
    }).catch(() => {
      setStatus('pending');
    });
  }, [onConfirm, suggestion.id, status]);

  const handleRejectClick = useCallback(() => {
    if (status !== 'pending') return;
    if (directRejectReason !== undefined) {
      setSelectedReason('other');
      setStatus('rejecting');
      Promise.resolve(onReject(suggestion.id, directRejectReason)).then(() => {
        setStatus('rejected');
      }).catch(() => {
        setStatus('pending');
        setSelectedReason(null);
      });
      return;
    }
    setShowRejectionReasons(true);
  }, [status, directRejectReason, onReject, suggestion.id]);

  const handleReasonSelect = useCallback((reason: RejectReason) => {
    if (status !== 'pending') return;
    if (reason === 'other') {
      setShowOtherInput(true);
      return;
    }
    setSelectedReason(reason);
    setStatus('rejecting');
    Promise.resolve(onReject(suggestion.id, reason)).then(() => {
      setStatus('rejected');
    }).catch(() => {
      setStatus('pending');
      setSelectedReason(null);
      setShowRejectionReasons(false);
    });
  }, [onReject, suggestion.id, status]);

  const handleOtherSubmit = useCallback(() => {
    if (status !== 'pending') return;
    const trimmed = otherReason.trim();
    const finalReason = trimmed || 'other';
    setSelectedReason('other');
    setStatus('rejecting');
    Promise.resolve(onReject(suggestion.id, finalReason)).then(() => {
      setStatus('rejected');
    }).catch(() => {
      setStatus('pending');
      setSelectedReason(null);
      setShowRejectionReasons(false);
      setShowOtherInput(false);
    });
  }, [onReject, otherReason, suggestion.id, status]);

  useEffect(() => {
    if (status !== 'confirmed' && status !== 'rejected') return;
    const timer = window.setTimeout(() => {
      setRemoved(true);
      onDismiss(suggestion.id);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [status, onDismiss, suggestion.id]);

  if (removed) return null;

  const isActioned = status === 'confirmed' || status === 'rejected' || status === 'confirming' || status === 'rejecting';
  const priorityLabel = PRIORITY_LABELS[suggestion.priority] ?? suggestion.priority;
  const priorityColor = PRIORITY_COLORS[suggestion.priority] ?? PRIORITY_COLORS.low;
  const relativeTime = formatRelativeTime(suggestion.createdAt);

  return (
    <article
      role="article"
      aria-label={`${suggestion.title} - ${suggestion.roleName}`}
      className={cn(
        "bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-[10px] p-4 transition-all duration-200",
        "hover:shadow-[0_4px_12px_rgba(0,0,0,0.08)] hover:-translate-y-px",
        "motion-reduce:transition-none motion-reduce:hover:translate-y-0",
        isActioned && "opacity-50 motion-reduce:opacity-100",
        status === 'confirmed' && "border-indigo-300 dark:border-indigo-500",
        status === 'rejected' && "border-slate-300 dark:border-slate-600",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0 flex-1">
          <span
            className="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center"
            style={{ backgroundColor: `${suggestion.roleColor}15`, color: suggestion.roleColor }}
          >
            {(() => {
              const Icon = getRoleIconComponent(suggestion.roleIcon);
              return <Icon size={16} strokeWidth={2} />;
            })()}
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 flex-wrap">
              <h3 className="font-medium text-[14px] text-slate-800 dark:text-slate-100">{suggestion.title}</h3>
              <span className={cn("shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium", priorityColor)}>
                {priorityLabel}
              </span>
            </div>
            {!hideDescription && <p className="text-[12px] text-slate-500 dark:text-slate-300 mt-1">{suggestion.content}</p>}
            <div className="flex items-center gap-2 mt-2 text-[11px] text-slate-400 dark:text-slate-500">
              <span className="font-medium" style={{ color: suggestion.roleColor }}>{suggestion.roleName}</span>
              {relativeTime && (
                <>
                  <span aria-hidden="true">·</span>
                  <span>{relativeTime}</span>
                </>
              )}
            </div>
          </div>
        </div>

        {status === 'confirmed' && (
          <div className="shrink-0 w-7 h-7 rounded-full bg-indigo-100 dark:bg-indigo-950/60 flex items-center justify-center">
            <Check size={16} className="text-indigo-600 dark:text-indigo-300" />
          </div>
        )}
        {status === 'rejected' && (
          <div className="shrink-0 flex items-center gap-1.5">
            {selectedReason && (
              <span className="text-[11px] text-slate-400 dark:text-slate-500">
                {selectedReason === 'other' && otherReason.trim()
                  ? otherReason.trim()
                  : REJECT_REASONS.find(r => r.value === selectedReason)?.label}
              </span>
            )}
            <div className="w-7 h-7 rounded-full bg-slate-100 dark:bg-slate-700 flex items-center justify-center">
              <X size={16} className="text-slate-400 dark:text-slate-300" />
            </div>
          </div>
        )}
      </div>

      {status === 'pending' && !showRejectionReasons && (
        <div className="flex gap-2 mt-3 justify-end">
          <button
            type="button"
            onClick={handleRejectClick}
            className="px-3 py-1.5 rounded-lg text-[13px] font-medium text-slate-500 dark:text-slate-300 border border-slate-200 dark:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700 hover:text-slate-700 dark:hover:text-slate-100 transition-colors motion-reduce:transition-none"
          >
            {rejectLabel}
          </button>
          <button
            type="button"
            onClick={handleConfirm}
            className="px-4 py-1.5 rounded-lg text-[13px] font-medium bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm transition-colors motion-reduce:transition-none"
          >
            {confirmLabel}
          </button>
        </div>
      )}

      {showRejectionReasons && status === 'pending' && (
        <div className="mt-3 pt-3 border-t border-slate-100 dark:border-slate-700">
          <p className="text-[12px] text-slate-500 dark:text-slate-300 mb-2">拒绝原因：</p>
          <div className="flex gap-2 flex-wrap">
            {REJECT_REASONS.map(reason => (
              <button
                key={reason.value}
                type="button"
                onClick={() => handleReasonSelect(reason.value)}
                className={cn(
                  "px-2.5 py-1 rounded-lg text-[12px] font-medium border transition-colors motion-reduce:transition-none",
                  showOtherInput && reason.value === 'other'
                    ? "bg-indigo-50 dark:bg-indigo-950/60 border-indigo-300 dark:border-indigo-500 text-indigo-600 dark:text-indigo-300"
                    : "text-slate-600 dark:text-slate-300 bg-slate-50 dark:bg-slate-700 border-slate-200 dark:border-slate-600 hover:bg-slate-100 dark:hover:bg-slate-600 hover:text-slate-800 dark:hover:text-slate-100",
                )}
              >
                {reason.label}
              </button>
            ))}
          </div>
          {showOtherInput && (
            <div className="mt-2 flex gap-2 items-center">
              <input
                type="text"
                value={otherReason}
                onChange={e => setOtherReason(e.target.value)}
                placeholder="输入具体原因（可选）"
                className="flex-1 px-3 py-1.5 rounded-lg text-[12px] bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-100 border border-slate-200 dark:border-slate-600 placeholder:text-slate-400 dark:placeholder:text-slate-500 focus:outline-none focus:border-indigo-300 dark:focus:border-indigo-500"
              />
              <button
                type="button"
                onClick={handleOtherSubmit}
                className="px-3 py-1.5 rounded-lg text-[12px] font-medium bg-slate-700 dark:bg-indigo-600 text-white hover:bg-slate-800 dark:hover:bg-indigo-700 transition-colors motion-reduce:transition-none"
              >
                提交
              </button>
            </div>
          )}
        </div>
      )}
    </article>
  );
}
