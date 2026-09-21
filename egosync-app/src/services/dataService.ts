import { invoke } from '@/transport';
import { emitFrontendLocalEvent } from '@/transport/localEvents';

export type ExportFormat = 'sqlite' | 'json' | 'markdown';

export interface ExportResult {
  files: string[];
  sqlitePath: string | null;
  jsonPath: string | null;
  markdownPath: string | null;
}

export interface ImportResult {
  rolesCount: number;
  tasksCount: number;
  memoriesCount: number;
  conversationsCount: number;
  messagesCount: number;
}

/** 导入后密钥可达性报告项（Story 17.3：/api/import 响应）。 */
export interface MissingSecret {
  configName: string;
  apiKeyRef: string;
  message: string;
}

/** 云端导入响应：ImportResult + 密钥缺失报告（结构化、不阻塞导入结果）。 */
export interface WebImportResult {
  imported: ImportResult;
  missingSecrets: MissingSecret[];
}

/** AppError 响应判别头（与 server routes.rs / http.ts 同源约定）。 */
const APP_ERROR_HEADER = 'x-egosync-app-error';

/** 云端备份端点的 fetch 超时（评审修复：慢链路挂起会让 UI 永停「导入中」）。 */
const WEB_BACKUP_TIMEOUT_MS = 120_000;

/**
 * 解析云端数据端点响应：200 + 判别头 ⇒ reject 单键 map 解析值（与
 * HttpTransport invoke 的错误解包同构——桌面 invoke rejection 载荷同源）；
 * 401/429/404/413/5xx ⇒ reject 错误文案（非白名单状态即异常路径）。
 *
 * 评审修复（17.3 分诊）：
 * - 401 先发 `auth:unauthorized`（与 HttpTransport 同款全局信号——会话
 *   过期/导入清空会话后，AuthGate 带用户回登录页而非在备份区留死胡同
 *   红错；导入会整体替换会话表，导入成功后的下一个请求即 401 是常态）；
 * - 成功面强制 JSON：200 但非 JSON body（反代故障页等）按解析失败处理，
 *   不把 HTML 文本当结果返回（UI 会渲染出「已恢复 undefined 个角色」）。
 */
async function unwrapCloudResponse<T>(res: Response): Promise<T> {
  const text = await res.text();
  let parsed: unknown;
  let isJson = true;
  try {
    parsed = JSON.parse(text);
  } catch {
    isJson = false;
  }
  if (res.ok) {
    if (res.headers.get(APP_ERROR_HEADER)) {
      throw isJson ? parsed : new Error('云端数据端点返回了无法解析的错误响应');
    }
    if (!isJson) {
      throw new Error('云端数据端点响应不是有效的 JSON');
    }
    return parsed as T;
  }
  if (res.status === 401) {
    emitFrontendLocalEvent('auth:unauthorized');
  }
  throw new Error(`云端数据端点请求失败（HTTP ${res.status}）`);
}

export const dataService = {
  dataExport: (formats: ExportFormat[]) =>
    invoke<ExportResult>('data_export', { formats }),
  dataDestroy: () => invoke<void>('data_destroy'),
  pickImportFile: () => invoke<string | null>('pick_import_file'),
  dataImport: (filePath: string) =>
    invoke<ImportResult>('data_import', { filePath }),

  // ── Story 17.3：云端（浏览器宿主）逻辑级备份端点 ──
  // /api/export、/api/import 是 server 独立路由（不经 /api/cmd 分发——
  // data_export/data_import 保持 desktop-only），浏览器经相对路径 fetch
  //（credentials same-origin 自动携带会话 Cookie）。仅浏览器宿主调用
  //（设置页按 !isTauriHost() 门控）；密钥值永不进响应（仅 apiKeyRef）。

  /** 云端导出：GET /api/export → 导出包 JSON Blob（与桌面导出包同格式）。 */
  webExport: async (): Promise<Blob> => {
    const res = await fetch('/api/export', {
      method: 'GET',
      credentials: 'same-origin',
      signal: AbortSignal.timeout(WEB_BACKUP_TIMEOUT_MS),
    });
    if (res.ok && !res.headers.get(APP_ERROR_HEADER)) {
      return await res.blob();
    }
    // 错误面：与成功路径共用解包（AppError 单键 map / 非 200 白名单）
    throw await unwrapCloudResponse<never>(res);
  },

  /** 云端导入：上传导出包 JSON 文件 → POST /api/import（导入前服务端
   *  自动备份；响应含密钥缺失报告——重录指引由 UI 展示）。 */
  webImport: async (file: File): Promise<WebImportResult> => {
    // 体积预检（评审修复）：与 server MAX_BODY_BYTES=50MB 对齐——超限包
    // 先在本地拒绝，免去整包上传后才收 413（慢链路上传几分钟再失败）。
    const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;
    if (file.size > MAX_UPLOAD_BYTES) {
      throw new Error('导出包超过 50MB 上限（与服务端请求体上限一致）');
    }
    const res = await fetch('/api/import', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: await file.text(),
      signal: AbortSignal.timeout(WEB_BACKUP_TIMEOUT_MS),
    });
    return await unwrapCloudResponse<WebImportResult>(res);
  },
};
