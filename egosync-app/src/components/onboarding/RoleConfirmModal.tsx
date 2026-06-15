import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';
import {
  ROLE_ICONS,
  ROLE_COLORS,
  DEFAULT_ICON_ID,
  DEFAULT_COLOR_HEX,
  normalizeIconId,
  normalizeColorHex,
  getRoleIconComponent,
} from '../../lib/roleIcons';

export interface RoleProposal {
  name: string;
  icon: string | null;   // backend payload icon id (white-list) or null
  color: string | null;  // backend payload hex or null
  goal: string | null;
}

export interface RoleConfirmModalProps {
  /** Whether the modal is visible. */
  open: boolean;
  /** Initial values for the form, coming from the LLM proposal. */
  proposal: RoleProposal | null;
  /** Called when user clicks "创建"; receives the (possibly edited) values. */
  onConfirm: (values: { name: string; icon: string; color: string; goal: string }) => void;
  /** Called when user clicks "不需要" or closes the modal. */
  onCancel: () => void;
  /** Set to true while the parent is awaiting roleService.create() to disable buttons. */
  busy?: boolean;
}

/**
 * 角色提议确认弹窗。
 *
 * 标题固定为「需要为您创建这个角色吗？」
 *
 * 用户可在创建前编辑：
 * - 名称（文本输入，必填）
 * - 图标（24 个黑白线框 Lucide 图标网格，单选）
 * - 品牌色（8 个色板，单选）
 * - 目标（多行文本输入，可空）
 *
 * 点「创建」→ onConfirm；点「不需要」/关闭 → onCancel。
 */
export function RoleConfirmModal({ open, proposal, onConfirm, onCancel, busy }: RoleConfirmModalProps) {
  const [name, setName] = useState('');
  const [iconId, setIconId] = useState<string>(DEFAULT_ICON_ID);
  const [colorHex, setColorHex] = useState<string>(DEFAULT_COLOR_HEX);
  const [goal, setGoal] = useState('');

  // 弹窗每次打开时，用提议数据重置表单
  useEffect(() => {
    if (open && proposal) {
      setName(proposal.name?.trim() ?? '');
      setIconId(normalizeIconId(proposal.icon));
      setColorHex(normalizeColorHex(proposal.color));
      setGoal(proposal.goal?.trim() ?? '');
    }
  }, [open, proposal]);

  // ESC 关闭
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !busy) {
        onCancel();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, busy, onCancel]);

  if (!open) return null;

  const trimmedName = name.trim();
  const canConfirm = trimmedName.length > 0 && !busy;

  const handleConfirm = () => {
    if (!canConfirm) return;
    onConfirm({
      name: trimmedName,
      icon: iconId,
      color: colorHex,
      goal: goal.trim(),
    });
  };

  const SelectedIcon = getRoleIconComponent(iconId);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="role-confirm-modal-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onClick={() => {
        if (!busy) onCancel();
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        className="bg-white dark:bg-slate-800 rounded-2xl shadow-2xl w-full max-w-md mx-4 p-6 space-y-5 max-h-[90vh] overflow-y-auto"
      >
        {/* Header */}
        <div className="flex items-start justify-between gap-3">
          <h3
            id="role-confirm-modal-title"
            className="text-lg font-semibold text-slate-800 dark:text-slate-100 leading-tight"
          >
            需要为您创建这个角色吗？
          </h3>
          <button
            onClick={() => {
              if (!busy) onCancel();
            }}
            aria-label="关闭"
            disabled={busy}
            className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors disabled:opacity-40"
          >
            <X size={20} />
          </button>
        </div>

        {/* Preview tile */}
        <div className="flex items-center gap-3 p-3 rounded-xl bg-slate-50 dark:bg-slate-700/40">
          <div
            className="w-12 h-12 rounded-xl flex items-center justify-center text-white shadow-sm shrink-0"
            style={{ backgroundColor: colorHex }}
            aria-hidden="true"
          >
            <SelectedIcon size={22} strokeWidth={2} />
          </div>
          <div className="min-w-0">
            <div className="text-[15px] font-medium text-slate-800 dark:text-slate-100 truncate">
              {trimmedName || '（请输入角色名）'}
            </div>
            {goal.trim() && (
              <div className="text-xs text-slate-500 dark:text-slate-400 truncate">{goal.trim()}</div>
            )}
          </div>
        </div>

        {/* Name */}
        <div className="space-y-1.5">
          <label htmlFor="role-name" className="text-xs font-medium text-slate-600 dark:text-slate-300">
            名称
          </label>
          <input
            id="role-name"
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            disabled={busy}
            maxLength={20}
            placeholder="例如：产品经理"
            className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-lg px-3 py-2 text-[14px] text-slate-800 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-indigo-500/30 focus:border-indigo-500 disabled:opacity-50"
          />
        </div>

        {/* Icon picker */}
        <div className="space-y-1.5">
          <label className="text-xs font-medium text-slate-600 dark:text-slate-300">图标</label>
          <div
            role="radiogroup"
            aria-label="选择图标"
            className="grid grid-cols-8 gap-1.5"
          >
            {ROLE_ICONS.map(opt => {
              const Component = opt.component;
              const selected = opt.id === iconId;
              return (
                <button
                  key={opt.id}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  aria-label={opt.label}
                  title={opt.label}
                  disabled={busy}
                  onClick={() => setIconId(opt.id)}
                  className={cn(
                    'aspect-square w-full rounded-lg flex items-center justify-center transition-all border',
                    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500',
                    selected
                      ? 'bg-slate-800 dark:bg-indigo-600 text-white border-slate-800 dark:border-indigo-600 shadow-sm'
                      : 'bg-white dark:bg-slate-700 text-slate-600 dark:text-slate-300 border-slate-200 dark:border-slate-600 hover:bg-slate-100 dark:hover:bg-slate-600',
                    busy && 'opacity-50 cursor-not-allowed',
                  )}
                >
                  <Component size={16} strokeWidth={2} />
                </button>
              );
            })}
          </div>
        </div>

        {/* Color picker */}
        <div className="space-y-1.5">
          <label className="text-xs font-medium text-slate-600 dark:text-slate-300">品牌色</label>
          <div role="radiogroup" aria-label="选择品牌色" className="flex flex-wrap gap-2">
            {ROLE_COLORS.map(opt => {
              const selected = opt.hex === colorHex;
              return (
                <button
                  key={opt.hex}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  aria-label={opt.label}
                  title={opt.label}
                  disabled={busy}
                  onClick={() => setColorHex(opt.hex)}
                  className={cn(
                    'w-8 h-8 rounded-full transition-all border-2',
                    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-indigo-500',
                    selected
                      ? 'border-slate-800 dark:border-slate-100 scale-110 shadow-sm'
                      : 'border-transparent hover:scale-105',
                    busy && 'opacity-50 cursor-not-allowed',
                  )}
                  style={{ backgroundColor: opt.hex }}
                />
              );
            })}
          </div>
        </div>

        {/* Goal */}
        <div className="space-y-1.5">
          <label htmlFor="role-goal" className="text-xs font-medium text-slate-600 dark:text-slate-300">
            目标 <span className="text-slate-400">（可选）</span>
          </label>
          <textarea
            id="role-goal"
            value={goal}
            onChange={e => setGoal(e.target.value)}
            disabled={busy}
            maxLength={80}
            rows={2}
            placeholder="例如：打磨更好的产品，与用户共创"
            className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-lg px-3 py-2 text-[14px] text-slate-800 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-indigo-500/30 focus:border-indigo-500 disabled:opacity-50 resize-none"
          />
        </div>

        {/* Footer buttons */}
        <div className="flex items-center justify-end gap-2 pt-1">
          <button
            type="button"
            onClick={onCancel}
            disabled={busy}
            className="px-4 py-2 text-[14px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors disabled:opacity-40"
          >
            不需要
          </button>
          <button
            type="button"
            onClick={handleConfirm}
            disabled={!canConfirm}
            className="px-5 py-2 text-[14px] font-medium text-white bg-slate-800 dark:bg-indigo-600 hover:opacity-90 rounded-lg shadow-sm transition-opacity disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {busy ? '创建中…' : '创建'}
          </button>
        </div>
      </div>
    </div>
  );
}