// Story 15.5：HttpTransport —— 浏览器宿主实现。
//
// - invoke：fetch `POST /api/cmd/{cmd}`（credentials:'same-origin' 自动携带
//   egosync_session Cookie、Content-Type JSON、空参⇒`{}`）；
//   200+判别头 `X-Egosync-App-Error: 1` ⇒ reject body 解析值（与 Tauri
//   invoke rejection 载荷同构）；200 ⇒ resolve body；401/429/404/网络 ⇒
//   reject HttpTransportError{status,body}——其中 401 额外经前端本地事件
//   发射 `auth:unauthorized`（Story 16.1 全局拦截：AuthGate 回登录页，
//   不白屏；rejection 照旧交付调用方 catch 路径）；
// - 事件：单条 EventSource `/api/events` 多路复用（按帧 `event:` 字段分发
//   给全部同名 handler，data JSON 解析 payload 同构）；FRONTEND_LOCAL_EVENTS
//   走进程内总线不进 EventSource；**无订阅即关**（Story 16.1）——全部事件
//   订阅撤空即关闭 EventSource 并取消重建定时器（登出/401 后 App 卸载
//   触发，防未认证 401 重建循环；再订阅时懒重建）；
// - 连接状态机：connecting → online → reconnecting → online（初始
//   connecting，首 onopen 前的 onerror 保持 connecting）；网络级断连由
//   EventSource 原生自动重连兜底（浏览器管退避），致命关闭（非 200 等，
//   浏览器不重连）由应用层定时重建（FATAL_CLOSE_REBUILD_MS）；
// - onopen-after-error 触发白名单重放（`{}` 逐条、结果含错误也入集、并发
//   防护只重放一次）并经 `transport:reconnected` 前端本地事件交付结果集；
// - getAuthStatus()：首次成功请求后缓存（login 5 次/分/IP 限流预算保护
//   ——15.4 Design Notes 遗嘱）；登录/setup 成功与登出后经
//   invalidateAuthStatusCache() 失效（15-5 G11 遗嘱——否则 pre-login 的
//   authenticated:false 钉死整个进程期）。

import { TRANSPORT_CAPABILITIES } from './capabilities';
import {
  emitFrontendLocalEvent,
  isFrontendLocalEvent,
  subscribeFrontendLocalEvent,
} from './localEvents';
import { HttpTransportError } from './types';
import type { AuthStatus, ConnectionState, Transport, UnlistenFn } from './types';

/** AppError 响应判别头（与 server routes.rs 常量同源约定）。 */
const APP_ERROR_HEADER = 'x-egosync-app-error';

/** EventSource 致命关闭（非 200 等）后的重建退避间隔（浏览器不原生重连）。 */
const FATAL_CLOSE_REBUILD_MS = 5000;

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
  /** 致命关闭后的重建定时器（防叠加：同一时刻至多一个待触发的重建）。 */
  private rebuildTimer: ReturnType<typeof setTimeout> | null = null;
  /** `/api/auth/status` 结果缓存（限流预算保护）。 */
  private authStatusCache: AuthStatus | undefined;

  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const body = JSON.stringify(args ?? {});
    return fetch(`/api/cmd/${command}`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body,
    }).then(
      (res) =>
        // text() 的 onRejected 只捕「响应体读取中断」（连接中途重置）——
        // 原始流错误不得逃出 HttpTransportError 契约（评审 G3）；业务
        // 错误（onFulfilled 内的 reject）经同一 then 的 onRejected 不受影响
        res.text().then(
          (text) => {
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
            if (res.status === 401) {
              // Story 16.1：会话失效全局信号——AuthGate 收到后回登录页
              //（内存态随 App 卸载清空）。rejection 照旧交付（调用方 catch
              // 路径行为不变）；登录面自身的 401 走 REST 直连不经此处。
              emitFrontendLocalEvent('auth:unauthorized');
            }
            return Promise.reject(
              new HttpTransportError(res.status, parsed)
            ) as Promise<T>;
          },
          () =>
            // 响应体读取中断：与网络失败同形状（status 0 + body null）
            Promise.reject(new HttpTransportError(0, null)) as Promise<T>
        ),
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
      // Story 16.1：无订阅即关——全部事件订阅撤空 ⇒ 关闭 EventSource 并
      // 取消重建定时器。登出/401 后 AuthGate 卸载 App 触发本路径（App 顶层
      // 的 useEngineEvent 卸载即撤订阅）；再订阅时 ensureEventSource 懒重建。
      // 不关则会形成未认证 401 的 5s 重建循环（SSE 致命关闭→定时重建→再 401）。
      if (this.handlers.size === 0) {
        this.teardownEventSource();
      }
    };
  }

  onConnectionStateChange(handler: (state: ConnectionState) => void): UnlistenFn {
    // 订阅即回调当前状态（初始 connecting / 恢复后 online）——与 setState
    // 扇出同款隔离（评审 G8）：坏订阅者不得炸掉订阅流程本身
    try {
      handler(this.state);
    } catch (e) {
      console.error('传输状态订阅者抛错（已隔离）:', e);
    }
    this.stateHandlers.add(handler);
    return () => {
      this.stateHandlers.delete(handler);
    };
  }

  /**
   * 前端本地事件：浏览器分支进程内消化（不进传输契约）。
   * 进程内同步总线无失败面 ⇒ 恒 resolve 的 Promise（接口与 Tauri 分支同形）。
   */
  emitFrontendEvent(event: string, payload?: unknown): Promise<void> {
    emitFrontendLocalEvent(event, payload);
    return Promise.resolve();
  }

  /**
   * 认证态发现（`GET /api/auth/status`）：首次成功请求后缓存。
   *
   * `/api/auth/*` 与 setup 共享 5 次/分/IP 限流——轮询该端点会烧穿 login
   * 预算（15-4 Design Notes 遗嘱），故进程内只请求一次；AuthGate（16.1）
   * 为消费者。缓存失效见 [`invalidateAuthStatusCache`]（15-5 G11 遗嘱：
   * 登录/setup 成功与登出后必须失效，否则 pre-login 的
   * authenticated:false 钉死整个进程期）。
   */
  async getAuthStatus(): Promise<AuthStatus> {
    if (this.authStatusCache !== undefined) {
      return this.authStatusCache;
    }
    const res = await fetch('/api/auth/status', { credentials: 'same-origin' });
    if (res.status !== 200) {
      const parsed = parseJsonOrText(await res.text());
      throw new HttpTransportError(res.status, parsed);
    }
    this.authStatusCache = parseJsonOrText(await res.text()) as AuthStatus;
    return this.authStatusCache;
  }

  /** 认证态缓存失效（登录/setup 成功与登出后调用）。 */
  invalidateAuthStatusCache(): void {
    this.authStatusCache = undefined;
  }

  // ── 内部：EventSource 生命周期与状态机 ──

  /**
   * 关闭 SSE 连接并取消重建定时器（Story 16.1：无订阅即关）。
   *
   * - 登出/401 后 App 卸载 ⇒ 最后一个事件订阅撤空 ⇒ 本方法执行：
   *   EventSource 关闭 + 重建定时器取消（防 401 重建循环）；
   * - 状态回 connecting（诚实呈现：下次订阅时懒重建、onopen 后回 online）。
   */
  private teardownEventSource(): void {
    if (this.rebuildTimer) {
      clearTimeout(this.rebuildTimer);
      this.rebuildTimer = null;
    }
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
    this.setState('connecting');
  }

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
      // 致命关闭（HTTP 错误等）：浏览器对非 200 不做原生重连（WHATWG
      // 语义=永久关闭）——应用层定时重建，否则事件与恢复重放永不复活
      // （评审 G2）；首连期非致命 onerror 保持 connecting（冻结语义）
      if ((source as EventSource).readyState === EventSource.CLOSED) {
        this.eventSource = null;
        this.scheduleRebuild();
      }
    };
    this.eventSource = source;
    // 重建场景（致命关闭后）：既有桥接重挂到新连接——桥接闭包按
    // handlers 集合扇出、与具体源无关；不重挂则事件投递随旧连接死掉
    for (const [event, bridge] of this.bridges) {
      source.addEventListener(event, bridge);
    }
  }

  /** 致命关闭后的重建退避：单一定时器防叠加，到期重建 SSE 连接。 */
  private scheduleRebuild(): void {
    if (this.rebuildTimer) return;
    this.rebuildTimer = setTimeout(() => {
      this.rebuildTimer = null;
      this.ensureEventSource();
    }, FATAL_CLOSE_REBUILD_MS);
  }

  private setState(next: ConnectionState): void {
    if (this.state === next) return;
    this.state = next;
    // 单个订阅者抛错不得中断其余订阅者与状态机推进（评审 G8：
    // onopen 内 setState 先于恢复重放——坏订阅者曾断掉整条恢复链）
    for (const handler of this.stateHandlers) {
      try {
        handler(next);
      } catch (e) {
        console.error('传输状态订阅者抛错（已隔离）:', e);
      }
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
