import { useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { ListTodo, Clock, AlertTriangle, Brain, MessageSquare, CalendarDays, ChevronDown } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import { useDashboard } from '../../hooks/useDashboard';
import type { DashboardStatus } from '../../types/dashboard';

type TimeRange = {
  startAt: string | null;
  endAt: string | null;
};

type DatePreset = 'all' | 'recent3' | 'recent7' | 'recentMonth' | 'custom';

const DATE_PRESETS: Array<{ value: DatePreset; label: string }> = [
  { value: 'all', label: '全部日期' },
  { value: 'recent3', label: '最近3天' },
  { value: 'recent7', label: '最近7天' },
  { value: 'recentMonth', label: '最近1个月' },
  { value: 'custom', label: '自定义时间' },
];

function dateInputToIso(value: string, dayOffset = 0): string | null {
  if (!value) return null;
  const match = /^(\d{4,})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return null;

  const [, yearText, monthText, dayText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const date = new Date(0);
  date.setFullYear(year, month - 1, day);
  date.setHours(0, 0, 0, 0);
  if (
    Number.isNaN(date.getTime())
    || date.getFullYear() !== year
    || date.getMonth() !== month - 1
    || date.getDate() !== day
  ) return null;

  date.setDate(date.getDate() + dayOffset);
  return date.toISOString();
}

function toDateInput(iso: string | null, exclusiveEnd = false): string {
  if (!iso) return '';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  if (exclusiveEnd) date.setDate(date.getDate() - 1);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function getPresetTimeRange(preset: Exclude<DatePreset, 'custom'>, now = new Date()): TimeRange {
  if (preset === 'all') return { startAt: null, endAt: null };

  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const end = new Date(today);
  end.setDate(end.getDate() + 1);
  const start = new Date(today);

  if (preset === 'recent3') {
    start.setDate(start.getDate() - 2);
  } else if (preset === 'recent7') {
    start.setDate(start.getDate() - 6);
  } else {
    const day = start.getDate();
    start.setDate(1);
    start.setMonth(start.getMonth() - 1);
    const lastDay = new Date(start.getFullYear(), start.getMonth() + 1, 0).getDate();
    start.setDate(Math.min(day, lastDay));
  }

  return { startAt: start.toISOString(), endAt: end.toISOString() };
}

type DateRangeFilterProps = {
  timeRange: TimeRange;
  setTimeRange: Dispatch<SetStateAction<TimeRange>>;
};

function DateRangeFilter({ timeRange, setTimeRange }: DateRangeFilterProps) {
  const [datePreset, setDatePreset] = useState<DatePreset>('all');
  const [isOpen, setIsOpen] = useState(false);
  const [isCustomEditorOpen, setIsCustomEditorOpen] = useState(false);
  const [draftStart, setDraftStart] = useState('');
  const [draftEnd, setDraftEnd] = useState('');

  const appliedStart = toDateInput(timeRange.startAt);
  const appliedEnd = toDateInput(timeRange.endAt, true);
  const datePresetLabel = DATE_PRESETS.find((item) => item.value === datePreset)?.label ?? '全部日期';
  const displayLabel = datePreset === 'custom' && appliedStart && appliedEnd
    ? `${appliedStart} 至 ${appliedEnd}`
    : datePresetLabel;
  const customRangeError = draftStart && draftEnd && draftStart > draftEnd
    ? '结束日期不能早于开始日期'
    : '';

  const openCustomEditor = () => {
    setDraftStart(appliedStart);
    setDraftEnd(appliedEnd);
    setIsCustomEditorOpen(true);
  };

  const handleToggle = () => {
    const nextOpen = !isOpen;
    setIsOpen(nextOpen);
    if (nextOpen && datePreset === 'custom') openCustomEditor();
  };

  const handlePresetSelect = (preset: DatePreset) => {
    if (preset === 'custom') {
      openCustomEditor();
      setIsOpen(true);
      return;
    }

    setDatePreset(preset);
    setIsCustomEditorOpen(false);
    setTimeRange(getPresetTimeRange(preset));
    setIsOpen(false);
  };

  const handleCustomApply = () => {
    if (!draftStart || !draftEnd || customRangeError) return;
    const startAt = dateInputToIso(draftStart);
    const endAt = dateInputToIso(draftEnd, 1);
    if (!startAt || !endAt) return;

    setDatePreset('custom');
    setTimeRange({ startAt, endAt });
    setIsCustomEditorOpen(false);
    setIsOpen(false);
  };

  return (
    <div className="relative w-[196px] shrink-0">
      <button
        type="button"
        onClick={handleToggle}
        className="flex w-full items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-2.5 py-1.5 text-left text-[12px] text-slate-700 focus:outline-none focus:ring-1 focus:ring-indigo-400 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
        aria-label="日期筛选"
        aria-haspopup="dialog"
        aria-expanded={isOpen}
      >
        <CalendarDays size={14} className="shrink-0 text-slate-400" />
        <span className="min-w-0 flex-1 truncate">{displayLabel}</span>
        <ChevronDown size={14} className="shrink-0 text-slate-400" />
      </button>

      {isOpen && (
        <div
          role="dialog"
          aria-label="日期筛选选项"
          className="absolute right-0 top-full z-20 mt-2 w-[250px] rounded-xl border border-slate-200 bg-white p-2 shadow-lg dark:border-slate-700 dark:bg-slate-800"
        >
          <div className="space-y-1" aria-label="日期预设选项">
            {DATE_PRESETS.map((preset) => (
              <button
                key={preset.value}
                type="button"
                onClick={() => handlePresetSelect(preset.value)}
                className={cn(
                  'w-full rounded-lg px-2.5 py-1.5 text-left text-[12px] transition-colors',
                  (datePreset === preset.value || (preset.value === 'custom' && isCustomEditorOpen))
                    ? 'bg-indigo-50 text-indigo-700 dark:bg-indigo-950/50 dark:text-indigo-300'
                    : 'text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-700',
                )}
                aria-pressed={datePreset === preset.value || (preset.value === 'custom' && isCustomEditorOpen)}
              >
                {preset.label}
              </button>
            ))}
          </div>

          {isCustomEditorOpen && (
            <div className="mt-2 space-y-2 border-t border-slate-200 pt-2 dark:border-slate-700">
              <label className="block text-[11px] text-slate-500 dark:text-slate-400">
                开始日期
                <input
                  type="date"
                  value={draftStart}
                  onChange={(e) => setDraftStart(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-2 py-1.5 text-[12px] text-slate-700 focus:outline-none focus:ring-1 focus:ring-indigo-400 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200"
                  aria-label="自定义开始日期"
                />
              </label>
              <label className="block text-[11px] text-slate-500 dark:text-slate-400">
                结束日期
                <input
                  type="date"
                  value={draftEnd}
                  onChange={(e) => setDraftEnd(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-2 py-1.5 text-[12px] text-slate-700 focus:outline-none focus:ring-1 focus:ring-indigo-400 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200"
                  aria-label="自定义结束日期"
                />
              </label>
              {customRangeError && <div className="text-[11px] text-red-500">{customRangeError}</div>}
              <div className="flex justify-end gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => {
                    setIsCustomEditorOpen(false);
                    setIsOpen(false);
                  }}
                  className="rounded-lg px-2.5 py-1.5 text-[12px] text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-700"
                >
                  取消
                </button>
                <button
                  type="button"
                  onClick={handleCustomApply}
                  disabled={!draftStart || !draftEnd || Boolean(customRangeError)}
                  className="rounded-lg bg-indigo-600 px-2.5 py-1.5 text-[12px] text-white hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  应用日期
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function clampEnergy(value: number): number {
  if (typeof value !== 'number' || Number.isNaN(value)) return 0;
  return Math.max(0, Math.min(100, value));
}

function formatRelativeTime(iso: string | null): string {
  if (!iso) return '暂无活动';
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '暂无活动';
  const diffMs = Date.now() - then;
  if (diffMs < 0) return '刚刚活跃';
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 1) return '刚刚活跃';
  if (minutes < 60) return `${minutes}分钟前活跃`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前活跃`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}天前活跃`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}个月前活跃`;
  return `${Math.floor(months / 12)}年前活跃`;
}

export function DashboardTab({ onViewChange }: { onViewChange?: (view: string) => void }) {
  const {
    statuses,
    isLoading,
    error,
    metrics,
    metricsLoading,
    metricsError,
    scope,
    setScope,
    timeRange,
    setTimeRange,
  } = useDashboard();

  const handleScopeChange = (value: string) => {
    if (value === 'all') {
      setScope({ type: 'all' });
    } else if (value === 'butler') {
      setScope({ type: 'butler' });
    } else {
      setScope({ type: 'role', roleId: value });
    }
  };

  const scopeValue =
    scope.type === 'all' ? 'all' : scope.type === 'butler' ? 'butler' : scope.roleId;

  const metricCards = [
    { icon: ListTodo, label: '任务总数', value: metrics?.taskCount ?? 0, color: 'text-indigo-500' },
    { icon: Brain, label: '记忆数量', value: metrics?.memoryCount ?? 0, color: 'text-purple-500' },
    { icon: Clock, label: '待处理任务', value: metrics?.pendingTaskCount ?? 0, color: 'text-amber-500' },
    { icon: MessageSquare, label: '对话数量', value: metrics?.conversationCount ?? 0, color: 'text-blue-500' },
  ];

  if (isLoading) {
    return <div className="text-center text-slate-400 dark:text-slate-500 text-sm py-8">加载中…</div>;
  }

  if (error) {
    return <div className="text-center text-red-500 text-sm py-8">{error}</div>;
  }

  return (
    <div className="space-y-5">
      <div className="space-y-3">
        <div className="flex items-center justify-between mb-1">
          <h3 className="text-[12px] font-bold tracking-widest text-slate-400 dark:text-slate-500 uppercase">活动统计</h3>
        </div>

        <div className="flex items-center gap-2 mb-2">
          <select
            value={scopeValue}
            onChange={(e) => handleScopeChange(e.target.value)}
            className="min-w-0 flex-1 text-[12px] rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2.5 py-1.5 text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-400"
            aria-label="Agent 筛选器"
          >
            <option value="all">全部</option>
            <option value="butler">管家</option>
            {statuses.map((role: DashboardStatus) => (
              <option key={role.roleId} value={role.roleId}>{role.roleName}</option>
            ))}
          </select>

          <DateRangeFilter timeRange={timeRange} setTimeRange={setTimeRange} />
        </div>

        {metricsError && metrics === null && (
          <div className="text-center text-red-500 text-sm py-2">{metricsError}</div>
        )}
        {metricsError && metrics !== null && (
          <div className="text-center text-amber-500 text-[12px] py-1">{metricsError}（显示上一次成功数据）</div>
        )}

        <div className="grid grid-cols-2 gap-2.5" aria-label="活动指标卡区域">
          {metricCards.map((card) => {
            const Icon = card.icon;
            return (
              <div
                key={card.label}
                className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-2.5"
                aria-label={card.label}
              >
                <div className="flex min-w-0 items-center gap-2">
                  <Icon size={18} className={cn(card.color, "shrink-0")} />
                  <div className="truncate text-[11px] text-slate-400 dark:text-slate-500">{card.label}</div>
                </div>
                <div className="mt-1.5 shrink-0 text-right text-[18px] font-bold text-slate-800 dark:text-slate-100">
                  {/* AC-13 加载期间保留上一次成功统计：仅首次加载（无缓存）时显示省略号 */}
                  {metricsLoading && metrics === null ? '…' : card.value}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {statuses.length === 0 ? (
        <div className="text-center text-slate-400 dark:text-slate-500 text-sm py-8">暂无角色数据</div>
      ) : (
        <>
          <div className="flex items-center justify-between mb-1">
            <h3 className="text-[12px] font-bold tracking-widest text-slate-400 dark:text-slate-500 uppercase">角色状态总览</h3>
          </div>
          {statuses.map((role: DashboardStatus) => {
        const energy = clampEnergy(role.energy);
        const isLow = energy < 40;
        const isUrgent = role.hasUrgent;
        const Icon = getRoleIconComponent(role.roleIcon);
        const roleColor = normalizeColorHex(role.roleColor);
        return (
          <button
            key={role.roleId}
            onClick={() => onViewChange?.(role.roleId)}
            className={cn(
              "w-full text-left bg-white dark:bg-slate-800 border rounded-xl p-4 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 group",
              isLow ? "border-red-200 dark:border-red-800" : "border-slate-200 dark:border-slate-700 hover:border-indigo-200 dark:hover:border-indigo-700"
            )}
          >
            <div className="flex items-center gap-3.5">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center text-white shadow-sm shrink-0" style={{ backgroundColor: roleColor }}>
                <Icon size={20} strokeWidth={2} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[14.5px] font-semibold text-slate-800 dark:text-slate-100 truncate">{role.roleName}</span>
                  <span className={cn("text-[12px] font-bold", energy >= 70 ? "text-emerald-600" : energy >= 40 ? "text-amber-600" : "text-red-500")}>{energy}%</span>
                </div>
                <div
                  className="w-full h-1.5 bg-slate-100 dark:bg-slate-700 rounded-full overflow-hidden"
                  role="progressbar"
                  aria-valuenow={energy}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label={`${role.roleName} 能量值 ${energy}%`}
                >
                  <div
                    className={cn("h-full rounded-full transition-all duration-500", energy >= 70 ? "bg-emerald-500" : energy >= 40 ? "bg-amber-500" : "bg-red-500")}
                    style={{ width: `${energy}%` }}
                  />
                </div>
                <div className="flex items-center gap-3 mt-2.5 text-[12px] text-slate-500 dark:text-slate-400">
                  <span className="flex items-center gap-1"><ListTodo size={12} /> {role.pendingTasksCount} 待办</span>
                  <span className="flex items-center gap-1"><Clock size={12} /> {formatRelativeTime(role.lastActiveAt)}</span>
                  {isUrgent && <span className="text-amber-600 font-medium flex items-center gap-1"><AlertTriangle size={12} /> 需关注</span>}
                </div>
              </div>
            </div>
          </button>
        );
      })}
        </>
      )}
    </div>
  );
}
