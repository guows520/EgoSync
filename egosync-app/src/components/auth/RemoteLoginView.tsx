// Story 16.3：远程桌面令牌视图（AuthGate 远程分支的 login 态）。
//
// 与浏览器 LoginView 的差异（桌面远程模式语义）：
// - 令牌**不换发 Cookie**（SameSite=Strict 跨 tauri 源不可携带）——验证
//   即 `GET /api/auth/status`（Bearer 判定），通过后写 keyring 持久化
//   并重验（I/O 矩阵「令牌重录（验证→keyring→重验）」）；
// - 401 = 令牌不正确（如实文案，可重试）；网络失败 = 服务器不可达
//   （**绝不误报令牌失效**——网络错误≠401 严格分流）；
// - 「切回本地」逃生口（I/O 矩阵：离线/令牌失效时均可回本地模式）。
//
// 消费路径：AuthGate 在远程桌面收到 401（auth:unauthorized）或引导时
// keyring 令牌缺失/失效 ⇒ 本视图；验证成功 ⇒ onSuccess（gate 进 ready）。

import { FormEvent, useState } from 'react';
import { KeyRound, ArrowLeft } from 'lucide-react';
import { getAuthStatus, HttpTransportError, updateRemoteToken } from '@/transport';
import { remoteModeSaveConfig } from '../../services/desktopModeService';
import { loginErrorText } from './authErrors';

interface RemoteLoginViewProps {
  /** 远程实例 base URL（展示——模式文件持久值）。 */
  remoteUrl: string;
  /** 令牌验证并持久化成功（gate 进 ready）。 */
  onSuccess: () => void;
  /** 切回本地模式（确认后 save+restart——逃生口）。 */
  onSwitchToLocal: () => void;
}

export function RemoteLoginView({ remoteUrl, onSuccess, onSwitchToLocal }: RemoteLoginViewProps) {
  const [token, setToken] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState('');
  const [isSwitching, setIsSwitching] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = token.trim();
    if (!trimmed || isSubmitting) return;
    setIsSubmitting(true);
    setError('');
    try {
      // 1) 验证：Bearer 判定（不落任何持久面——失败可重试零残留）
      const ok = await verifyToken(remoteUrl, trimmed);
      if (!ok) {
        setError('令牌不正确，请重试。');
        return;
      }
      // 2) 持久化：keyring（经壳命令 remote_mode_save_config——模式文件
      //    同步重写 remote + URL，幂等）
      await remoteModeSaveConfig({ mode: 'remote', remoteUrl, token: trimmed });
      // 3) 传输实例换令牌（就地——订阅面不断裂）后经传输通道重验
      //    （I/O 矩阵「验证→keyring→重验」——重验即真实业务通道的
      //    Bearer 判定，验证与持久化之间远端令牌轮换在此如实呈现）
      updateRemoteToken(trimmed);
      const recheck = await getAuthStatus();
      if (!recheck.authenticated) {
        setError('令牌保存后重验失败，请重试。');
        return;
      }
      onSuccess();
    } catch (err) {
      // 网络失败/5xx：如实呈现（不是令牌失效——网络错误≠401 严格分流）
      setError(loginErrorText(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSwitchToLocal = async () => {
    if (isSwitching) return;
    setIsSwitching(true);
    try {
      await onSwitchToLocal();
    } catch (e) {
      setError(`切回本地失败：${e instanceof Error ? e.message : String(e)}`);
      setIsSwitching(false);
    }
  };

  return (
    <div className="h-screen supports-[height:100dvh]:h-dvh flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800 transition-colors duration-300">
      <div className="max-w-md w-full mx-4 text-center space-y-6 p-8">
        <div className="w-16 h-16 rounded-2xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-sm">
          <KeyRound size={32} strokeWidth={2} />
        </div>
        <div className="space-y-2">
          <h2 className="text-xl font-semibold text-slate-800 dark:text-slate-100">连接远程实例</h2>
          <p className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            输入远程实例的访问令牌继续。令牌将保存在系统钥匙串中。
          </p>
          <p className="text-slate-400 dark:text-slate-500 text-[12px] break-all" data-testid="remote-login-url">
            {remoteUrl}
          </p>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4 text-left" noValidate>
          <div>
            <label
              htmlFor="egosync-remote-token"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              访问令牌
            </label>
            <input
              id="egosync-remote-token"
              type="password"
              value={token}
              onChange={e => setToken(e.target.value)}
              placeholder="请输入实例访问令牌"
              autoComplete="current-password"
              autoFocus
              disabled={isSubmitting}
              aria-invalid={error ? true : undefined}
              className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
            />
          </div>
          {error && (
            <div role="alert" className="text-[13px] text-red-600 dark:text-red-400 leading-relaxed">
              {error}
            </div>
          )}
          <button
            type="submit"
            disabled={!token.trim() || isSubmitting}
            className="w-full px-6 py-3 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSubmitting ? '验证中...' : '验证并进入'}
          </button>
        </form>
        <div className="pt-2 border-t border-slate-200/60 dark:border-slate-700/60">
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
 * 令牌验证（Bearer 判定）：`GET /api/auth/status` 携候选令牌。
 *
 * 直连 fetch（不经传输单例——单例持有引导期令牌，候选令牌须独立验证）；
 * 网络失败 ⇒ HttpTransportError(0, null)（loginErrorText 分流为断网文案
 * ——网络错误≠401 严格分流的视图侧落点）；401 ⇒ HttpTransportError(401)
 * ——**但本函数只在 status 200 时返回布尔**，非 200 统一上抛由
 * loginErrorText 分流（401 ⇒ 「令牌不正确」）。
 */
async function verifyToken(baseUrl: string, token: string): Promise<boolean> {
  const url = `${baseUrl.replace(/\/+$/, '')}/api/auth/status`;
  const res = await fetch(url, {
    credentials: 'same-origin',
    headers: { Authorization: `Bearer ${token}` },
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
  const text = await res.text();
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = {};
  }
  const status = parsed as { authenticated?: unknown };
  return status.authenticated === true;
}
