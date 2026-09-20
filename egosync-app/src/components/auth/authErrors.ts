// Story 16.1：认证面错误文案——统一、诚实、不泄露实例/用户存在性。
//
// - env 锁定与已初始化在协议上不可区分（15.4 特性）——setup 404 用同一句
//   文案覆盖两态（「实例已初始化或已由环境变量锁定」）；
// - 登录失败统一 401 不泄露存在性——统一一句「令牌不正确」；
// - 认证面限流 429 与网络失败（status 0）各自有专属退避文案。

import { HttpTransportError } from '@/transport';

/** 认证面限流（429）统一文案（I/O 矩阵钉死）。 */
export const AUTH_RATE_LIMIT_MESSAGE = '尝试过于频繁，请稍后再试';

/** setup 404 统一锁定说明（env 锁定态与已初始化态不可区分——特性）。 */
export const SETUP_LOCKED_MESSAGE = '实例已初始化或已由环境变量锁定，无法再次初始化。';

/** 登录失败统一文案（401——不泄露实例/用户存在性）。 */
export const LOGIN_FAILED_MESSAGE = '令牌不正确，请重试。';

/** setup 令牌长度不足（后端 401 如实呈现——前端预校验通常已拦住）。 */
export const SETUP_SHORT_TOKEN_MESSAGE = '令牌长度不足，请设置至少 8 位字符。';

/** 网络不可达统一文案。 */
export const NETWORK_UNREACHABLE_MESSAGE = '无法连接服务器，请检查网络后重试。';

/** 服务端 5xx 统一文案。 */
export const SERVER_ERROR_MESSAGE = '服务暂时不可用，请稍后重试。';

/** 认证面通用兜底文案。 */
export const AUTH_FALLBACK_MESSAGE = '操作失败，请稍后重试。';

/** 登录面错误分流（按 HttpTransportError.status）。 */
export function loginErrorText(error: unknown): string {
  if (error instanceof HttpTransportError) {
    if (error.status === 0) return NETWORK_UNREACHABLE_MESSAGE;
    if (error.status === 401) return LOGIN_FAILED_MESSAGE;
    if (error.status === 429) return AUTH_RATE_LIMIT_MESSAGE;
    if (error.status >= 500) return SERVER_ERROR_MESSAGE;
  }
  return AUTH_FALLBACK_MESSAGE;
}

/** setup 面错误分流（404 = env 锁定/已初始化——统一锁定说明）。 */
export function setupErrorText(error: unknown): string {
  if (error instanceof HttpTransportError) {
    if (error.status === 0) return NETWORK_UNREACHABLE_MESSAGE;
    if (error.status === 401) return SETUP_SHORT_TOKEN_MESSAGE;
    if (error.status === 404) return SETUP_LOCKED_MESSAGE;
    if (error.status === 429) return AUTH_RATE_LIMIT_MESSAGE;
    if (error.status >= 500) return SERVER_ERROR_MESSAGE;
  }
  return AUTH_FALLBACK_MESSAGE;
}
