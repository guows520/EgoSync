// Story 16.1：setup 向导——未初始化实例的首访引导（AuthGate 的 setup 态）。
//
// - 令牌 ≥8 字符前端预校验（后端 <8 字符 401 如实呈现为对应文案）；
// - 成功路径：setup 200 ⇒ 随即自动携令牌 login → 缓存失效 → onSuccess
//   （gate 进 ready——免用户二次输入）；
// - env 锁定态 / 已初始化实例：setup 404 ⇒ 统一锁定说明（不泄露两态
//   区分），提供返回登录入口，不自动重试；
// - 视觉与 LoginView 同语言（OnboardingView 居中卡片）。

import { FormEvent, useState } from 'react';
import { Home } from 'lucide-react';
import { HttpTransportError, invalidateAuthStatusCache } from '@/transport';
import { authService } from '../../services/authService';
import { loginErrorText, setupErrorText } from './authErrors';

/** setup 令牌最短长度（与服务端 SETUP_TOKEN_MIN_LEN 同源约定）。 */
const SETUP_TOKEN_MIN_LEN = 8;

interface SetupViewProps {
  /** setup + 自动登录成功（gate 进 ready）。 */
  onSuccess: () => void;
  /** 返回登录页（env 锁定/已初始化的 404 路径或用户主动返回）。 */
  onSwitchToLogin: () => void;
}

export function SetupView({ onSuccess, onSwitchToLogin }: SetupViewProps) {
  const [token, setToken] = useState('');
  const [confirmToken, setConfirmToken] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState('');
  /** 404 统一锁定说明的显性返回入口（I/O 矩阵：env 锁定态不自动重试）。 */
  const [showBackToLogin, setShowBackToLogin] = useState(false);

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
      await authService.setup(trimmed);
      // setup 成功 ⇒ 自动携令牌 login（免二次输入）→ 进入应用
      try {
        await authService.login(trimmed);
        invalidateAuthStatusCache();
        onSuccess();
      } catch (loginErr) {
        // 初始化已成功、自动登录失败（网络抖动等）：如实呈现，引导手登
        // （内层按登录面分流——setup 面文案如「令牌长度不足」在此语境误导）
        setError(
          `初始化已完成，但自动登录失败（${loginErrorText(loginErr)}）。请返回登录页手动登录。`
        );
        setShowBackToLogin(true);
      }
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

  return (
    <div className="h-screen supports-[height:100dvh]:h-dvh flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800 transition-colors duration-300">
      <div className="max-w-md w-full mx-4 text-center space-y-6 p-8">
        <div className="w-16 h-16 rounded-2xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-sm">
          <Home size={32} strokeWidth={2} />
        </div>
        <div className="space-y-2">
          <h2 className="text-xl font-semibold text-slate-800 dark:text-slate-100">初始化 EgoSync 实例</h2>
          <p className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            首次使用：设置一个访问令牌（至少 {SETUP_TOKEN_MIN_LEN} 位字符）。
            <br />
            它是今后登录本实例的唯一凭据，请妥善保管。
          </p>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4 text-left" noValidate>
          <div>
            <label
              htmlFor="egosync-setup-token"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              访问令牌
            </label>
            <input
              id="egosync-setup-token"
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
              htmlFor="egosync-setup-token-confirm"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              确认令牌
            </label>
            <input
              id="egosync-setup-token-confirm"
              type="password"
              value={confirmToken}
              onChange={e => setConfirmToken(e.target.value)}
              placeholder="再输入一次"
              autoComplete="new-password"
              disabled={isSubmitting}
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
        <div className="pt-2 border-t border-slate-200/60 dark:border-slate-700/60">
          <button
            type="button"
            onClick={onSwitchToLogin}
            className="text-[13px] text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
          >
            {showBackToLogin ? '返回登录' : '已有实例？返回登录'}
          </button>
        </div>
      </div>
    </div>
  );
}
