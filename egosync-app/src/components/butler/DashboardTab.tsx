import { ListTodo, Clock, AlertTriangle } from 'lucide-react';
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
  const { statuses, isLoading, error } = useDashboard();

  if (isLoading) {
    return <div className="text-center text-slate-400 text-sm py-8">加载中…</div>;
  }

  if (error) {
    return <div className="text-center text-red-500 text-sm py-8">{error}</div>;
  }

  if (statuses.length === 0) {
    return <div className="text-center text-slate-400 text-sm py-8">暂无角色数据</div>;
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between mb-1">
        <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">角色状态总览</h3>
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
              "w-full text-left bg-white border rounded-xl p-4 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 group",
              isLow ? "border-red-200" : "border-slate-200 hover:border-indigo-200"
            )}
          >
            <div className="flex items-center gap-3.5">
              <div className="w-10 h-10 rounded-lg flex items-center justify-center text-white shadow-sm shrink-0" style={{ backgroundColor: roleColor }}>
                <Icon size={20} strokeWidth={2} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[14.5px] font-semibold text-slate-800 truncate">{role.roleName}</span>
                  <span className={cn("text-[12px] font-bold", energy >= 70 ? "text-emerald-600" : energy >= 40 ? "text-amber-600" : "text-red-500")}>{energy}%</span>
                </div>
                <div
                  className="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden"
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
                <div className="flex items-center gap-3 mt-2.5 text-[12px] text-slate-500">
                  <span className="flex items-center gap-1"><ListTodo size={12} /> {role.pendingTasksCount} 待办</span>
                  <span className="flex items-center gap-1"><Clock size={12} /> {formatRelativeTime(role.lastActiveAt)}</span>
                  {isUrgent && <span className="text-amber-600 font-medium flex items-center gap-1"><AlertTriangle size={12} /> 需关注</span>}
                </div>
              </div>
            </div>
          </button>
        );
      })}
    </div>
  );
}
