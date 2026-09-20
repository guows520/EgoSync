// Story 15.5：前端本地事件——进程内 EventEmitter 与 FRONTEND_LOCAL_EVENTS 路由。
//
// 前端本地事件定义：前端→前端、Rust 无监听、不进 SSE 契约的事件。
// - Tauri 分支：emit/listen 走 @tauri-apps/api/event 原样（桌面行为零变化）；
// - 浏览器分支：进程内 EventEmitter 消化（HttpTransport.on 对本清单事件
//   订阅进程内总线，不建 EventSource 订阅）。
//
// 名单：
// - `skill-scope-updated`：全仓唯一前端→前端 emit（skillService.notifyScopeUpdated）；
// - `transport:reconnected`：HttpTransport 重连重放的结果集交付通道
//   （payload = { results: { 命令名: 结果或错误 } }，UI 消费归 16.2）；
// - `auth:unauthorized`（Story 16.1）：认证失效全局信号——HttpTransport
//   invoke 401 与主动登出都发射；AuthGate 订阅后回登录页（内存态随 App
//   卸载清空）。不进 SSE 契约（认证失效时 SSE 已在拆除路径上）。

/** 前端本地事件名单（路由判别用；Tauri 分支不消费本名单——统一走 listen）。 */
export const FRONTEND_LOCAL_EVENTS: readonly string[] = [
  'skill-scope-updated',
  'transport:reconnected',
  'auth:unauthorized',
];

/** 是否前端本地事件（浏览器分支进程内消化、不进 SSE 契约）。 */
export function isFrontendLocalEvent(event: string): boolean {
  return FRONTEND_LOCAL_EVENTS.includes(event);
}

type LocalEventHandler = (payload: unknown) => void;

/** 进程内事件总线（浏览器分支的前端本地事件通道）。 */
const handlers = new Map<string, Set<LocalEventHandler>>();

/** 进程内发射前端本地事件（浏览器分支消费；无订阅者即丢弃——与 tauri emit 语义一致）。 */
export function emitFrontendLocalEvent(event: string, payload?: unknown): void {
  const set = handlers.get(event);
  if (!set) return;
  for (const handler of set) {
    try {
      handler(payload);
    } catch (e) {
      // 消费方异常不中断其他订阅者（容错与 SSE 扇出语义对齐）
      console.error(`[transport] 前端本地事件 ${event} 处理失败:`, e);
    }
  }
}

/** 订阅进程内前端本地事件，返回取消函数。 */
export function subscribeFrontendLocalEvent(
  event: string,
  handler: LocalEventHandler
): () => void {
  let set = handlers.get(event);
  if (!set) {
    set = new Set();
    handlers.set(event, set);
  }
  set.add(handler);
  return () => {
    set.delete(handler);
    if (set.size === 0) {
      handlers.delete(event);
    }
  };
}
