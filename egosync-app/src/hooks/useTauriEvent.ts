import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
  deps: unknown[] = []
) {
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let mounted = true;

    listen<T>(eventName, (event) => {
      if (mounted) {
        handler(event.payload);
      }
    }).then((fn) => {
      if (mounted) {
        unlisten = fn;
      } else {
        Promise.resolve(fn()).catch(() => {});
      }
    }).catch(() => {});

    return () => {
      mounted = false;
      Promise.resolve(unlisten?.()).catch(() => {});
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventName, ...deps]);
}
