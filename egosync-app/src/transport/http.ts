// Story 15.5：HttpTransport —— 浏览器宿主实现。
//
// - invoke：fetch `POST /api/cmd/{cmd}`（credentials:'same-origin' 自动携带
//   egosync_session Cookie、Content-Type JSON、空参⇒`{}`）；
//   200+判别头 `X-Egosync-App-Error: 1` ⇒ reject body 解析值（与 Tauri
//   invoke rejection 载荷同构）；200 ⇒ resolve body；401/429/404/网络 ⇒
//   reject HttpTransportError{status,body}（401 不触发任何跳转——全局拦截归 16.1）；
// - 事件：单条 EventSource `/api/events` 多路复用（按帧 `event:` 字段分发
//   给全部同名 handler，data JSON 解析 payload 同构）；FRONTEND_LOCAL_EVENTS
//   走进程内总线不进 EventSource；
// - 连接状态机：connecting → online → reconnecting → online（初始
//   connecting，首 onopen 前的 onerror 保持 connecting）；EventSource 原生
//   自动重连（浏览器管退避，应用层只管状态呈现）；
// - onopen-after-error 触发白名单重放（`{}` 逐条、结果含错误也入集、并发
//   防护只重放一次）并经 `transport:reconnected` 前端本地事件交付结果集；
// - getAuthStatus()：首次成功请求后缓存（login 5 次/分/IP 限流预算保护
//   ——15.4 Design Notes 遗嘱）。

import { TRANSPORT_CAPABILITIES } from './capabilities';
import {
  emitFrontendLocalEvent,
  isFrontendLocalEvent,
  subscribeFrontendLocalEvent,
} from './localEvents';
import { HttpTransportError } from './types';
import type { ConnectionState, Transport, UnlistenFn } from './types';

/** AppError 响应判别头（与 server routes.rs 常量同源约定）。 */
const APP_ERROR_HEADER = 'x-egosync-app-error';

type StateHandler = (state: ConnectionState) => void;
type EventHandler = (payload: unknown) => void;

export class HttpTransport implements Transport {
  readonly capabilities = TRANSPORT_CAPABILITIES;

  /** SSE 连接（懒建：首个非本地事件订阅时；进程存活期不主动关闭）。 */
  private eventSource: EventSource | null = null;
  /** 事件名 → handler 集合（多路复用扇出）。 */
  private readonly handlers = new Map<string, Set<EventHandler>>();
  /** 事件名 → EventSource 上注册的桥接 listener（去重注册）。 */
  private readonly bridges = new Map<string, EventListener>();
  /** 连接状态订阅者。 */
  private readonly stateHandlers = new Set<StateHandler>();
  private state: ConnectionState = 'connecting';
  /** 重放并发防护：重放进行中时后续 onopen 不再触发（连续恢复只重放一次）。 */
  private replaying = false;
  /** `/api/auth/status` 结果缓存（限流预算保护）。 */
  private authStatusCache: unknown | undefined;

  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const body = JSON.stringify(args ?? {});
    return fetch(`/api/cmd/${command}`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body,
    }).then(
      (res) =>
        res.text().then((text) => {
          const parsed = parseJsonOrText(text);
          if (res.status === 200) {
            if (res.headers.get(APP_ERROR_HEADER) != null) {
              // 业务错误：200 + 判别头 ⇒ reject body 解析值（AppError 单键
              // map，与 Tauri invoke rejection 载荷同构）
              return Promise.reject(parsed) as Promise<T>;
            }
            return parsed as T;
          }
          // 401 / 429 / 404 / 5xx 等传输层错误（仅 HTTP 侧存在）
          return Promise.reject(
            new HttpTransportError(res.status, parsed)
          ) as Promise<T>;
        }),
      () =>
        // 网络失败（fetch reject）：status 0 + body null
        Promise.reject(new HttpTransportError(0, null)) as Promise<T>
    );
  }

  on<T>(event: string, handler: (payload: T) => void): UnlistenFn {
    // 前端本地事件：进程内消化，不进 SSE 契约
    if (isFrontendLocalEvent(event)) {
      return subscribeFrontendLocalEvent(event, handler as EventHandler);
    }
    this.ensureEventSource();
    let set = this.handlers.get(event);
    if (!set) {
      set = new Set();
      this.handlers.set(event, set);
    }
    set.add(handler as EventHandler);
    this.ensureBridge(event);
    return () => {
      const current = this.handlers.get(event);
      if (!current) return;
      current.delete(handler as EventHandler);
      if (current.size === 0) {
        this.handlers.delete(event);
        this.removeBridge(event);
      }
    };
  }

  onConnectionStateChange(handler: (state: ConnectionState) => void): UnlistenFn {
    // 订阅即回调当前状态（初始 connecting / 恢复后 online）
    handler(this.state);
    this.stateHandlers.add(handler);
    return () => {
      this.stateHandlers.delete(handler);
    };
  }

  /** 前端本地事件：浏览器分支进程内消化（不进传输契约）。 */
  emitFrontendEvent(event: string, payload?: unknown): void {
    emitFrontendLocalEvent(event, payload);
  }

  /**
   * 认证态发现（`GET /api/auth/status`）：首次成功请求后缓存。
   *
   * `/api/auth/*` 与 setup 共享 5 次/分/IP 限流——轮询该端点会烧穿 login
   * 预算（15-4 Design Notes 遗嘱），故进程内只请求一次。登录页/setup 向导
   * 的消费接线归 16.1。
   */
  async getAuthStatus(): Promise<unknown> {
    if (this.authStatusCache !== undefined) {
      return this.authStatusCache;
    }
    const res = await fetch('/api/auth/status', { credentials: 'same-origin' });
    if (res.status !== 200) {
      const parsed = parseJsonOrText(await res.text());
      throw new HttpTransportError(res.status, parsed);
    }
    this.authStatusCache = parseJsonOrText(await res.text());
    return this.authStatusCache;
  }

  // ── 内部：EventSource 生命周期与状态机 ──

  private ensureEventSource(): void {
    if (this.eventSource) return;
    const source = new EventSource('/api/events');
    source.onopen = () => {
      const wasReconnecting = this.state === 'reconnecting';
      this.setState('online');
      if (wasReconnecting) {
        // onopen-after-error：恢复并重放白名单（结果经 transport:reconnected 交付）
        void this.replayAfterReconnect();
      }
    };
    source.onerror = () => {
      if (this.state === 'online') {
        this.setState('reconnecting');
      }
      // 首次连接期的 onerror 保持 connecting（状态机冻结语义）；
      // EventSource 原生自动重连（浏览器管退避）
    };
    this.eventSource = source;
  }

  private setState(next: ConnectionState): void {
    if (this.state === next) return;
    this.state = next;
    for (const handler of this.stateHandlers) {
      handler(next);
    }
  }

  /** 事件名桥接 listener：EventSource 同名帧 → handler 集合扇出（data JSON 解析）。 */
  private ensureBridge(event: string): void {
    if (this.bridges.has(event) || !this.eventSource) return;
    const bridge: EventListener = (ev: Event) => {
      const data = (ev as MessageEvent<string>).data;
      let payload: unknown;
      try {
        payload = JSON.parse(data);
      } catch {
        return; // 非法 JSON 帧静默丢弃（与 listen 容错语义对齐）
      }
      const set = this.handlers.get(event);
      if (!set) return;
      for (const handler of set) {
        try {
          handler(payload);
        } catch (e) {
          console.error(`[transport] 事件 ${event} 处理失败:`, e);
        }
      }
    };
    this.eventSource.addEventListener(event, bridge);
    this.bridges.set(event, bridge);
  }

  private removeBridge(event: string): void {
    const bridge = this.bridges.get(event);
    if (!bridge || !this.eventSource) return;
    this.eventSource.removeEventListener(event, bridge);
    this.bridges.delete(event);
  }

  /**
   * 重连补齐重放：白名单 31 条只读 query 以 `{}` 逐条重放。
   *
   * - 写入类 command 永不在白名单（capabilities 单源不变量）；
   * - 单条失败（含 AppError 与传输错误）逐条入结果集，不中断整体；
   * - 并发防护：重放进行中时后续 onopen 不再触发（连续恢复只重放一次）；
   * - 结果集经 `transport:reconnected` 前端本地事件交付（UI 消费归 16.2）。
   */
  private async replayAfterReconnect(): Promise<void> {
    if (this.replaying) return;
    this.replaying = true;
    try {
      const results: Record<string, unknown> = {};
      for (const command of this.capabilities.replayWhitelist) {
        try {
          results[command] = await this.invoke<unknown>(command, {});
        } catch (e) {
          // 单条错误逐条入集不中断整体（AppError map 或 HttpTransportError）
          results[command] = e;
        }
      }
      emitFrontendLocalEvent('transport:reconnected', { results });
    } finally {
      this.replaying = false;
    }
  }
}

/** body 解析：JSON 可解析则对象/标量，否则原始文本（形状由调用方钉死）。 */
function parseJsonOrText(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}
