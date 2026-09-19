import { useEffect } from 'react';
import { getTransport } from '@/transport';

/**
 * 引擎事件 hook（Story 15.5）——接替 useTauriEvent，签名与语义完全一致。
 *
 * 内部走 `getTransport().on`（Tauri 分支 listen 透传 + mounted 防泄漏 +
 * 静默容错；浏览器分支单条 EventSource 多路复用）。桌面行为零变化：
 * 旧 useTauriEvent 的 listen 异步语义/卸载竞态防护平移在 TauriTransport.on。
 */
export function useEngineEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
  deps: unknown[] = []
) {
  useEffect(() => {
    const unlisten = getTransport().on<T>(eventName, handler);
    return () => {
      unlisten();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventName, ...deps]);
}
