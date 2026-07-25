import { ListTodo, Clock, AlertTriangle, Brain, MessageSquare } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import { useDashboard } from '../../hooks/useDashboard';
import type { DashboardStatus } from '../../types/dashboard';

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

  const dateInputToIso = (value: string, dayOffset = 0): string | null => {
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
  };

  const handleStartChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTimeRange((prev) => ({
      ...prev,
      startAt: dateInputToIso(e.target.value),
    }));
  };

  const handleEndChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTimeRange((prev) => ({
      ...prev,
      endAt: dateInputToIso(e.target.value, 1),
    }));
  };

  const scopeValue =
    scope.type === 'all' ? 'all' : scope.type === 'butler' ? 'butler' : scope.roleId;

  const toDateInput = (iso: string | null, exclusiveEnd = false): string => {
    if (!iso) return '';
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return '';
    if (exclusiveEnd) date.setDate(date.getDate() - 1);
    const pad = (value: number) => String(value).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  };

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

          <input
            type="date"
            value={toDateInput(timeRange.startAt)}
            onChange={handleStartChange}
            className="shrink-0 text-[12px] rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2.5 py-1.5 text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-400"
            aria-label="开始日期"
          />
          <span className="text-[12px] text-slate-400">至</span>
          <input
            type="date"
            value={toDateInput(timeRange.endAt, true)}
            onChange={handleEndChange}
            className="shrink-0 text-[12px] rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2.5 py-1.5 text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-400"
            aria-label="结束日期"
          />
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
                className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-3"
                aria-label={card.label}
              >
                <div className="flex min-w-0 items-center gap-2">
                  <Icon size={20} className={cn(card.color, "shrink-0")} />
                  <div className="truncate text-[11px] text-slate-400 dark:text-slate-500">{card.label}</div>
                </div>
                <div className="mt-2 shrink-0 text-right text-[20px] font-bold text-slate-800 dark:text-slate-100">
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
