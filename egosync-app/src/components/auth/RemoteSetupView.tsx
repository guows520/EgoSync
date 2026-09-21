// Story 16.3 [T1 修订]：远程桌面 setup 视图（AuthGate 远程分支的 setup 态）。
//
// **不得复用浏览器 SetupView**（spec 冻结矩阵第 5 行根因）：SetupView 的
// authService.setup 走相对路径 fetch——在 `tauri://localhost` webview 源下
// 相对路径不可达远端实例（提交必失败的死胡同界面）；本视图以绝对 URL
// 直连远端。
//
// 流程（「未初始化实例无令牌可验——此调用无 Bearer」）：
// 1. `POST {base}/api/setup {token}`（直连 fetch、无 Authorization 头——
//    实例尚未初始化，不存在可验令牌）；
// 2. 成功后令牌即主令牌：写 keyring（remote_mode_save_config mode=remote，
//    模式文件幂等重写）→ updateRemoteToken（传输就地换令牌）→
//    getAuthStatus 重验（Bearer 判定）→ ready；
// 3. 失败如实呈现：网络失败（断网文案——不误报令牌问题）/ 401（令牌
//    长度不足）/ 404（实例已初始化或 env 锁定——统一锁定说明 + 返回
//    登录入口，不自动重试）/ 429 / 5xx；
// 4. setup 态含「切回本地」逃生口（I/O 矩阵「可中途切回本地」）。
//
// 视觉与 SetupView/RemoteLoginView 同语言（居中卡片 + 桌面壳语义）。

import { FormEvent, useState } from 'react';
import { Home, ArrowLeft } from 'lucide-react';
import { getAuthStatus, HttpTransportError, updateRemoteToken } from '@/transport';
import { toErrorMessage } from '../../lib/errorMessage';
import { remoteModeSaveConfig } from '../../services/desktopModeService';
import { setupErrorText } from './authErrors';

/** setup 令牌最短长度（与服务端 SETUP_TOKEN_MIN_LEN 同源约定）。 */
const SETUP_TOKEN_MIN_LEN = 8;

interface RemoteSetupViewProps {
  /** 远程实例 base URL（模式文件持久值——直连目标与展示同源）。 */
  remoteUrl: string;
  /** setup + 重验成功（gate 进 ready）。 */
  onSuccess: () => void;
  /** 返回登录页（env 锁定/已初始化的 404 路径或用户主动返回——令牌重录）。 */
  onSwitchToLogin: () => void;
  /** 切回本地模式（确认后 save+restart——setup 态逃生口）。[评审轮2 U9]
   * 如实声明 Promise：实参是失败 rethrow 的 async 函数（见 RemoteLoginView）。 */
  onSwitchToLocal: () => Promise<void>;
}

export function RemoteSetupView({ remoteUrl, onSuccess, onSwitchToLogin, onSwitchToLocal }: RemoteSetupViewProps) {
  const [token, setToken] = useState('');
  const [confirmToken, setConfirmToken] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState('');
  /** 404 统一锁定说明的显性返回登录入口（不自动重试）。 */
  const [showBackToLogin, setShowBackToLogin] = useState(false);
  const [isSwitching, setIsSwitching] = useState(false);

  const trimmed = token.trim();
  const confirmTrimmed = confirmToken.trim();
  const preValidationError =
    trimmed.length === 0
      ? ''
      : trimmed.length < SETUP_TOKEN_MIN_LEN
        ? `令牌至少 ${SETUP_TOKEN_MIN_LEN} 位字符`
        : confirmTrimmed.length > 0 && confirmTrimmed !== trimmed
          ? '两次输入的令牌不一致'
          : '';

  const canSubmit = trimmed.length >= SETUP_TOKEN_MIN_LEN && confirmTrimmed === trimmed && !isSubmitting;

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    setIsSubmitting(true);
    setError('');
    setShowBackToLogin(false);
    try {
      // 1) 绝对 URL 直连 setup（无 Bearer——未初始化实例无令牌可验）
      await setupRemoteInstance(remoteUrl, trimmed);
      // 2) 令牌即主令牌：写 keyring（模式文件幂等重写 remote + URL）
      await remoteModeSaveConfig({ mode: 'remote', remoteUrl, token: trimmed });
      // 3) 传输实例换令牌（就地——订阅面不断裂；认证缓存随换令牌失效）
      //    后经传输通道重验（Bearer 判定——验证与持久化之间远端令牌
      //    轮换在此如实呈现）
      updateRemoteToken(trimmed);
      const recheck = await getAuthStatus();
      if (!recheck.authenticated) {
        setError('令牌保存后重验失败，请重试。');
        return;
      }
      onSuccess();
    } catch (err) {
      setError(setupErrorText(err));
      // 404 = env 锁定/已初始化——统一锁定说明 + 返回登录（不自动重试）
      if (err instanceof HttpTransportError && err.status === 404) {
        setShowBackToLogin(true);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  /** 切回本地（[T3] 同款：失败 rethrow 由 AuthGate 分流，finally 复位）。 */
  const handleSwitchToLocal = async () => {
    if (isSwitching) return;
    setIsSwitching(true);
    try {
      await onSwitchToLocal();
    } catch (err) {
      // [评审轮2 U15] AppError 单键对象 reject ⇒ 首值（非 [object Object]）
      setError(`切回本地失败：${toErrorMessage(err)}`);
    } finally {
      setIsSwitching(false);
    }
  };

  return (
    <div
      className="h-screen supports-[height:100dvh]:h-dvh flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800 transition-colors duration-300"
      data-testid="remote-setup-view"
    >
      <div className="max-w-md w-full mx-4 text-center space-y-6 p-8">
        <div className="w-16 h-16 rounded-2xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-sm">
          <Home size={32} strokeWidth={2} />
        </div>
        <div className="space-y-2">
          <h2 className="text-xl font-semibold text-slate-800 dark:text-slate-100">初始化远程 EgoSync 实例</h2>
          <p className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            该实例尚未初始化：设置一个访问令牌（至少 {SETUP_TOKEN_MIN_LEN} 位字符）。
            <br />
            它是今后访问该实例的唯一凭据，将保存在系统钥匙串中，请妥善保管。
          </p>
          <p className="text-slate-400 dark:text-slate-500 text-[12px] break-all" data-testid="remote-setup-url">
            {remoteUrl}
          </p>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4 text-left" noValidate>
          <div>
            <label
              htmlFor="egosync-remote-setup-token"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              访问令牌
            </label>
            <input
              id="egosync-remote-setup-token"
              type="password"
              value={token}
              onChange={e => setToken(e.target.value)}
              placeholder={`至少 ${SETUP_TOKEN_MIN_LEN} 位字符`}
              autoComplete="new-password"
              autoFocus
              disabled={isSubmitting}
              aria-invalid={preValidationError || error ? true : undefined}
              className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
            />
          </div>
          <div>
            <label
              htmlFor="egosync-remote-setup-token-confirm"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              确认令牌
            </label>
            <input
              id="egosync-remote-setup-token-confirm"
              type="password"
              value={confirmToken}
              onChange={e => setConfirmToken(e.target.value)}
              placeholder="再输入一次"
              autoComplete="new-password"
              disabled={isSubmitting}
              aria-invalid={preValidationError || error ? true : undefined}
              className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
            />
          </div>
          {preValidationError && !error && (
            <div className="text-[13px] text-amber-600 dark:text-amber-400 leading-relaxed">
              {preValidationError}
            </div>
          )}
          {error && (
            <div role="alert" className="text-[13px] text-red-600 dark:text-red-400 leading-relaxed">
              {error}
            </div>
          )}
          <button
            type="submit"
            disabled={!canSubmit}
            className="w-full px-6 py-3 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSubmitting ? '初始化中...' : '完成初始化并进入'}
          </button>
        </form>
        <div className="pt-2 border-t border-slate-200/60 dark:border-slate-700/60 flex items-center justify-center gap-4">
          <button
            type="button"
            onClick={onSwitchToLogin}
            className="text-[13px] text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
          >
            {showBackToLogin ? '返回登录' : '已初始化？输入令牌登录'}
          </button>
          <span className="text-slate-300 dark:text-slate-600">|</span>
          <button
            type="button"
            onClick={handleSwitchToLocal}
            disabled={isSwitching}
            className="inline-flex items-center gap-1.5 text-[13px] text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors disabled:opacity-50"
          >
            <ArrowLeft size={14} className="shrink-0" />
            {isSwitching ? '正在切回本地...' : '切回本地模式'}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * 绝对 URL 直连 `POST {base}/api/setup {token}`（[T1] 远程桌面专属——
 * 浏览器 SetupView 的相对路径在 webview 源下不可达远端）。
 *
 * **无 Authorization 头**：未初始化实例无令牌可验（setup 是唯一免凭据的
 * 初始化写入面）。网络失败 ⇒ HttpTransportError(0)（setupErrorText 分流
 * 为断网文案——网络错误≠令牌问题严格分流）；非 200 ⇒ HttpTransportError
 * 上抛由 setupErrorText 分流（401 令牌过短 / 404 已初始化或 env 锁定 /
 * 429 限流 / 5xx 服务端错误）。
 */
async function setupRemoteInstance(baseUrl: string, token: string): Promise<void> {
  const url = `${baseUrl.replace(/\/+$/, '')}/api/setup`;
  const res = await fetch(url, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token }),
  }).catch(() => {
    throw new HttpTransportError(0, null);
  });
  if (res.status !== 200) {
    let body: unknown = null;
    try {
      body = JSON.parse(await res.text());
    } catch {
      body = null;
    }
    throw new HttpTransportError(res.status, body);
  }
}
