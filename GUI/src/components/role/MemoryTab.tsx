import { useState } from 'react';
import { Clock } from 'lucide-react';

export function MemoryTab() {
  const [expanded, setExpanded] = useState(false);
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
        <button onClick={() => setExpanded(!expanded)} className="w-full bg-slate-50 border border-slate-100 rounded-md p-2.5 text-[12px] text-slate-500 font-mono flex items-center gap-2 hover:bg-slate-100 hover:text-slate-700 transition-colors cursor-pointer text-left">
          <Clock size={14} /> 引用于 2026-05-18 10:20 对话记录 <span className="ml-auto text-indigo-500 text-[11px]">{expanded ? '收起' : '查看原文'}</span>
        </button>
        {expanded && (
          <div className="mt-3 border-l-2 border-indigo-300 pl-4 space-y-2.5 animate-in slide-in-from-top-2 duration-200">
            <div className="text-[13px] text-slate-500"><span className="font-semibold text-slate-700">boss：</span>帮我分析一下这个方案的可行性，要有数据支撑。</div>
            <div className="text-[13px] text-slate-500"><span className="font-semibold text-indigo-600">产品经理：</span>好的，我来整理市场数据和用户反馈。纯主观判断还是需要定量支撑？</div>
            <div className="text-[13px] text-slate-500"><span className="font-semibold text-slate-700">boss：</span>对，我不喜欢纯感觉的东西，要有收益模型。</div>
          </div>
        )}
      </div>
    </div>
  );
}
