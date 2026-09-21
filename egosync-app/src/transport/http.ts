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
//
// Story 16.3 远程桌面通道（构造选项注入，浏览器默认路径字节级不变）：
// - `new HttpTransport()`（浏览器）：相对路径 + Cookie 凭证——既有行为；
// - `new HttpTransport({kind:'remote', baseUrl, token})`（桌面远程模式）：
//   绝对 URL + `Authorization: Bearer` 头 + SSE 一次性票据
//   （`POST /api/events/ticket` 签发 → `GET /api/events?ticket=` 建流——
//   EventSource 无法带自定义头）；
// - 远程模式应用层退避 governor：致命关闭重建 1s→30s 封顶指数退避+抖动
//   （架构裁决 B；浏览器分支维持既有固定 5s——浏览器由原生重连兜底）。

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

/** 远程模式退避参数（架构裁决 B 冻结款）：基数 1s、封顶 30s、抖动 ±20%。 */
const REMOTE_BACKOFF_BASE_MS = 1000;
const REMOTE_BACKOFF_MAX_MS = 30000;
const REMOTE_BACKOFF_JITTER = 0.2;

/**
 * 远程桌面构造选项（Story 16.3）。
 *
 * `kind:'remote'` ⇒ 绝对 base URL + Bearer 令牌 + SSE 票据流程 + 应用层
 * 退避 governor。缺省（无参构造）= 浏览器相对路径语义（字节级不变）。
 */
export interface HttpTransportOptions {
  kind: 'remote';
  /** 远程实例 base URL（`https://host[:port]`，无尾斜杠——经 trimRemoteBase 归一）。 */
  baseUrl: string;
  /** 远程实例主令牌（Bearer 头与票据签发共用）。 */
  token: string;
}

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

  // ── Story 16.3：远程桌面通道态 ──
  /** 远程模式配置（null = 浏览器相对路径——既有行为）。 */
  private remote: HttpTransportOptions | null;
  /** 远程退避 governor 当前指数（致命关闭连续次数；onopen 复位）。 */
  private remoteBackoffAttempt = 0;
  /**
   * SSE 建流票（远程模式专用）：ticket 在手 ⇒ 直接携票建流（免一次
   * 签发往返）；票据一次性——建流后置空，下次重建重走签发。
   */
  private sseTicket: string | null = null;
  /** 票据重试防环：建流票据被拒后重取重试仅一次（防签发-建流死循环）。 */
  private sseTicketRetried = false;
  /**
   * 建流世代号（teardown 递增）：异步签发-建流期间发生 teardown（订阅
   * 撤空）时，进行中的建流作废——防孤儿连接（无订阅即关不变量）。
   */
  private sseGeneration = 0;

  constructor(options?: HttpTransportOptions) {
    this.remote = options ?? null;
  }

  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const body = JSON.stringify(args ?? {});
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.remote) {
      headers['Authorization'] = `Bearer ${this.remote.token}`;
    }
    return fetch(this.resolveUrl(`/api/cmd/${command}`), {
      method: 'POST',
      credentials: 'same-origin',
      headers,
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
              // 远程桌面（16.3）同语义消费：AuthGate 远程分支回令牌重录视图。
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
   *
   * 远程桌面（16.3）：Bearer 头携带主令牌——`authenticated:true` 即
   * 令牌有效（server auth_status 支持 Bearer 叠加判定）。
   */
  async getAuthStatus(): Promise<AuthStatus> {
    if (this.authStatusCache !== undefined) {
      return this.authStatusCache;
    }
    const headers: Record<string, string> = {};
    if (this.remote) {
      headers['Authorization'] = `Bearer ${this.remote.token}`;
    }
    const res = await fetch(this.resolveUrl('/api/auth/status'), {
      credentials: 'same-origin',
      headers,
    });
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

  /**
   * 远程令牌更新（Story 16.3 令牌重录路径）：进程内实例**不重建**——
   * AuthGate 的 auth:unauthorized 订阅挂在既有实例上，重建实例会割裂
   * 订阅面；就地换令牌 + 认证缓存失效（下次 getAuthStatus 以新令牌判定）。
   *
   * 仅远程实例有意义（浏览器实例 no-op）；SSE 在手票据一并作废
   * （以旧令牌签发的票据不该再用于新令牌会话——下次建流重走签发）。
   */
  updateToken(token: string): void {
    if (!this.remote) return;
    this.remote = { ...this.remote, token };
    this.authStatusCache = undefined;
    this.sseTicket = null;
  }

  // ── 内部：EventSource 生命周期与状态机 ──

  /**
   * 关闭 SSE 连接并取消重建定时器（Story 16.1：无订阅即关）。
   *
   * - 登出/401 后 App 卸载 ⇒ 最后一个事件订阅撤空 ⇒ 本方法执行：
   *   EventSource 关闭 + 重建定时器取消（防 401 重建循环）；
   * - 状态回 connecting（诚实呈现：下次订阅时懒重建、onopen 后回 online）；
   * - 远程模式（16.3）：世代号递增（在手票据与进行中的异步建流一并作废
   *   ——票据 30s TTL 短时效，重订阅时重走签发，不做跨生命周期滞留）。
   */
  private teardownEventSource(): void {
    this.sseGeneration += 1;
    if (this.rebuildTimer) {
      clearTimeout(this.rebuildTimer);
      this.rebuildTimer = null;
    }
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
    this.sseTicket = null;
    this.sseTicketRetried = false;
    this.remoteBackoffAttempt = 0;
    this.setState('connecting');
  }

  private ensureEventSource(): void {
    if (this.eventSource) return;
    if (this.remote) {
      // 远程模式：票据流程建流（异步签发——EventSource 构造是同步的，
      // 票据获取后才构造；世代号防 teardown 竞态）
      void this.ensureRemoteEventSource();
      return;
    }
    this.connectEventSource('/api/events');
  }

  /**
   * 远程模式 SSE 建流（Story 16.3）：
   * 1. 无在手票据 ⇒ `POST /api/events/ticket`（Bearer）签发；
   * 2. `GET /api/events?ticket=` 构造 EventSource；
   * 3. 签发失败（网络断/服务器不可达）⇒ 退避 governor 调度重建
   *    （REMOTE_OFFLINE 呈现归 useConnectionState/ConnectionStatus 三态面）。
   *
   * 令牌失效（签发 401）**不由本通道判定**——票据签发失败只意味着
   * 「此刻无法建流」；令牌失效由 invoke 通道的 401 统一判定
   * （网络错误≠401 严格分流）。此处 401 仅转发全局事件供 AuthGate
   * 分流（AuthGate 在此场景必然先经 invoke/status 通道收到同一信号）。
   */
  private async ensureRemoteEventSource(): Promise<void> {
    const generation = this.sseGeneration;
    try {
      if (!this.sseTicket) {
        const ticket = await this.fetchSseTicket();
        // [评审轮2 U16] 签发往返期间 teardown（世代翻新）：票据不入库
        // ——原实现先赋值后检查，票据跨订阅生命周期滞留，重订阅会复用
        // 旧票（过期票 ⇒ 多一次建流失败后才自愈）
        if (generation !== this.sseGeneration) return;
        this.sseTicket = ticket;
      }
    } catch (e) {
      if (e instanceof HttpTransportError && e.status === 401) {
        emitFrontendLocalEvent('auth:unauthorized');
        return;
      }
      // 网络失败/5xx/429：退避 governor 重试（1s→30s+抖动）
      this.scheduleRebuild();
      return;
    }
    // 签发往返期间发生了 teardown（订阅撤空/世代翻新）——建流作废
    if (generation !== this.sseGeneration) return;
    // 另一条建流路径已构造连接（并发订阅场景）——本票据作废即可
    if (this.eventSource) return;
    this.connectEventSource(`/api/events?ticket=${encodeURIComponent(this.sseTicket)}`);
  }

  /** `POST /api/events/ticket`（Bearer）→ `{ticket}`（网络失败 ⇒ status 0 错误）。 */
  private async fetchSseTicket(): Promise<string> {
    const res = await fetch(this.resolveUrl('/api/events/ticket'), {
      method: 'POST',
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${this.remote!.token}`,
      },
      body: '{}',
    }).catch(() => {
      throw new HttpTransportError(0, null);
    });
    const text = await res.text().catch(() => {
      throw new HttpTransportError(0, null);
    });
    if (res.status !== 200) {
      throw new HttpTransportError(res.status, parseJsonOrText(text));
    }
    const parsed = parseJsonOrText(text) as { ticket?: unknown };
    if (typeof parsed?.ticket !== 'string' || parsed.ticket.length === 0) {
      throw new HttpTransportError(0, null);
    }
    return parsed.ticket;
  }

  /** EventSource 构造与桥接挂接（浏览器/远程同路径，仅 URL 与重连策略不同）。 */
  private connectEventSource(url: string): void {
    const source = new EventSource(this.remote ? this.resolveUrl(url) : url);
    source.onopen = () => {
      const wasReconnecting = this.state === 'reconnecting';
      this.remoteBackoffAttempt = 0;
      this.sseTicketRetried = false;
      // 票据已消费（一次性）——持票清空，下次重建重走签发
      this.sseTicket = null;
      this.setState('online');
      if (wasReconnecting) {
        // onopen-after-error：恢复并重放白名单（结果经 transport:reconnected 交付）
        void this.replayAfterReconnect();
      }
    };
    source.onerror = () => {
      if (this.state === 'online') {
        this.setState('reconnecting');
      } else if (this.remote && this.state === 'connecting') {
        // [评审轮2 U17] 远程首连失败：立即转 reconnecting——gate 已过而
        // 事件流失联时「重连中」横幅须可呈现（三态诚实呈现；此前恒留
        // connecting ⇒ 静默失联无感知）。浏览器首连保持 connecting 为
        // 冻结语义（非致命 onerror = 原生重连中），不受影响。
        this.setState('reconnecting');
      }
      if (!this.remote) {
        // 浏览器路径（字节级不变）：致命关闭（HTTP 错误等）浏览器对非 200
        // 不做原生重连（WHATWG 语义=永久关闭）——应用层定时重建，否则
        // 事件与恢复重放永不复活（评审 G2）；首连期非致命 onerror 保持
        // connecting（冻结语义）
        if ((source as EventSource).readyState === EventSource.CLOSED) {
          this.eventSource = null;
          this.scheduleRebuild();
        }
        return;
      }
      // 远程路径（16.3 全接管）：票据是一次性的——原生重连复用已消费
      // 票据的 URL 必然 401（永远无法成功），浏览器自管退避也不可控。
      // 任何错误都由应用层接管：首个错误立即重取票据重试（票据消费/
      // 过期的正常路径——I/O 矩阵「票据过期→重取，不升级为离线」），
      // 其后走退避 governor（1s→30s+抖动——AC「拔线后指数退避重连」）。
      source.close();
      this.eventSource = null;
      this.sseTicket = null;
      if (!this.sseTicketRetried) {
        this.sseTicketRetried = true;
        void this.ensureRemoteEventSource();
        return;
      }
      this.scheduleRebuild();
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
    const delay = this.remote
      ? this.remoteBackoffDelay()
      : FATAL_CLOSE_REBUILD_MS;
    this.rebuildTimer = setTimeout(() => {
      this.rebuildTimer = null;
      this.ensureEventSource();
    }, delay);
  }

  /**
   * 远程退避 governor（架构裁决 B 冻结款）：1s→30s 封顶指数退避 + ±20% 抖动。
   *
   * attempt 从 1 起（首次失败即 1s 档），封顶 30s；抖动为乘性（对期望
   * 延迟 ±20%）——多客户端同断线场景防雷鸣群重连（thundering herd）。
   */
  private remoteBackoffDelay(): number {
    this.remoteBackoffAttempt += 1;
    const exponential = Math.min(
      REMOTE_BACKOFF_BASE_MS * 2 ** (this.remoteBackoffAttempt - 1),
      REMOTE_BACKOFF_MAX_MS,
    );
    const jitter = 1 + (Math.random() * 2 - 1) * REMOTE_BACKOFF_JITTER;
    return Math.round(exponential * jitter);
  }

  /** 远程模式绝对 URL 拼接（base 无尾斜杠归一 + path 保证前导斜杠）。 */
  private resolveUrl(path: string): string {
    if (!this.remote) return path;
    const base = this.remote.baseUrl.replace(/\/+$/, '');
    return `${base}${path.startsWith('/') ? path : `/${path}`}`;
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
