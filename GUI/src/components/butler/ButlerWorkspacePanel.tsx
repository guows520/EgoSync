import { X, Plus, Circle } from 'lucide-react';
import { cn } from '../../lib/utils';
import { DashboardTab } from './DashboardTab';
import { MemoryTab } from '../role/MemoryTab';
import { ButlerSettingsContent } from './ButlerSettingsContent';

export function ButlerWorkspacePanel({ roles, currentTab, setTab, archivedRoles, onRestoreRole, onViewChange }: any) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b border-slate-200/80 px-6 pt-4 bg-white/60 backdrop-blur-md shrink-0">
        <div className="flex gap-6">
          <button onClick={() => setTab('dashboard')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'dashboard' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            仪表盘
          </button>
          <button onClick={() => setTab('tasks')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'tasks' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            通用任务
          </button>
          <button onClick={() => setTab('memory')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'memory' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            管家记忆
          </button>
          <button onClick={() => setTab('settings')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'settings' ? "text-indigo-600 border-current" : "border-transparent text-slate-500 hover:text-slate-800")}>
            管家设置
          </button>
        </div>
        <button onClick={() => setTab(null)} className="pb-3.5 text-slate-400 hover:text-slate-700 transition-colors">
          <X size={18} />
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-6 scroll-smooth">
        {currentTab === 'dashboard' && <DashboardTab roles={roles} onViewChange={onViewChange} />}
        {currentTab === 'tasks' && (
          <div className="space-y-6">
            <div>
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">通用任务</h3>
                <button className="hover:bg-slate-200 p-1.5 rounded-md transition-colors text-indigo-600"><Plus size={18}/></button>
              </div>
              <div className="space-y-2.5">
                <div className="bg-white border border-slate-200 rounded-xl p-4 flex gap-3.5 shadow-sm">
                  <button className="text-slate-300 hover:text-emerald-500 transition-colors shrink-0 mt-0.5"><Circle size={20} strokeWidth={2.5} /></button>
                  <div className="flex-1">
                    <p className="text-[14.5px] text-slate-800 font-medium leading-snug">整理本周日程表</p>
                    <div className="flex items-center gap-2 mt-2.5">
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-slate-100 text-slate-600 border border-slate-200">通用</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
        {currentTab === 'memory' && <MemoryTab roleId={null} includeRoleMemories />}
        {currentTab === 'settings' && <ButlerSettingsContent archivedRoles={archivedRoles} onRestoreRole={onRestoreRole} />}
      </div>
    </div>
  );
}
