// Story 16.1：登录视图——已初始化实例的未登录入口（AuthGate 的 login 态）。
//
// - 统一失败文案（401 不泄露存在性）/ 429 退避文案 / 断网文案（可重试提交）；
// - 次级「首次部署？」入口 → setup 向导（env 锁定/已初始化实例在 setup 侧
//   收到 404 统一锁定说明后可返回登录）；
// - 成功路径：login 200 → 失效认证态缓存 → onSuccess（gate 进 ready）。
//
// 视觉：OnboardingView 居中卡片设计语言（slate/indigo + 居中卡片 + 阴影），
// 主题跟随 `egosync-theme`（index.html 内联脚本已置 html.dark）。

import { FormEvent, useState } from 'react';
import { KeyRound } from 'lucide-react';
import { invalidateAuthStatusCache } from '@/transport';
import { authService } from '../../services/authService';
import { loginErrorText } from './authErrors';

const TOKEN_MIN_HINT = '请输入实例访问令牌';

interface LoginViewProps {
  /** 登录成功（gate 进 ready——App 挂载）。 */
  onSuccess: () => void;
  /** 次级入口：切换到 setup 向导（「首次部署？」）。 */
  onSwitchToSetup: () => void;
}

export function LoginView({ onSuccess, onSwitchToSetup }: LoginViewProps) {
  const [token, setToken] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = token.trim();
    if (!trimmed || isSubmitting) return;
    setIsSubmitting(true);
    setError('');
    try {
      await authService.login(trimmed);
      // 登录成功：缓存立即失效（15-5 G11 遗嘱——pre-login 的
      // authenticated:false 不得钉死进程期；ready 后如需重查按新态获取）
      invalidateAuthStatusCache();
      onSuccess();
    } catch (err) {
      // 统一失败文案（401/429/断网/5xx）——可重试（I/O 矩阵）
      setError(loginErrorText(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="h-screen flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800 transition-colors duration-300">
      <div className="max-w-md w-full mx-4 text-center space-y-6 p-8">
        <div className="w-16 h-16 rounded-2xl bg-slate-800 dark:bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-sm">
          <KeyRound size={32} strokeWidth={2} />
        </div>
        <div className="space-y-2">
          <h2 className="text-xl font-semibold text-slate-800 dark:text-slate-100">登录 EgoSync</h2>
          <p className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            输入实例的访问令牌继续。
          </p>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4 text-left" noValidate>
          <div>
            <label
              htmlFor="egosync-login-token"
              className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5"
            >
              访问令牌
            </label>
            <input
              id="egosync-login-token"
              type="password"
              value={token}
              onChange={e => setToken(e.target.value)}
              placeholder={TOKEN_MIN_HINT}
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
            {isSubmitting ? '登录中...' : '登录'}
          </button>
        </form>
        <div className="pt-2 border-t border-slate-200/60 dark:border-slate-700/60">
          <button
            type="button"
            onClick={onSwitchToSetup}
            className="text-[13px] text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
          >
            首次部署？设置访问令牌
          </button>
        </div>
      </div>
    </div>
  );
}
