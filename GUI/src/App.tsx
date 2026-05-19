import { useState } from 'react';
import {
  Home, Briefcase, Heart, BookOpen, Circle, Clock, ListTodo, BrainCircuit, 
  Sliders, Play, Plus, GripVertical, Settings as SettingsIcon, X, BarChart2, 
  Download, Trash2, CheckCircle2, Target, AlertTriangle, Check, ArrowRight
} from 'lucide-react';
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const ROLES = [
  { id: 'pm', name: '产品经理', icon: Briefcase, color: 'bg-indigo-600', text: 'text-indigo-600', tint: 'bg-indigo-50/40', energy: 85, status: 'green' },
  { id: 'family', name: '家庭', icon: Heart, color: 'bg-amber-600', text: 'text-amber-600', tint: 'bg-amber-50/40', energy: 60, status: 'yellow' },
  { id: 'study', name: '学习者', icon: BookOpen, color: 'bg-purple-600', text: 'text-purple-600', tint: 'bg-purple-50/40', energy: 70, status: 'none' }
];

export default function App() {
  const [scenario, setScenario] = useState<'daily' | 'onboard' | 'conflict' | 'review'>('daily');
  const [currentView, setCurrentView] = useState('butler');
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isArbOpen, setIsArbOpen] = useState(false);
  const [isReviewOpen, setIsReviewOpen] = useState(false);
  const [isTaskModalOpen, setIsTaskModalOpen] = useState(false);

  // Switch scenario handler
  const handleScenarioSwitch = (s: any) => {
    setScenario(s);
    if (s === 'onboard') {
      setCurrentView('onboard');
    } else {
      setCurrentView('butler');
      if (s === 'conflict') setIsArbOpen(true);
      if (s === 'review') setIsReviewOpen(true);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-[#F8F9FA] text-slate-900 font-sans selection:bg-indigo-100">
      {/* PITCH SCENARIO BAR (Fixed Top) */}
      <div className="h-12 bg-slate-900 text-slate-200 flex items-center px-6 justify-between shrink-0 z-50">
        <div className="flex items-center gap-4">
          <span className="text-[11px] font-bold tracking-widest uppercase text-indigo-400">Pitch Mode</span>
          <div className="flex bg-slate-800 rounded-lg p-1 gap-1">
            {(['onboard', 'daily', 'conflict', 'review'] as const).map(s => (
              <button
                key={s}
                onClick={() => handleScenarioSwitch(s)}
                className={cn(
                  "px-3 py-1 rounded-md text-[12px] font-medium transition-all",
                  scenario === s ? "bg-indigo-500 text-white shadow-sm" : "text-slate-400 hover:text-slate-200 hover:bg-slate-700"
                )}
              >
                {s === 'onboard' && '1. 冷启动'}
                {s === 'daily' && '2. 晨间简报'}
                {s === 'conflict' && '3. 冲突仲裁'}
                {s === 'review' && '4. 周复盘'}
              </button>
            ))}
          </div>
        </div>
        <div className="text-[12px] text-slate-500 flex items-center gap-2">
          <span>EgoSync V1 High-Fidelity Prototype</span>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden relative">
        <Sidebar currentView={currentView} onViewChange={setCurrentView} onOpenSettings={() => setIsSettingsOpen(true)} />
        
        <main className="flex-1 relative overflow-hidden bg-white/30 backdrop-blur-3xl shadow-[inset_1px_0_10px_rgba(0,0,0,0.02)]">
          {currentView === 'onboard' && <OnboardingView onComplete={() => handleScenarioSwitch('daily')} />}
          {currentView === 'butler' && <ButlerView scenario={scenario} onViewChange={setCurrentView} onOpenArb={() => setIsArbOpen(true)} onOpenReview={() => setIsReviewOpen(true)} />}
          {ROLES.map(r => r.id === currentView && <RoleView key={r.id} roleId={currentView} onOpenTask={() => setIsTaskModalOpen(true)} />)}
        </main>
      </div>

      {/* OVERLAYS */}
      {isSettingsOpen && <GlobalSettingsModal onClose={() => setIsSettingsOpen(false)} />}
      {isArbOpen && <ArbitrationModal onClose={() => setIsArbOpen(false)} />}
      {isReviewOpen && <WeeklyReviewModal onClose={() => setIsReviewOpen(false)} />}
      {isTaskModalOpen && <TaskModal onClose={() => setIsTaskModalOpen(false)} />}
    </div>
  );
}

/* ========================================================================== 
   SIDEBAR
   ========================================================================== */
function Sidebar({ currentView, onViewChange, onOpenSettings }: any) {
  return (
    <aside className="w-16 bg-[#F1F3F5] border-r border-slate-200 flex flex-col items-center py-6 z-10 shrink-0 shadow-sm relative">
      <div className="flex-1 flex flex-col items-center gap-4 w-full">
        <button
          onClick={() => onViewChange('butler')}
          className={cn("w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-200", currentView === 'butler' ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200 scale-105" : "text-slate-500 hover:bg-slate-200")}
        >
          <Home size={22} strokeWidth={2.5} />
        </button>
        <div className="w-6 border-b border-slate-300 my-2"></div>
        {ROLES.map(role => (
          <button
            key={role.id}
            onClick={() => onViewChange(role.id)}
            className={cn(
              "relative w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-200",
              currentView === role.id ? `${role.color} text-white shadow-lg scale-105` : "text-slate-500 hover:bg-slate-200",
              role.status !== 'none' && currentView !== role.id && "breathe"
            )}
          >
            <role.icon size={22} strokeWidth={2.5} />
            {role.status === 'green' && <span className="absolute -top-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-[#F1F3F5] bg-emerald-500 shadow-sm"></span>}
            {role.status === 'yellow' && <span className="absolute -top-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-[#F1F3F5] bg-amber-500 shadow-sm"></span>}
          </button>
        ))}
      </div>
      
      <button onClick={onOpenSettings} className="w-11 h-11 rounded-xl flex items-center justify-center text-slate-400 hover:bg-slate-200 hover:text-slate-700 transition-colors mt-auto">
        <SettingsIcon size={22} />
      </button>
    </aside>
  );
}

/* ========================================================================== 
   VIEWS
   ========================================================================== */
function ButlerView({ scenario, onViewChange, onOpenArb, onOpenReview }: any) {
  const [input, setInput] = useState('');
  const handleSend = () => {
    if (!input.trim()) return;
    setInput('');
    // Placeholder for send action
  };

  return (
    <div className="h-full flex flex-col relative bg-white/40 animate-in fade-in duration-500">
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 bg-white/60 backdrop-blur-md flex items-center px-8 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800">数字管家</h2>
        </div>
      </header>

      {/* Chat Area */}
      <div className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="max-w-3xl mx-auto space-y-6">
          <div className="flex flex-col items-start gap-1.5 animate-in slide-in-from-bottom-2">
            <span className="text-[11px] font-medium text-slate-400 ml-1 uppercase tracking-wider">管家</span>
            <div className="bg-white border border-slate-200/80 shadow-sm rounded-2xl rounded-tl-sm p-6 w-full text-[14.5px] leading-[1.7] text-slate-700">
              
              {scenario === 'daily' && (
                <>
                  <p className="mb-5">早上好，boss。今天最重要的一件事是下午3点的产品评审。
                  <button onClick={() => onViewChange('pm')} className="text-indigo-600 font-medium underline decoration-dotted underline-offset-4 hover:decoration-solid px-1 transition-all">产品经理</button>
                  已经准备好了演示文稿摘要，你可以点进去查看。另外，
                  <button onClick={() => onViewChange('family')} className="text-amber-600 font-medium underline decoration-dotted underline-offset-4 hover:decoration-solid px-1 transition-all">家庭角色</button>
                  想跟你聊一下周末的活动安排。</p>
                  
                  <div className="space-y-3">
                    <ActionCard icon="📋" title="产品评审准备就绪" meta="产品经理 · 2分钟前" primaryBtn="查看" onPrimary={() => onViewChange('pm')} />
                    <ActionCard icon="💬" title="周末活动建议" meta="家庭 · 希望确认" primaryBtn="聊聊" onPrimary={() => onViewChange('family')} />
                  </div>
                </>
              )}

              {scenario === 'conflict' && (
                <>
                  <p className="mb-5">下午好，boss。我发现你的日程中存在一处冲突，需要你来定夺。</p>
                  <div className="bg-amber-50 border border-amber-200 rounded-xl p-5 flex items-center justify-between shadow-sm">
                    <div>
                      <h3 className="font-semibold text-sm flex items-center gap-2 text-amber-800">
                        <AlertTriangle size={16} /> 角色日程冲突
                      </h3>
                      <p className="text-[13px] text-amber-700/80 mt-1.5">产品经理 vs 家庭 · 周五 15:00</p>
                    </div>
                    <button onClick={onOpenArb} className="px-5 py-2.5 rounded-lg text-[13px] font-medium bg-amber-600 text-white hover:bg-amber-700 shadow-sm transition-colors">处理仲裁</button>
                  </div>
                </>
              )}

              {scenario === 'review' && (
                <>
                  <p className="mb-5">周末好，boss。这周过得很充实，我们一起来看看各个角色的进展吧？</p>
                  <button onClick={onOpenReview} className="px-6 py-3 rounded-xl text-[14px] font-medium bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm transition-all hover:-translate-y-0.5 flex items-center gap-2">
                    <BarChart2 size={18} /> 开始周复盘
                  </button>
                </>
              )}

            </div>
          </div>
        </div>
      </div>

      {/* Input Area */}
      <div className="p-5 bg-white/70 backdrop-blur-md border-t border-slate-200/60 shrink-0">
        <div className="relative max-w-3xl mx-auto">
          <input 
            type="text" 
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSend()}
            placeholder="跟管家说点什么，比如：帮我安排一个会议..." 
            className="w-full bg-white border border-slate-200 rounded-xl pl-5 pr-14 py-3.5 text-[14px] focus:outline-none focus:ring-2 focus:ring-slate-500/20 focus:border-slate-500 transition-all shadow-sm" 
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
  );
}

function ActionCard({ icon, title, meta, primaryBtn, onPrimary }: any) {
  const [done, setDone] = useState(false);
  if (done) return null;
  return (
    <div className="bg-white border border-slate-200 rounded-xl p-4 flex items-center justify-between hover:shadow-[0_4px_12px_rgba(0,0,0,0.05)] hover:-translate-y-0.5 transition-all duration-200">
      <div className="flex items-center gap-3">
        <span className="text-xl">{icon}</span>
        <div>
          <h3 className="font-medium text-[14px] text-slate-800">{title}</h3>
          <p className="text-[12px] text-slate-400 mt-1">{meta}</p>
        </div>
      </div>
      <div className="flex gap-2">
        <button onClick={() => setDone(true)} className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-500 hover:text-slate-900 hover:bg-slate-100 transition-colors">稍后</button>
        <button onClick={onPrimary} className="px-4 py-2 rounded-lg text-[13px] font-medium bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm transition-colors">{primaryBtn}</button>
      </div>
    </div>
  );
}

function OnboardingView({ onComplete }: any) {
  const [step, setStep] = useState(0);
  const [input, setInput] = useState('');
  const [chat, setChat] = useState<{sender:string, text:string}[]>([
    {sender: 'butler', text: '你好 👋 我是你的数字管家。聊聊你最近在忙什么？我来帮你安排。'}
  ]);

  const handleSend = () => {
    if (!input.trim()) return;
    const newChat = [...chat, {sender: 'user', text: input}];
    setChat(newChat);
    setInput('');
    
    if (step === 0) {
      setTimeout(() => {
        setChat([...newChat, {sender: 'butler', text: '听起来你工作中产品管理很重要。要不要先创建一个 **产品经理** 角色？'}]);
        setStep(1);
      }, 1000);
    } else if (step === 1) {
      setTimeout(() => {
        setChat([...newChat, {sender: 'butler', text: '✨ 产品经理角色已创建！好的，我来照顾这些方面。现在先熟悉一下环境吧。'}]);
        setTimeout(() => onComplete(), 2000);
      }, 1000);
    }
  };

  return (
    <div className="h-full flex flex-col relative bg-white/40">
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 bg-white/60 backdrop-blur-md flex items-center px-8 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800">数字管家</h2>
        </div>
      </header>
      
      {/* Chat Area */}
      <div className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="max-w-3xl mx-auto space-y-6">
          {chat.map((msg, i) => (
            <div key={i} className={cn("flex flex-col gap-1.5 animate-in slide-in-from-bottom-2", msg.sender === 'butler' ? "items-start" : "items-end")}>
              <span className="text-[11px] font-medium text-slate-400 uppercase tracking-wider mx-1">
                {msg.sender === 'butler' ? '管家' : 'boss'}
              </span>
              <div className={cn(
                "px-5 py-3.5 rounded-2xl max-w-[85%] text-[14px] leading-[1.7] shadow-sm",
                msg.sender === 'butler' ? "bg-white border border-slate-200/80 rounded-tl-sm text-slate-700" : "bg-indigo-600 text-white rounded-tr-sm shadow-indigo-200/50"
              )} dangerouslySetInnerHTML={{__html: msg.text.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')}} />
            </div>
          ))}
        </div>
      </div>

      {/* Input Area */}
      <div className="p-5 bg-white/70 backdrop-blur-md border-t border-slate-200/60 shrink-0">
        <div className="relative max-w-3xl mx-auto">
          <input 
            type="text" 
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSend()}
            placeholder={step === 0 ? "比如：我是一个产品经理，最近在忙新产品上线..." : "确认，或者告诉我你想调整..."} 
            className="w-full bg-white border border-slate-200 rounded-xl pl-5 pr-14 py-3.5 text-[14px] focus:outline-none focus:ring-2 focus:ring-slate-500/20 focus:border-slate-500 transition-all shadow-sm" 
          />
          <button onClick={handleSend} className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700">
            <Play size={16} className="ml-0.5" fill="currentColor" />
          </button>
        </div>
      </div>
    </div>
  );
}

function RoleView({ roleId, onOpenTask }: any) {
  const role = ROLES.find(r => r.id === roleId)!;
  const [openTab, setOpenTab] = useState<'tasks' | 'memory' | 'settings' | null>(null);

  const toggleTab = (tab: 'tasks' | 'memory' | 'settings') => {
    setOpenTab(prev => prev === tab ? null : tab);
  };

  return (
    <div className={cn("h-full flex flex-col transition-colors duration-500", role.tint)}>
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 bg-white/60 backdrop-blur-md flex items-center px-8 gap-4 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className={cn("w-[46px] h-[46px] rounded-xl flex items-center justify-center text-white shadow-sm", role.color)}>
          <role.icon size={24} strokeWidth={2} />
        </div>
        <div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800">{role.name}</h2>
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
        <div className={cn("flex flex-col relative bg-white/40 transition-all duration-500 ease-in-out", openTab ? "w-[60%] border-r border-slate-200/60" : "w-full")}>
          <div className="flex-1 overflow-y-auto p-8">
            <div className={cn("mx-auto space-y-6 transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              <div className="flex flex-col gap-1.5 items-start">
                <span className="text-[11px] font-medium text-slate-400 ml-1 uppercase tracking-wider">{role.name}</span>
                <div className="bg-white border border-slate-200/80 shadow-sm rounded-2xl rounded-tl-sm p-6 max-w-[85%] text-[14.5px] leading-[1.7] text-slate-700">
                  boss，关于周末的科技馆活动我已经规划好了路线。你需要我现在把详情发给你吗？
                </div>
              </div>
            </div>
          </div>
          <div className="p-5 bg-white/70 backdrop-blur-md border-t border-slate-200/60 shrink-0">
            <div className={cn("relative mx-auto transition-all duration-500", openTab ? "w-full" : "max-w-3xl")}>
              <input type="text" placeholder={`跟 ${role.name} 说点什么...`} className="w-full bg-white border border-slate-200 rounded-xl pl-5 pr-14 py-3.5 text-[14px] focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 transition-all shadow-sm" />
              <button className={cn("absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white rounded-lg transition-colors shadow-sm", role.color, "hover:brightness-110")}>
                <Play size={16} className="ml-0.5" fill="currentColor" />
              </button>
            </div>
          </div>
        </div>

        {/* Workspace Panel */}
        {openTab && (
          <div className="w-[40%] bg-slate-50/60 flex flex-col backdrop-blur-sm border-l border-white/40 shadow-[-8px_0_24px_rgba(0,0,0,0.02)] animate-in slide-in-from-right-8 duration-300">
            <RoleWorkspacePanel role={role} currentTab={openTab} setTab={setOpenTab} onOpenTask={onOpenTask} />
          </div>
        )}
      </div>
    </div>
  );
}

function RoleWorkspacePanel({ role, currentTab, setTab, onOpenTask }: any) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b border-slate-200/80 px-6 pt-4 bg-white/60 backdrop-blur-md shrink-0">
        <div className="flex gap-8">
          <button onClick={() => setTab('tasks')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'tasks' ? `${role.text} border-current` : "border-transparent text-slate-500 hover:text-slate-800")}>
            任务清单
          </button>
          <button onClick={() => setTab('memory')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'memory' ? `${role.text} border-current` : "border-transparent text-slate-500 hover:text-slate-800")}>
            记忆档案
          </button>
          <button onClick={() => setTab('settings')} className={cn("pb-3.5 text-[14px] font-medium transition-colors border-b-[3px]", currentTab === 'settings' ? `${role.text} border-current` : "border-transparent text-slate-500 hover:text-slate-800")}>
            设置
          </button>
        </div>
        <button onClick={() => setTab(null)} className="pb-3.5 text-slate-400 hover:text-slate-700 transition-colors">
          <X size={18} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-6 scroll-smooth">
        {currentTab === 'tasks' && <TasksTab role={role} onOpenTask={onOpenTask} />}
        {currentTab === 'memory' && <MemoryTab />}
        {currentTab === 'settings' && <SettingsTab />}
      </div>
    </div>
  );
}

function TasksTab({ role, onOpenTask }: any) {
  return (
    <div className="space-y-6">
      <div>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">Q1 · 重要且紧急</h3>
          <button onClick={onOpenTask} className={cn("hover:bg-slate-200 p-1.5 rounded-md transition-colors", role.text)}><Plus size={18}/></button>
        </div>
        <div className="space-y-2.5">
          <div className="bg-white border border-slate-200 rounded-xl p-4 flex gap-3.5 shadow-sm group cursor-pointer hover:border-indigo-300 hover:shadow-md transition-all">
            <GripVertical size={18} className="text-slate-300 shrink-0 mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab" />
            <button className="text-slate-300 hover:text-emerald-500 transition-colors shrink-0 mt-0.5"><Circle size={20} strokeWidth={2.5} /></button>
            <div className="flex-1">
              <p className="text-[14.5px] text-slate-800 font-medium leading-snug">跟进核心OKR指标</p>
              <div className="flex items-center gap-2 mt-2.5">
                <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-red-50 text-red-600 border border-red-100">今日 18:00</span>
                <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-amber-50 text-amber-600 border border-amber-100">大石头</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function MemoryTab() {
  return (
    <div className="space-y-4">
      <div className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm hover:shadow-md transition-shadow">
        <div className="flex justify-between items-start mb-3">
          <h4 className="text-[15px] font-medium text-slate-800 flex items-center gap-2">
            <span className="text-indigo-500">🧠</span> 数据驱动偏好
          </h4>
          <button className="text-[12px] text-red-500 hover:bg-red-50 hover:border-red-200 border border-transparent px-2.5 py-1 rounded-md transition-colors">遗忘</button>
        </div>
        <p className="text-[14px] text-slate-600 leading-relaxed mb-4">boss在评估决策时，非常看重定量数据和收益模型，不喜欢纯主观体验的描述。</p>
        <div className="bg-slate-50 border border-slate-100 rounded-md p-2.5 text-[12px] text-slate-500 font-mono flex items-center gap-2">
          <Clock size={14} /> 引用于 2026-05-18 10:20 对话记录
        </div>
      </div>
    </div>
  );
}

function SettingsTab() {
  return (
    <div className="space-y-8">
      <div>
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">主动性级别</label>
        <div className="bg-slate-100/80 rounded-xl p-1.5 flex shadow-inner">
          <button className="flex-1 py-2 text-[13px] font-medium rounded-lg text-slate-500 hover:text-slate-800 transition-colors">仅被动执行</button>
          <button className="flex-1 py-2 text-[13px] font-medium rounded-lg bg-white shadow-sm text-indigo-600 border border-slate-200/50">主动建议</button>
        </div>
        <p className="text-[12px] text-slate-400 mt-2.5 leading-relaxed">角色会定时审视目标，主动生成待处理建议卡片。</p>
      </div>

      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">Skill 插件配置</label>
        <div className="space-y-3">
          <div className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2.5">
                <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]"></span>
                <span className="text-[14.5px] font-medium text-slate-800">Web Search API</span>
              </div>
              <span className="text-[12px] font-medium text-emerald-600 bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-100">已启用</span>
            </div>
            <input type="password" value="sk-xxxx-xxxx-xxxx-xxxx" readOnly className="w-full text-[13px] bg-slate-50 border border-slate-200 rounded-lg px-3 py-2 text-slate-500 focus:outline-none font-mono" />
          </div>
          <button className="w-full py-3 border border-dashed border-slate-300 rounded-xl text-[13px] font-medium text-slate-500 hover:text-indigo-600 hover:border-indigo-300 hover:bg-indigo-50/50 transition-all flex items-center justify-center gap-1.5">
            <Plus size={16}/> 添加新 Skill
          </button>
        </div>
      </div>
    </div>
  );
}

/* ========================================================================== 
   MODALS / OVERLAYS
   ========================================================================== */

function Modal({ children, onClose, width = "w-[540px]" }: any) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/20 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="absolute inset-0" onClick={onClose}></div>
      <div className={cn("bg-white rounded-2xl shadow-2xl overflow-hidden relative z-10 animate-in zoom-in-95 duration-200", width)}>
        {children}
      </div>
    </div>
  );
}

function GlobalSettingsModal({ onClose }: any) {
  const [tab, setTab] = useState('llm');
  return (
    <div className="fixed inset-0 z-50 bg-white flex animate-in fade-in duration-300">
      <div className="w-64 bg-slate-50 border-r border-slate-200 p-6 flex flex-col gap-2 shrink-0">
        <div className="flex items-center justify-between mb-8 px-3">
          <h2 className="text-[16px] font-semibold text-slate-800">全局设置</h2>
        </div>
        <button onClick={() => setTab('llm')} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'llm' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>模型配置 (BYOK)</button>
        <button onClick={() => setTab('data')} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'data' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>数据与主权</button>
        <button onClick={() => setTab('mission')} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'mission' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>使命宣言</button>
      </div>
      <div className="flex-1 p-12 overflow-y-auto max-w-5xl mx-auto">
        <div className="flex justify-between items-center mb-8">
          <h3 className="text-[24px] font-semibold text-slate-800">{tab === 'llm' ? 'LLM Provider 配置' : tab === 'data' ? '数据与隐私' : '个人使命宣言'}</h3>
          <button onClick={onClose} className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"><X size={24}/></button>
        </div>
        
        {tab === 'llm' && (
          <div className="space-y-6">
            <div className="bg-indigo-50 border border-indigo-100 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed">
              EgoSync 采用 BYOK (Bring Your Own Key) 模式，我们不触碰你的数据，也不赚取 API 差价。支持 OpenAI 兼容格式或 Anthropic 格式。
            </div>
            <div className="space-y-4">
              <div>
                <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Provider 标准</label>
                <select className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                  <option>OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)</option>
                  <option>Anthropic (Claude)</option>
                </select>
              </div>
              <div>
                <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Base URL</label>
                <input type="text" defaultValue="https://api.openai.com/v1" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
              </div>
              <div>
                <label className="block text-[13px] font-medium text-slate-700 mb-1.5">API Key</label>
                <input type="password" defaultValue="sk-..........................." className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
              </div>
              <div>
                <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Model Name</label>
                <input type="text" defaultValue="gpt-4o" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
              </div>
            </div>
            <button className="mt-4 px-6 py-2.5 bg-indigo-600 text-white rounded-lg text-[14px] font-medium hover:bg-indigo-700 transition-colors">保存并测试连接</button>
          </div>
        )}

        {tab === 'data' && (
          <div className="space-y-8">
            <div>
              <h4 className="text-[15px] font-medium text-slate-800 mb-2">导出完整数据</h4>
              <p className="text-[13px] text-slate-500 mb-4">将所有角色的记忆、任务和对话记录导出为标准的 JSON/Markdown 格式。</p>
              <button className="flex items-center gap-2 px-5 py-2.5 border border-slate-300 rounded-lg text-[14px] font-medium text-slate-700 hover:bg-slate-50 transition-colors">
                <Download size={16}/> 导出存档
              </button>
            </div>
            <div className="pt-6 border-t border-slate-200">
              <h4 className="text-[15px] font-medium text-red-600 mb-2 flex items-center gap-2">危险区域</h4>
              <p className="text-[13px] text-slate-500 mb-4">永久销毁本地数据库中的所有数据。此操作不可逆！</p>
              <button className="flex items-center gap-2 px-5 py-2.5 bg-red-50 border border-red-200 rounded-lg text-[14px] font-medium text-red-600 hover:bg-red-100 transition-colors">
                <Trash2 size={16}/> 销毁所有数据
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function ArbitrationModal({ onClose }: any) {
  const [step, setStep] = useState(1);
  return (
    <Modal onClose={onClose} width="w-[600px]">
      <div className="flex flex-col">
        <div className="px-6 py-4 border-b border-slate-200 flex items-center justify-between bg-slate-50/50">
          <h2 className="text-[16px] font-semibold flex items-center gap-2 text-slate-800"><AlertTriangle size={18} className="text-amber-500"/> 时间冲突仲裁</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>
        
        <div className="px-6 py-3 flex gap-2 border-b border-slate-100">
          <div className={cn("px-3 py-1 rounded-full text-[12px] font-medium", step >= 1 ? "bg-indigo-100 text-indigo-700" : "bg-slate-100 text-slate-400")}>1. 诉求呈现</div>
          <div className={cn("px-3 py-1 rounded-full text-[12px] font-medium", step >= 2 ? "bg-indigo-100 text-indigo-700" : "bg-slate-100 text-slate-400")}>2. 方案建议</div>
          <div className={cn("px-3 py-1 rounded-full text-[12px] font-medium", step >= 3 ? "bg-indigo-100 text-indigo-700" : "bg-slate-100 text-slate-400")}>3. 决策</div>
        </div>

        <div className="p-6">
          <div className="flex gap-4 mb-6">
            <div className="flex-1 bg-indigo-50/50 border border-indigo-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-indigo-800 mb-2 flex items-center gap-1.5"><Briefcase size={14}/> 产品经理</h4>
              <p className="text-[13px] text-slate-600 leading-relaxed">周五下午 15:00 需要参加产品架构评审会，预计 1.5 小时。</p>
            </div>
            <div className="flex items-center text-slate-300 font-bold italic">VS</div>
            <div className="flex-1 bg-amber-50/50 border border-amber-100 rounded-xl p-4">
              <h4 className="text-[13px] font-semibold text-amber-800 mb-2 flex items-center gap-1.5"><Heart size={14}/> 家庭</h4>
              <p className="text-[13px] text-slate-600 leading-relaxed">周五下午 15:30 答应了去接孩子放学并参加家长会。</p>
            </div>
          </div>

          {step >= 2 && (
            <div className="bg-emerald-50 border border-emerald-100 rounded-xl p-5 animate-in slide-in-from-bottom-2 mb-6">
              <h4 className="text-[13px] font-semibold text-emerald-800 mb-2 flex items-center gap-1.5">💡 管家建议</h4>
              <p className="text-[14px] text-emerald-900/80 leading-relaxed">
                根据你的使命宣言「家庭优先原则」，我建议优先保证家长会。我已经为你拟好了一封邮件，向团队申请将评审会提前至 13:30，会议室在此时间段空闲。
              </p>
            </div>
          )}
        </div>

        <div className="px-6 py-4 border-t border-slate-200 bg-slate-50 flex justify-end gap-3">
          {step === 1 && <button onClick={() => setStep(2)} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700">下一步</button>}
          {step === 2 && (
            <>
              <button onClick={onClose} className="px-5 py-2.5 border border-slate-300 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-100">延后处理</button>
              <button onClick={onClose} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 flex items-center gap-2"><Check size={16}/> 采纳建议</button>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}

function WeeklyReviewModal({ onClose }: any) {
  return (
    <Modal onClose={onClose} width="w-[700px]">
      <div className="p-8">
        <div className="flex justify-between items-start mb-6">
          <div>
            <h2 className="text-[24px] font-light text-slate-800">周复盘</h2>
            <p className="text-[13px] text-slate-500 mt-1">2026年5月12日 - 18日</p>
          </div>
          <button onClick={onClose} className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"><X size={20}/></button>
        </div>

        <div className="bg-slate-50 rounded-xl p-5 mb-8">
          <p className="text-[14px] text-slate-700 leading-[1.8]">
            这周你在3个维度都有进展。**产品经理**角色完成了竞品分析和两次评审准备，效率很高。**家庭**角色帮你安排了周末科技馆活动，孩子们很开心。**学习者**角色稳步推进《系统思考》的阅读。
          </p>
        </div>

        <div className="grid grid-cols-2 gap-8">
          <div>
            <h3 className="text-[13px] font-semibold text-slate-800 mb-4">角色能量趋势</h3>
            <div className="flex items-end gap-4 h-32 border-b border-slate-200 pb-2">
              <div className="flex-1 bg-indigo-500 rounded-t-md relative group" style={{ height: '85%' }}>
                <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-indigo-600 opacity-0 group-hover:opacity-100 transition-opacity">85%</span>
              </div>
              <div className="flex-1 bg-amber-500 rounded-t-md relative group" style={{ height: '60%' }}>
                <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-amber-600 opacity-0 group-hover:opacity-100 transition-opacity">60%</span>
              </div>
              <div className="flex-1 bg-purple-500 rounded-t-md relative group" style={{ height: '70%' }}>
                <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[11px] font-bold text-purple-600 opacity-0 group-hover:opacity-100 transition-opacity">70%</span>
              </div>
            </div>
            <div className="flex gap-4 mt-2 text-[11px] text-slate-400 text-center">
              <div className="flex-1">PM</div>
              <div className="flex-1">家庭</div>
              <div className="flex-1">学习</div>
            </div>
          </div>

          <div>
            <h3 className="text-[13px] font-semibold text-slate-800 mb-4">本周大石头</h3>
            <div className="space-y-3">
              <div className="flex items-center gap-3 text-[13px] text-slate-700">
                <CheckCircle2 size={16} className="text-emerald-500 shrink-0" />
                <span>竞品分析报告完成</span>
              </div>
              <div className="flex items-center gap-3 text-[13px] text-slate-700">
                <CheckCircle2 size={16} className="text-emerald-500 shrink-0" />
                <span>周末家庭活动安排</span>
              </div>
              <div className="flex items-center gap-3 text-[13px] text-slate-700 opacity-60">
                <ArrowRight size={16} className="text-amber-500 shrink-0" />
                <span>《系统思考》阅读（延至下周）</span>
              </div>
            </div>
          </div>
        </div>

        <div className="mt-10 flex justify-center">
          <button onClick={onClose} className="px-6 py-2.5 bg-indigo-600 text-white rounded-xl text-[14px] font-medium hover:bg-indigo-700 transition-colors">规划下周大石头</button>
        </div>
      </div>
    </Modal>
  );
}

function TaskModal({ onClose }: any) {
  return (
    <Modal onClose={onClose} width="w-[500px]">
      <div className="p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-[18px] font-semibold text-slate-800">新建任务</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>
        
        <div className="space-y-5">
          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-1.5">任务内容</label>
            <input type="text" placeholder="例如：准备Q3 OKR规划" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>
          
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-[13px] font-medium text-slate-700 mb-1.5">四象限分类</label>
              <select className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none">
                <option>Q1: 重要且紧急</option>
                <option>Q2: 重要不紧急</option>
                <option>Q3: 紧急不重要</option>
                <option>Q4: 不重要不紧急</option>
              </select>
            </div>
            <div>
              <label className="block text-[13px] font-medium text-slate-700 mb-1.5">截止时间</label>
              <input type="date" className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 outline-none text-slate-600" />
            </div>
          </div>

          <label className="flex items-center gap-3 p-3 border border-amber-200 bg-amber-50/50 rounded-lg cursor-pointer">
            <input type="checkbox" className="w-4 h-4 text-amber-600 rounded border-amber-300 focus:ring-amber-500" />
            <div className="flex-1">
              <div className="text-[13px] font-semibold text-amber-800">标记为本周大石头</div>
              <div className="text-[11px] text-amber-600/80">在周规划中优先受到系统时间保护</div>
            </div>
            <Target size={20} className="text-amber-500/50" />
          </label>
        </div>

        <div className="mt-8 flex justify-end gap-3">
          <button onClick={onClose} className="px-5 py-2.5 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50">取消</button>
          <button onClick={onClose} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700">保存任务</button>
        </div>
      </div>
    </Modal>
  );
}
