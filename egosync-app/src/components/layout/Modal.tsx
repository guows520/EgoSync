import { useEffect, useRef } from 'react';
import { cn } from '../../lib/utils';

export interface ModalProps {
  children: React.ReactNode;
  onClose: () => void;
  /** 期望宽度（Tailwind w-* class）。375px 级移动视口下统一收缩为
   *  w-[calc(100%-2rem)]（Story 16.2 响应式基线），桌面宽度不变。 */
  width?: string;
  ariaLabel: string;
}

/** 移动视口断点下的统一宽度（与视口留 16px 两侧边距）。 */
const MODAL_MOBILE_WIDTH = 'max-md:w-[calc(100%-2rem)] max-md:max-w-none';

export function Modal({ children, onClose, width = "w-[540px]", ariaLabel }: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  const previousFocusRef = useRef<Element | null>(null);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onCloseRef.current();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  useEffect(() => {
    previousFocusRef.current = document.activeElement;
    const dialog = dialogRef.current;
    if (dialog) {
      dialog.focus();
    }
    return () => {
      const previous = previousFocusRef.current;
      if (previous instanceof HTMLElement && document.body.contains(previous)) {
        previous.focus();
      }
    };
  }, []);

  return (
    // Story 16.4：上下安全区内边距（viewport-fit=cover 下 iOS 刘海/底部
    // 横条不贴边；无 inset 设备 env()=0 零影响——桌面渲染不变）。
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/20 backdrop-blur-sm animate-in fade-in duration-200 pt-[max(0px,env(safe-area-inset-top))] pb-[max(0px,env(safe-area-inset-bottom))]">
      <div className="absolute inset-0" onClick={onClose}></div>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        tabIndex={-1}
        className={cn("bg-white rounded-2xl shadow-2xl overflow-hidden relative z-10 animate-in zoom-in-95 duration-200 outline-none max-md:max-h-full max-md:overflow-y-auto", width, MODAL_MOBILE_WIDTH)}
      >
        {children}
      </div>
    </div>
  );
}
