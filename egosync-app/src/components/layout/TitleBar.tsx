import { useState, useEffect } from 'react';
import { Minus, Square, X, Copy } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauriHost } from '@/transport';
import { Logo } from '../common/Logo';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    // 浏览器宿主：无窗口 API（getCurrentWindow 会抛错），跳过订阅
    if (!isTauriHost()) return;
    const win = getCurrentWindow();
    win.isMaximized().then(setIsMaximized).catch(() => {});
    const unlisten = win.onResized(() => {
      win.isMaximized().then(setIsMaximized).catch(() => {});
    });
    return () => {
      unlisten.then(fn => fn()).catch(() => {});
    };
  }, []);

  // 浏览器退化：普通标题栏（仅标题文本；布局细节归 UX 阶段定稿）
  if (!isTauriHost()) {
    return (
      <div className="h-9 shrink-0 flex items-center px-3 bg-[#F1F3F5] dark:bg-slate-800 select-none">
        <div className="flex items-center gap-1.5 pointer-events-none">
          <Logo size={30} />
          <span className="text-[13px] font-medium text-slate-600 dark:text-slate-300">EgoSync</span>
        </div>
      </div>
    );
  }

  const handleMinimize = () => getCurrentWindow().minimize();
  const handleToggleMaximize = () => getCurrentWindow().toggleMaximize();
  const handleClose = () => getCurrentWindow().close();

  return (
    <div
      data-tauri-drag-region
      className="h-9 shrink-0 flex items-center justify-between px-3 bg-[#F1F3F5] dark:bg-slate-800 select-none"
    >
      <div className="flex items-center gap-1.5 pointer-events-none">
        <Logo size={30} />
        <span className="text-[13px] font-medium text-slate-600 dark:text-slate-300">EgoSync</span>
      </div>
      <div className="flex items-center">
      <button
        onClick={handleMinimize}
        className="w-8 h-7 flex items-center justify-center rounded-md text-slate-500 hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
        title="最小化"
      >
        <Minus size={15} />
      </button>
      <button
        onClick={handleToggleMaximize}
        className="w-8 h-7 flex items-center justify-center rounded-md text-slate-500 hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
        title={isMaximized ? '还原' : '最大化'}
      >
        {isMaximized ? <Copy size={13} /> : <Square size={12} />}
      </button>
      <button
        onClick={handleClose}
        className="w-8 h-7 flex items-center justify-center rounded-md text-slate-500 hover:bg-red-500 hover:text-white transition-colors"
        title="关闭"
      >
        <X size={15} />
      </button>
      </div>
    </div>
  );
}
