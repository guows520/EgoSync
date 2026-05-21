import { useState } from 'react';
import { Home, BarChart2, ListTodo, BrainCircuit, Sliders, Play } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ActionCard } from './ActionCard';
import { ButlerWorkspacePanel } from './ButlerWorkspacePanel';

export function ButlerView({ roles, onViewChange, archivedRoles, onRestoreRole }: any) {
  const [input, setInput] = useState('');
  const [openTab, setOpenTab] = useState<'dashboard' | 'tasks' | 'memory' | 'settings' | null>(null);
  const [messages, setMessages] = useState<{sender:string, text:string}[]>([]);
  const handleSend = () => {
    if (!input.trim()) return;
    const newMsgs = [...messages, {sender: 'user', text: input}];
    setMessages(newMsgs);
    setInput('');
    setTimeout(() => {
      setMessages([...newMsgs, {sender: 'butler', text: '好的，我来处理这件事。稍后给你更新进展。'}]);
    }, 1000);
  };

  const toggleTab = (tab: 'dashboard' | 'tasks' | 'memory' | 'settings') => {
    setOpenTab(prev => prev === tab ? null : tab);
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
          <div className="flex-1 overflow-y-auto p-8 scroll-smooth">
            <div className={cn("mx-auto space-y-6 transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              <div className="flex flex-col items-start gap-1.5 animate-in slide-in-from-bottom-2">
                <div className="bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 shadow-sm rounded-2xl rounded-tl-sm p-6 w-full text-[14.5px] leading-[1.7] text-slate-700 dark:text-slate-200">
                  <p className="mb-5">早上好，boss。今天最重要的一件事是下午3点的产品评审。
                  <button onClick={() => onViewChange('pm')} className="text-indigo-600 font-medium underline decoration-dotted underline-offset-4 hover:decoration-solid px-1 transition-all">产品经理</button>
                  已经准备好了演示文稿摘要，你可以点进去查看。另外，
                  <button onClick={() => onViewChange('family')} className="text-amber-600 font-medium underline decoration-dotted underline-offset-4 hover:decoration-solid px-1 transition-all">家庭角色</button>
                  想跟你聊一下周末的活动安排。</p>
                  
                  <div className="space-y-3">
                    <ActionCard icon="📋" title="产品评审准备就绪" meta="产品经理 · 2分钟前" primaryBtn="查看" onPrimary={() => onViewChange('pm')} />
                    <ActionCard icon="💬" title="周末活动建议" meta="家庭 · 希望确认" primaryBtn="聊聊" onPrimary={() => onViewChange('family')} />
                  </div>
                </div>
              </div>

              {/* User messages */}
              {messages.map((msg, i) => (
                <div key={i} className={cn("flex flex-col gap-1.5 animate-in slide-in-from-bottom-2", msg.sender === 'butler' ? "items-start" : "items-end")}>
                  <div className={cn(
                    "rounded-2xl p-6 max-w-[85%] text-[14.5px] leading-[1.7] shadow-sm",
                    msg.sender === 'butler' ? "bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 rounded-tl-sm text-slate-700 dark:text-slate-200" : "bg-slate-800 dark:bg-indigo-600 text-white rounded-tr-sm"
                  )}>
                    {msg.text}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Input Area */}
          <div className="p-5 bg-white/70 dark:bg-slate-800/70 backdrop-blur-md border-t border-slate-200/60 dark:border-slate-700/60 shrink-0">
            <div className={cn("relative mx-auto transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              <input 
                type="text" 
                value={input}
                onChange={e => setInput(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSend()}
                placeholder="跟管家说点什么，比如：帮我安排一个会议..." 
                className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-500/20 focus:border-slate-500 transition-all shadow-sm" 
              />
              <button 
                onClick={handleSend}
                className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700"
              >
                <Play size={16} className="ml-0.5" fill="currentColor" />
              </button>
            </div>
          </div>
        </div>

        {/* Workspace Panel */}
        {openTab && (
          <div className="w-[40%] bg-slate-50/60 dark:bg-slate-800/60 flex flex-col backdrop-blur-sm border-l border-white/40 dark:border-slate-700/40 shadow-[-8px_0_24px_rgba(0,0,0,0.02)] animate-in slide-in-from-right-8 duration-300">
            <ButlerWorkspacePanel roles={roles} currentTab={openTab} setTab={setOpenTab} archivedRoles={archivedRoles} onRestoreRole={onRestoreRole} onViewChange={onViewChange} />
          </div>
        )}
      </div>
    </div>
  );
}
