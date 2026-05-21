import { useState } from 'react';
import { cn } from '../../lib/utils';

export function ProactivityToggle() {
  const [level, setLevel] = useState<'passive' | 'moderate' | 'proactive'>('moderate');
  return (
    <div className="bg-slate-100/80 rounded-xl p-1.5 flex shadow-inner">
      <button onClick={() => setLevel('passive')} className={cn("flex-1 py-2 text-[13px] font-medium rounded-lg transition-colors", level === 'passive' ? "bg-white shadow-sm text-indigo-600 border border-slate-200/50" : "text-slate-500 hover:text-slate-800")}>静默执行</button>
      <button onClick={() => setLevel('moderate')} className={cn("flex-1 py-2 text-[13px] font-medium rounded-lg transition-colors", level === 'moderate' ? "bg-white shadow-sm text-indigo-600 border border-slate-200/50" : "text-slate-500 hover:text-slate-800")}>适度建议</button>
      <button onClick={() => setLevel('proactive')} className={cn("flex-1 py-2 text-[13px] font-medium rounded-lg transition-colors", level === 'proactive' ? "bg-white shadow-sm text-indigo-600 border border-slate-200/50" : "text-slate-500 hover:text-slate-800")}>积极主动</button>
    </div>
  );
}
