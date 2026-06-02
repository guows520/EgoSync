import { useEffect, useState } from 'react';
import { Home, BarChart2, ListTodo, BrainCircuit, Sliders } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ButlerWorkspacePanel } from './ButlerWorkspacePanel';
import { ChatStream } from '../chat/ChatStream';
import type { SourceNavigationTarget } from '../../types/chat';

export function ButlerView({ roles, onViewChange, archivedRoles, onRestoreRole, onRoleSourceNavigation, sourceNavigationTarget: externalSourceNavigationTarget, onSourceNavigationHandled }: any) {
  const [openTab, setOpenTab] = useState<'dashboard' | 'tasks' | 'memory' | 'settings' | null>(null);
  const [targetMemoryId, setTargetMemoryId] = useState<string | null>(null);
  const [sourceNavigationTarget, setSourceNavigationTarget] = useState<SourceNavigationTarget | null>(null);

  useEffect(() => {
    if (!externalSourceNavigationTarget) return;
    setSourceNavigationTarget(externalSourceNavigationTarget);
  }, [externalSourceNavigationTarget]);

  const toggleTab = (tab: 'dashboard' | 'tasks' | 'memory' | 'settings') => {
    setOpenTab(prev => prev === tab ? null : tab);
  };

  const handleMemoryReferenceClick = (memoryId: string) => {
    setTargetMemoryId(memoryId);
    setOpenTab('memory');
  };

  const handleSourceMessageClick = (target: SourceNavigationTarget) => {
    if (target.roleId) {
      onRoleSourceNavigation?.(target);
      return;
    }
    setSourceNavigationTarget(target);
  };

  return (
    <div className="h-full flex flex-col relative bg-white/40 dark:bg-slate-900/40 animate-in fade-in duration-500">
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-8 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">数字管家</h2>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => toggleTab('dashboard')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'dashboard' ? "bg-white shadow-sm text-indigo-600" : "text-slate-500 hover:bg-white/50")}>
            <BarChart2 size={16} /> 仪表盘
          </button>
          <button onClick={() => toggleTab('tasks')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'tasks' ? "bg-white shadow-sm text-indigo-600" : "text-slate-500 hover:bg-white/50")}>
            <ListTodo size={16} /> 任务
          </button>
          <button onClick={() => toggleTab('memory')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'memory' ? "bg-white shadow-sm text-indigo-600" : "text-slate-500 hover:bg-white/50")}>
            <BrainCircuit size={16} /> 记忆
          </button>
          <button onClick={() => toggleTab('settings')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'settings' ? "bg-white shadow-sm text-indigo-600" : "text-slate-500 hover:bg-white/50")}>
            <Sliders size={16} /> 设置
          </button>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Chat Area */}
        <div className={cn("flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out", openTab ? "w-[60%] border-r border-slate-200/60 dark:border-slate-700/60" : "w-full")}>
          <ChatStream
            role={null}
            onMemoryReferenceClick={handleMemoryReferenceClick}
            sourceNavigationTarget={sourceNavigationTarget}
            onSourceNavigationHandled={() => {
              setSourceNavigationTarget(null);
              onSourceNavigationHandled?.();
            }}
          />
        </div>

        {/* Workspace Panel */}
        {openTab && (
          <div className="w-[40%] bg-slate-50/60 dark:bg-slate-800/60 flex flex-col backdrop-blur-sm border-l border-white/40 dark:border-slate-700/40 shadow-[-8px_0_24px_rgba(0,0,0,0.02)] animate-in slide-in-from-right-8 duration-300">
            <ButlerWorkspacePanel
              roles={roles}
              currentTab={openTab}
              setTab={setOpenTab}
              archivedRoles={archivedRoles}
              onRestoreRole={onRestoreRole}
              onViewChange={onViewChange}
              targetMemoryId={targetMemoryId}
              onTargetMemoryHandled={() => setTargetMemoryId(null)}
              onSourceMessageClick={handleSourceMessageClick}
            />
          </div>
        )}
      </div>
    </div>
  );
}
