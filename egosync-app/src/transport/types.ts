// Story 15.5：传输抽象类型面——双宿主（Tauri / HTTP）同构契约。
//
// 对等契约（Boundaries 冻结款）：同一 command 名、参数形状（camelCase 单层
// 扁平对象、可选 `?? null`、复合输入包 `input` 键——现状惯例原样）、返回
// 形状、事件名在双通道同构；Tauri 分支行为与现状逐字节一致。

/** 事件取消订阅函数（与 @tauri-apps/api UnlistenFn 同形）。 */
export type UnlistenFn = () => void;

/** HTTP 传输层连接状态（Tauri 分支恒 online）。 */
export type ConnectionState = 'connecting' | 'online' | 'reconnecting';

/**
 * HTTP 传输层错误（仅 HTTP 侧存在；Tauri 分支无此路径）。
 *
 * - 业务错误（AppError）不是传输层错误：200 + 判别头 `X-Egosync-App-Error: 1`
 *   的 body 解析值经 reject 交付，与 Tauri invoke rejection 载荷同构；
 * - 本类只承载 401 / 429 / 404 / 网络失败等非 200 或连接层失败；
 * - `status` 为 HTTP 状态码，网络失败（fetch reject）时为 0；
 * - `body` 为解析后 body（JSON 可解析则对象，否则原始文本；网络失败为 null）。
 */
export class HttpTransportError extends Error {
  readonly status: number;
  readonly body: unknown;

  constructor(status: number, body: unknown) {
    super(`EgoSync HTTP transport error (status ${status})`);
    this.name = 'HttpTransportError';
    this.status = status;
    this.body = body;
  }
}

/** 传输层能力清单（由生成物 capabilities.ts 单源提供，双实现共享）。 */
export interface TransportCapabilities {
  /** web-ok 命令全名单（双宿主可用，103 条——工件驱动计数）。 */
  readonly webCommands: readonly string[];
  /** desktop-only 命令名单（浏览器分支 UI 按 capability 隐入口）。 */
  readonly desktopOnlyCommands: readonly string[];
  /** perf-test 门控命令名单（基准构建专用，常规宿主不可用）。 */
  readonly perfTestGatedCommands: readonly string[];
  /** 重连补齐重放白名单（只读 query，SSE 恢复后 `{}` 逐条重放）。 */
  readonly replayWhitelist: readonly string[];
  /** 查询某命令是否 web-ok（双宿主可用）。 */
  isWebCommand(command: string): boolean;
  /** 查询某命令是否 desktop-only（仅 Tauri 宿主注册）。 */
  isDesktopOnly(command: string): boolean;
}

/**
 * 传输接口——UI/services 层唯一通信面。
 *
 * 四核心成员：invoke / on / capabilities / onConnectionStateChange；
 * 另含 `emitFrontendEvent`（前端本地事件发射：Tauri 分支走 event 插件
 * emit 原样、浏览器分支进程内消化——不进传输契约）。
 */
export interface Transport {
  /** 命令调用（与 @tauri-apps/api invoke 同形：cmd + 可选 camelCase 参数）。 */
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  /** 事件订阅，返回同步 unlisten（Tauri 分支保留 listen 异步语义与竞态防护）。 */
  on<T>(event: string, handler: (payload: T) => void): UnlistenFn;
  /** 能力清单（生成物单源）。 */
  readonly capabilities: TransportCapabilities;
  /** 连接状态订阅：订阅即回调当前状态，此后每次迁移回调；返回取消函数。 */
  onConnectionStateChange(handler: (state: ConnectionState) => void): UnlistenFn;
  /**
   * 前端本地事件发射（skill-scope-updated / transport:reconnected 等）。
   *
   * 返回 Promise（基线同形：旧 `emit(...)` 直返 Promise，调用方
   * `await + catch` 兜底可达）——Tauri 分支透传 event 插件 emit 的
   * rejection；浏览器分支进程内总线无失败面、恒 resolve。
   */
  emitFrontendEvent(event: string, payload?: unknown): Promise<void>;
}
