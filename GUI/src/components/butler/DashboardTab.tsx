import { ListTodo, Clock, AlertTriangle } from 'lucide-react';
import { cn } from '../../lib/utils';

export function DashboardTab({ roles, onViewChange }: any) {
  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between mb-1">
        <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">角色状态总览</h3>
      </div>
      {roles.map((role: any) => {
        const isLow = role.energy < 40;
        const isUrgent = role.status === 'yellow';
        return (
          <button
            key={role.id}
            onClick={() => onViewChange?.(role.id)}
            className={cn(
              "w-full text-left bg-white border rounded-xl p-4 shadow-sm transition-all hover:shadow-md hover:-translate-y-0.5 group",
              isUrgent ? "border-amber-300 ring-1 ring-amber-200" : isLow ? "border-red-200" : "border-slate-200 hover:border-indigo-200"
            )}
          >
            <div className="flex items-center gap-3.5">
              <div className={cn("w-10 h-10 rounded-lg flex items-center justify-center text-white shadow-sm shrink-0", role.color)}>
                <role.icon size={20} strokeWidth={2} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[14.5px] font-semibold text-slate-800 truncate">{role.name}</span>
                  <span className={cn("text-[12px] font-bold", role.energy >= 70 ? "text-emerald-600" : role.energy >= 40 ? "text-amber-600" : "text-red-500")}>{role.energy}%</span>
                </div>
                <div className="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden">
                  <div
                    className={cn("h-full rounded-full transition-all duration-500", role.energy >= 70 ? "bg-emerald-500" : role.energy >= 40 ? "bg-amber-500" : "bg-red-500")}
                    style={{ width: `${role.energy}%` }}
                  />
                </div>
                <div className="flex items-center gap-3 mt-2.5 text-[12px] text-slate-500">
                  <span className="flex items-center gap-1"><ListTodo size={12} /> 3 待办</span>
                  <span className="flex items-center gap-1"><Clock size={12} /> 2小时前活跃</span>
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
