import { useState, useEffect } from 'react';
import { ListTodo, BrainCircuit, Sliders, Play, CheckCircle2, Check } from 'lucide-react';
import { cn } from '../../lib/utils';
import { RoleWorkspacePanel } from './RoleWorkspacePanel';

export function RoleView({ role, onOpenTask, initialTab, onTabConsumed, onUpdateRole }: any) {
  const [openTab, setOpenTab] = useState<'tasks' | 'memory' | 'settings' | null>(initialTab || null);
  useEffect(() => {
    if (initialTab && openTab !== initialTab) {
      setOpenTab(initialTab);
      onTabConsumed?.();
    }
  }, [initialTab]);
  const [input, setInput] = useState('');
  const [messages, setMessages] = useState<{sender:string, text:string, type?: 'suggestion', status?: 'pending'|'accepted'|'rejected'}[]>([
    {sender: 'role', text: 'boss，关于周末的科技馆活动我已经规划好了路线。你需要我现在把详情发给你吗？'},
    {sender: 'role', text: '我建议把「准备Q3 OKR规划」加入本周大石头，这样可以保证足够的专注时间。', type: 'suggestion', status: 'pending'}
  ]);

  const handleSuggestion = (index: number, action: 'accepted' | 'rejected') => {
    setMessages(prev => prev.map((m, i) => i === index ? {...m, status: action} : m));
  };

  const handleSend = () => {
    if (!input.trim()) return;
    const newMsgs = [...messages, {sender: 'user', text: input}];
    setMessages(newMsgs);
    setInput('');
    setTimeout(() => {
      setMessages([...newMsgs, {sender: 'role', text: '好的，我来处理这个事情，稍后给你反馈。'}]);
    }, 1000);
  };

  const toggleTab = (tab: 'tasks' | 'memory' | 'settings') => {
    setOpenTab(prev => prev === tab ? null : tab);
  };

  return (
    <div className={cn("h-full flex flex-col transition-colors duration-500", role.tint)}>
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-8 gap-4 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className={cn("w-[46px] h-[46px] rounded-xl flex items-center justify-center text-white shadow-sm", role.color)}>
          <role.icon size={24} strokeWidth={2} />
        </div>
        <div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">{role.name}</h2>
          <div className="flex items-center gap-3 mt-1.5">
            <div className="w-[120px] h-[6px] bg-slate-200/80 rounded-full overflow-hidden">
              <div className="h-full bg-emerald-500 rounded-full transition-all duration-1000" style={{ width: `${role.energy}%` }} />
            </div>
            <span className="text-[11px] text-slate-500 font-medium">{role.energy}% 能量</span>
          </div>
        </div>

        {/* Right-aligned Panel Toggles */}
        <div className="ml-auto flex items-center gap-2">
          <button onClick={() => toggleTab('tasks')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'tasks' ? `bg-white shadow-sm ${role.text}` : "text-slate-500 hover:bg-white/50")}>
            <ListTodo size={16} /> 任务
          </button>
          <button onClick={() => toggleTab('memory')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'memory' ? `bg-white shadow-sm ${role.text}` : "text-slate-500 hover:bg-white/50")}>
            <BrainCircuit size={16} /> 记忆
          </button>
          <button onClick={() => toggleTab('settings')} className={cn("flex items-center gap-1.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all", openTab === 'settings' ? `bg-white shadow-sm ${role.text}` : "text-slate-500 hover:bg-white/50")}>
            <Sliders size={16} /> 设置
          </button>
        </div>
      </header>

      {/* Body: Chat Stream & Optional Workspace Panel */}
      <div className="flex-1 flex overflow-hidden">
        {/* Chat Stream (Dynamic Width) */}
        <div className={cn("flex flex-col relative bg-white/40 dark:bg-slate-900/40 transition-all duration-500 ease-in-out", openTab ? "w-[60%] border-r border-slate-200/60 dark:border-slate-700/60" : "w-full")}>
          <div className="flex-1 overflow-y-auto p-8">
            <div className={cn("mx-auto space-y-6 transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              {messages.map((msg, i) => (
                <div key={i} className={cn("flex flex-col gap-1.5 animate-in slide-in-from-bottom-2", msg.sender === 'role' ? "items-start" : "items-end")}>
                  {msg.type === 'suggestion' ? (
                    <div className={cn(
                      "rounded-2xl max-w-[85%] shadow-sm border-l-4 overflow-hidden transition-all",
                      msg.status === 'accepted' ? "border-l-emerald-500 bg-emerald-50 dark:bg-emerald-900/20" : msg.status === 'rejected' ? "border-l-slate-300 bg-slate-50 dark:bg-slate-800/50 opacity-60" : `border-l-indigo-500 bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700`
                    )}>
                      <div className="p-5">
                        <div className="flex items-center gap-2 mb-2">
                          <span className="text-[11px] font-bold tracking-wider uppercase text-indigo-500">建议</span>
                          {msg.status === 'accepted' && <span className="text-[11px] font-bold text-emerald-600 flex items-center gap-1"><CheckCircle2 size={12} /> 已采纳</span>}
                          {msg.status === 'rejected' && <span className="text-[11px] font-bold text-slate-400">已忽略</span>}
                        </div>
                        <p className="text-[14px] leading-[1.7] text-slate-700 dark:text-slate-200">{msg.text}</p>
                        {msg.status === 'pending' && (
                          <div className="flex gap-2 mt-3.5">
                            <button onClick={() => handleSuggestion(i, 'accepted')} className="px-4 py-2 rounded-lg text-[13px] font-medium bg-indigo-600 text-white hover:bg-indigo-700 transition-colors flex items-center gap-1.5"><Check size={14} /> 采纳</button>
                            <button onClick={() => handleSuggestion(i, 'rejected')} className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-500 hover:text-slate-900 hover:bg-slate-100 transition-colors">忽略</button>
                          </div>
                        )}
                      </div>
                    </div>
                  ) : (
                    <div className={cn(
                      "rounded-2xl p-6 max-w-[85%] text-[14.5px] leading-[1.7] shadow-sm",
                      msg.sender === 'role' ? "bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 rounded-tl-sm text-slate-700 dark:text-slate-200" : `${role.color} text-white rounded-tr-sm`
                    )}>
                      {msg.text}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
          <div className="p-5 bg-white/70 dark:bg-slate-800/70 backdrop-blur-md border-t border-slate-200/60 dark:border-slate-700/60 shrink-0">
            <div className={cn("relative mx-auto transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              <input 
                type="text" 
                value={input}
                onChange={e => setInput(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSend()}
                placeholder={`跟 ${role.name} 说点什么...`} 
                className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all shadow-sm" 
              />
              <button onClick={handleSend} className={cn("absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white rounded-lg transition-colors shadow-sm", role.color, "hover:brightness-110")}>
                <Play size={16} className="ml-0.5" fill="currentColor" />
              </button>
            </div>
          </div>
        </div>

        {/* Workspace Panel */}
        {openTab && (
          <div className="w-[40%] bg-slate-50/60 dark:bg-slate-800/60 flex flex-col backdrop-blur-sm border-l border-white/40 dark:border-slate-700/40 shadow-[-8px_0_24px_rgba(0,0,0,0.02)] animate-in slide-in-from-right-8 duration-300">
            <RoleWorkspacePanel role={role} currentTab={openTab} setTab={setOpenTab} onOpenTask={onOpenTask} onUpdateRole={onUpdateRole} />
          </div>
        )}
      </div>
    </div>
  );
}
