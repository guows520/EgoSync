import { Plus, Circle, GripVertical } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ROLE_TASKS } from '../../constants/mockData';

export function TasksTab({ role, onOpenTask }: any) {
  const tasks = ROLE_TASKS[role.id] || [{ title: '暂无任务', deadline: undefined, isBigRock: false }];
  return (
    <div className="space-y-6">
      <div>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">Q1 · 重要且紧急</h3>
          <button onClick={onOpenTask} className={cn("hover:bg-slate-200 p-1.5 rounded-md transition-colors", role.text)}><Plus size={18}/></button>
        </div>
        <div className="space-y-2.5">
          {tasks.map((task: any, i: number) => (
            <div key={i} className="bg-white border border-slate-200 rounded-xl p-4 flex gap-3.5 shadow-sm group cursor-pointer hover:border-indigo-300 hover:shadow-md transition-all">
              <GripVertical size={18} className="text-slate-300 shrink-0 mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab" />
              <button className="text-slate-300 hover:text-emerald-500 transition-colors shrink-0 mt-0.5"><Circle size={20} strokeWidth={2.5} /></button>
              <div className="flex-1">
                <p className="text-[14.5px] text-slate-800 font-medium leading-snug">{task.title}</p>
                <div className="flex items-center gap-2 mt-2.5">
                  {task.deadline && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-red-50 text-red-600 border border-red-100">{task.deadline}</span>}
                  {task.isBigRock && <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-amber-50 text-amber-600 border border-amber-100">大石头</span>}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
