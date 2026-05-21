import { Bell, X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { MOCK_NOTIFICATIONS } from '../../constants/mockData';

export function NotificationPanel({ onClose }: any) {
  const levelConfig = {
    whisper: { label: '耳语', cls: 'bg-slate-100 text-slate-500 border-slate-200' },
    tap: { label: '轻触', cls: 'bg-blue-50 text-blue-600 border-blue-100' },
    knock: { label: '敲门', cls: 'bg-red-50 text-red-600 border-red-100 animate-pulse' },
  };

  return (
    <div className="fixed top-0 right-0 bottom-0 w-[380px] z-50 animate-in slide-in-from-right duration-300">
      <div className="h-full bg-white shadow-2xl border-l border-slate-200 flex flex-col">
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-200 shrink-0">
          <h2 className="text-[16px] font-semibold text-slate-800 flex items-center gap-2"><Bell size={18} /> 通知中心</h2>
          <button onClick={onClose} className="p-1.5 text-slate-400 hover:text-slate-700 hover:bg-slate-100 rounded-lg transition-colors"><X size={18} /></button>
        </div>
        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {MOCK_NOTIFICATIONS.map(n => {
            const lc = levelConfig[n.level];
            return (
              <div key={n.id} className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm hover:shadow-md transition-shadow">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className={cn("w-2.5 h-2.5 rounded-full", n.color)}></span>
                    <span className="text-[13px] font-semibold text-slate-700">{n.role}</span>
                  </div>
                  <span className={cn("px-2 py-0.5 rounded text-[10px] font-bold border", lc.cls)}>{lc.label}</span>
                </div>
                <p className="text-[13px] text-slate-600 leading-relaxed">{n.text}</p>
                <p className="text-[11px] text-slate-400 mt-2">{n.time}</p>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
