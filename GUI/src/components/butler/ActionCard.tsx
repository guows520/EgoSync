import { useState } from 'react';

export function ActionCard({ icon, title, meta, primaryBtn, onPrimary }: any) {
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
