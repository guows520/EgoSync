import { useState } from 'react';
import { Home, Play } from 'lucide-react';
import { cn } from '../../lib/utils';

export function OnboardingView({ onComplete }: any) {
  const [step, setStep] = useState(0);
  const [input, setInput] = useState('');
  const [userName, setUserName] = useState('');
  const [chat, setChat] = useState<{sender:string, text:string}[]>([
    {sender: 'butler', text: '你好 👋 我是你的数字管家。该怎么称呼你呢？'}
  ]);

  const handleSend = () => {
    if (!input.trim()) return;
    const newChat = [...chat, {sender: 'user', text: input}];
    setChat(newChat);
    setInput('');
    
    if (step === 0) {
      const name = input.trim();
      setUserName(name);
      setTimeout(() => {
        setChat([...newChat, {sender: 'butler', text: `很高兴为您服务，**${name}**！聊聊你最近在忙什么？我来帮你安排。`}]);
        setStep(1);
      }, 800);
    } else if (step === 1) {
      setTimeout(() => {
        setChat([...newChat, {sender: 'butler', text: `听起来你工作中产品管理很重要。要不要先创建一个 **产品经理** 角色？`}]);
        setStep(2);
      }, 1000);
    } else if (step === 2) {
      setTimeout(() => {
        setChat([...newChat, {sender: 'butler', text: `✨ 产品经理角色已创建！好的${userName ? `，${userName}` : ''}，我来照顾这些方面。现在先熟悉一下环境吧。`}]);
        setTimeout(() => onComplete(), 2000);
      }, 1000);
    }
  };

  return (
    <div className="h-full flex flex-col relative bg-white/40 dark:bg-slate-900/40">
      {/* Header */}
      <header className="h-[76px] border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-8 shrink-0 justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-4">
          <div className="w-[46px] h-[46px] rounded-xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center shadow-sm">
            <Home size={24} strokeWidth={2} />
          </div>
          <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">数字管家</h2>
        </div>
      </header>
      
      {/* Chat Area */}
      <div className="flex-1 overflow-y-auto p-8 scroll-smooth">
        <div className="max-w-3xl mx-auto space-y-6">
          {chat.map((msg, i) => (
            <div key={i} className={cn("flex flex-col gap-1.5 animate-in slide-in-from-bottom-2", msg.sender === 'butler' ? "items-start" : "items-end")}>
              <div className={cn(
                "px-5 py-3.5 rounded-2xl max-w-[85%] text-[14px] leading-[1.7] shadow-sm",
                msg.sender === 'butler' ? "bg-white dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 rounded-tl-sm text-slate-700 dark:text-slate-200" : "bg-indigo-600 text-white rounded-tr-sm shadow-indigo-200/50"
              )} dangerouslySetInnerHTML={{__html: msg.text.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')}} />
            </div>
          ))}
        </div>
      </div>

      {/* Input Area */}
      <div className="p-5 bg-white/70 dark:bg-slate-800/70 backdrop-blur-md border-t border-slate-200/60 dark:border-slate-700/60 shrink-0">
        <div className="relative max-w-3xl mx-auto">
          <input 
            type="text" 
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSend()}
            placeholder={step === 0 ? "输入你的名字或昵称..." : step === 1 ? "比如：我是一个产品经理，最近在忙新产品上线..." : "确认，或者告诉我你想调整..."} 
            className="w-full bg-white dark:bg-slate-700 border border-slate-200 dark:border-slate-600 rounded-xl pl-5 pr-14 py-3.5 text-[14px] dark:text-slate-100 dark:placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-500/20 focus:border-slate-500 transition-all shadow-sm" 
          />
          <button onClick={handleSend} className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 flex items-center justify-center text-white bg-slate-800 rounded-lg transition-colors shadow-sm hover:bg-slate-700">
            <Play size={16} className="ml-0.5" fill="currentColor" />
          </button>
        </div>
      </div>
    </div>
  );
}
