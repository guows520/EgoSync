import { useEffect, useState } from 'react';
import { Home, BarChart2, ListTodo, BrainCircuit, Sliders } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ButlerWorkspacePanel } from './ButlerWorkspacePanel';
import { ChatStream } from '../chat/ChatStream';
import { useSuggestions } from '../../hooks/useSuggestions';
import { useTaskDecompositions } from '../../hooks/useTaskDecompositions';
import { ActionCard } from './ActionCard';
import { taskService } from '../../services/taskService';
import type { SourceNavigationTarget } from '../../types/chat';
import type { Task, TaskActions } from '../../types/task';
import type { TaskScope } from '../../hooks/useTasks';
import type { NotificationWithRole } from '../../types/notification';
import type { SuggestionWithRole } from '../../types/suggestion';

/// Story 4.5: 将敲门通知映射为 ActionCard 可消费的 SuggestionWithRole 结构（复用建议卡片组件）。
function knockToSuggestion(n: NotificationWithRole): SuggestionWithRole {
  return {
    id: n.id,
    roleId: n.roleId,
    title: n.content,
    content: '',
    priority: 'high',
    status: 'pending',
    rejectionReason: null,
    convertedTaskId: null,
    conversationId: null,
    createdAt: n.createdAt,
    roleName: n.roleName,
    roleIcon: n.roleIcon,
    roleColor: n.roleColor,
  };
}

export function ButlerView({ roles, onViewChange, archivedRoles, onRestoreRole, onUpdateRole, onRoleSourceNavigation, sourceNavigationTarget: externalSourceNavigationTarget, onSourceNavigationHandled, onOpenTask, onTasksApiReady, knockNotifications, onDismissKnock, chatRefreshTrigger }: {
  roles: any[];
  onViewChange: (view: string) => void;
  archivedRoles: any[];
  onRestoreRole: (id: string) => Promise<void> | void;
  onUpdateRole: (role: any) => void;
  onRoleSourceNavigation: (target: SourceNavigationTarget) => void;
  sourceNavigationTarget: SourceNavigationTarget | null;
  onSourceNavigationHandled: () => void;
  onOpenTask: (scope: TaskScope, task?: Task | null) => void;
  onTasksApiReady: (key: string, actions: TaskActions) => void;
  knockNotifications: NotificationWithRole[];
  onDismissKnock: (id: string) => void;
  chatRefreshTrigger: number;
}) {
  const [openTab, setOpenTab] = useState<'dashboard' | 'tasks' | 'memory' | 'settings' | null>(null);
  const [targetMemoryId, setTargetMemoryId] = useState<string | null>(null);
  const [sourceNavigationTarget, setSourceNavigationTarget] = useState<SourceNavigationTarget | null>(null);
  const [butlerConversationId, setButlerConversationId] = useState<string | null>(null);
  const { suggestions, confirmSuggestion, rejectSuggestion, removeSuggestion } = useSuggestions(butlerConversationId);
  const { proposals, refetch: refetchTaskDecompositions, acceptProposal, keepSingleProposal } = useTaskDecompositions(butlerConversationId);

  useEffect(() => {
    if (!externalSourceNavigationTarget) return;
    setSourceNavigationTarget(externalSourceNavigationTarget);
  }, [externalSourceNavigationTarget]);

  // Story 4.6: 打开管家视角时主动触发一次 Q2 保护提醒检查
  useEffect(() => {
    taskService.checkQ2Reminders().catch((e: unknown) => {
      console.warn('Q2 保护提醒检查失败:', e);
    });
  }, []);

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
      {/* Header — Story 16.2 响应式基线：小屏 px-4 + 高度自适应（tab 组可换行），
          ≥px-8 与 h-[76px] 维持桌面原样 */}
      <header className="h-auto md:h-[76px] py-2 md:py-0 border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex flex-wrap items-center px-4 md:px-8 gap-2 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-3 md:gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">数字分身管家</h2>
        </div>
        <div className="flex items-center gap-1.5 md:gap-2">
          <button onClick={() => toggleTab('dashboard')} className={cn("flex items-center gap-1.5 px-2.5 md:px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'dashboard' ? "bg-white dark:bg-slate-700 shadow-sm text-indigo-600 dark:text-indigo-300" : "text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60")}>
            <BarChart2 size={16} /> 仪表盘
          </button>
          <button onClick={() => toggleTab('tasks')} className={cn("flex items-center gap-1.5 px-2.5 md:px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'tasks' ? "bg-white dark:bg-slate-700 shadow-sm text-indigo-600 dark:text-indigo-300" : "text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60")}>
            <ListTodo size={16} /> 任务
          </button>
          <button onClick={() => toggleTab('memory')} className={cn("flex items-center gap-1.5 px-2.5 md:px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'memory' ? "bg-white dark:bg-slate-700 shadow-sm text-indigo-600 dark:text-indigo-300" : "text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60")}>
            <BrainCircuit size={16} /> 记忆
          </button>
          <button onClick={() => toggleTab('settings')} className={cn("flex items-center gap-1.5 px-2.5 md:px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'settings' ? "bg-white dark:bg-slate-700 shadow-sm text-indigo-600 dark:text-indigo-300" : "text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60")}>
            <Sliders size={16} /> 设置
          </button>
        </div>
      </header>

      {/* Story 16.2 响应式基线：≥md 维持 65/35 双栏；小屏（375px 级）
          flex-col 堆叠——对话区 58% + 工作区 42%（断点方案归 UX 勘注定稿）。 */}
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        {/* Chat Area */}
        <div className={cn(
          "flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out min-h-0",
          openTab ? "w-full h-[58%] md:h-auto md:w-[65%] border-b md:border-b-0 md:border-r border-slate-200/60 dark:border-slate-700/60" : "w-full h-full",
        )}>
          {knockNotifications.length > 0 && (
            <div className="px-4 pt-4 space-y-3 shrink-0">
              {knockNotifications.map(n => (
                <ActionCard
                  key={n.id}
                  suggestion={knockToSuggestion(n)}
                  onConfirm={() => {}}
                  onReject={() => {}}
                  onDismiss={onDismissKnock}
                  confirmLabel="立即处理"
                  rejectLabel="稍后"
                  directRejectReason="later"
                  hideDescription
                />
              ))}
            </div>
          )}
          <ChatStream
            role={null}
            onMemoryReferenceClick={handleMemoryReferenceClick}
            sourceNavigationTarget={sourceNavigationTarget}
            onSourceNavigationHandled={() => {
              setSourceNavigationTarget(null);
              onSourceNavigationHandled?.();
            }}
            suggestions={suggestions}
            onConfirmSuggestion={confirmSuggestion}
            onRejectSuggestion={rejectSuggestion}
            onDismissSuggestion={removeSuggestion}
            taskDecompositions={proposals}
            onAcceptTaskDecomposition={acceptProposal}
            onKeepSingleTaskDecomposition={keepSingleProposal}
            onStreamDone={refetchTaskDecompositions}
            refreshTrigger={chatRefreshTrigger}
            onConversationIdChange={setButlerConversationId}
          />
        </div>

        {/* Workspace Panel */}
        {openTab && (
          <div className="w-full h-[42%] md:h-auto md:w-[35%] bg-white dark:bg-slate-900 flex flex-col border-t md:border-t-0 md:border-l border-slate-200/60 dark:border-slate-700/60 animate-in slide-in-from-right-8 duration-300 min-h-0">
            <ButlerWorkspacePanel
              roles={roles}
              currentTab={openTab}
              setTab={setOpenTab}
              archivedRoles={archivedRoles}
              onRestoreRole={onRestoreRole}
              onUpdateRole={onUpdateRole}
              onViewChange={onViewChange}
              targetMemoryId={targetMemoryId}
              onTargetMemoryHandled={() => setTargetMemoryId(null)}
              onSourceMessageClick={handleSourceMessageClick}
              onOpenTask={onOpenTask}
              onTasksApiReady={onTasksApiReady}
            />
          </div>
        )}
      </div>
    </div>
  );
}
