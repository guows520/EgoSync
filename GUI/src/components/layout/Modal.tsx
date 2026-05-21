import { cn } from '../../lib/utils';

export function Modal({ children, onClose, width = "w-[540px]" }: any) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/20 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="absolute inset-0" onClick={onClose}></div>
      <div className={cn("bg-white rounded-2xl shadow-2xl overflow-hidden relative z-10 animate-in zoom-in-95 duration-200", width)}>
        {children}
      </div>
    </div>
  );
}
