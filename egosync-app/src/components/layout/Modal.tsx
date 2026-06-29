import { useEffect, useRef } from 'react';
import { cn } from '../../lib/utils';

export interface ModalProps {
  children: React.ReactNode;
  onClose: () => void;
  width?: string;
  ariaLabel: string;
}

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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/20 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="absolute inset-0" onClick={onClose}></div>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        tabIndex={-1}
        className={cn("bg-white rounded-2xl shadow-2xl overflow-hidden relative z-10 animate-in zoom-in-95 duration-200 outline-none", width)}
      >
        {children}
      </div>
    </div>
  );
}
