import { useEffect, useState } from 'react';
import { getTransport, isTauriHost } from '@/transport';
import type { ConnectionState } from '@/transport';

/**
 * 连接状态 hook（Story 16.2）——包装 Transport.onConnectionStateChange。
 *
 * - 初始态按宿主诚实给定：Tauri 恒在线（'online'）；浏览器启动时连接
 *   尚未建立（'connecting'）——订阅即回调当前真实状态后立即校正；
 * - 三态诚实呈现（connecting / online / reconnecting），供徽标+横幅消费；
 * - 卸载即撤订阅（与 useEngineEvent 同款形状）。
 */
export function useConnectionState(): ConnectionState {
  // 惰性初始：Tauri 宿主恒 online；浏览器宿主启动即 connecting（订阅
  // 回调会立刻给出真实状态）。此前硬编码 'online' 会让浏览器首帧闪现
  // 错误的「在线」态——既违反三态诚实呈现，也让依赖徽标的等待锚点在
  // 真正建连之前即可通过（e2e 实测 66ms 即过）。
  const [state, setState] = useState<ConnectionState>(() =>
    isTauriHost() ? 'online' : 'connecting',
  );

  useEffect(() => {
    // 订阅即回调当前状态（HttpTransport 契约）——初始惰性值被立即校正。
    const unlisten = getTransport().onConnectionStateChange(next => {
      setState(next);
    });
    return unlisten;
  }, []);

  return state;
}
