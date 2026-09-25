import { useState } from 'react';
import { Bell, Bot, ChevronRight, Clock, LogOut, Moon, Plug, ShieldCheck, Sun } from 'lucide-react';
import { emitFrontendEvent, HttpTransportError, isTauriHost } from '@/transport';
import { authService } from '../../services/authService';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';

// Story 16.4：移动端「设置」tab（线框定稿 spec-16-4-mobile-wireframe.html
// 屏 3）——桌面 GlobalSettingsModal 同款全量的移动入口：
// - 5 个内容入口（模型服务 / MCP Server / 调度时间 / 数据与隐私 / 通知）
//   打开的就是桌面同款组件（GlobalSettingsModal 对应 tab——同一批组件
//   不重写；通知 = 调度时间 tab 内的敲门通知声音设置）；
// - 主题 / 登出 = 侧栏专属控件搬迁（人工裁决 2026-09-25）：主题为首页
//   一行切换（浅色/深色，替代侧栏按钮）；登出在列表底部带确认（防误触）。
// 桌面专属（远程模式/手机伴侣）由 capabilities 门控在
// GlobalSettingsModal 内自动隐藏——本视图不重复列举。
// 登出链路与 Sidebar 同语义（authService.logout + 429 提示 +
// auth:unauthorized 回登录页；Sidebar 冻结零其他改动，故此处复刻）。

interface MobileSettingsViewProps {
  /** 打开桌面同款 GlobalSettingsModal 并落到指定 tab。 */
  onOpenSettings: (tab: 'llm' | 'mcp' | 'scheduler' | 'data') => void;
  theme: 'light' | 'dark';
  onToggleTheme: () => void;
}

export function MobileSettingsView({ onOpenSettings, theme, onToggleTheme }: MobileSettingsViewProps) {
  const [showLogoutConfirm, setShowLogoutConfirm] = useState(false);
  const [isLoggingOut, setIsLoggingOut] = useState(false);
  const [logoutError, setLogoutError] = useState('');

  // 与 Sidebar.handleLogout 同语义（429 不本地登出；其余失败照常本地登出）
  const handleLogout = async () => {
    if (isLoggingOut) return;
    setIsLoggingOut(true);
    setLogoutError('');
    let localLogout = true;
    try {
      await authService.logout();
    } catch (e) {
      if (e instanceof HttpTransportError && e.status === 429) {
        localLogout = false;
        setLogoutError('登出请求过于频繁，请稍后再试');
      }
    } finally {
      setIsLoggingOut(false);
      if (localLogout) {
        void emitFrontendEvent('auth:unauthorized');
      }
    }
  };

  const contentRows = [
    { key: 'llm', label: '模型服务', Icon: Bot, onClick: () => onOpenSettings('llm') },
    { key: 'mcp', label: 'MCP Server', Icon: Plug, onClick: () => onOpenSettings('mcp') },
    { key: 'scheduler', label: '调度时间', Icon: Clock, onClick: () => onOpenSettings('scheduler') },
    { key: 'data', label: '数据与隐私', Icon: ShieldCheck, onClick: () => onOpenSettings('data') },
    // 通知 = 调度时间 tab 内的敲门通知声音设置（桌面同款内容入口）
    { key: 'notification', label: '通知', Icon: Bell, onClick: () => onOpenSettings('scheduler') },
  ];

  return (
    <div className="h-full flex flex-col bg-white/40 dark:bg-slate-900/40">
      <header className="border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex items-center px-4 py-3 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100">设置</h2>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-2.5">
        {contentRows.map(({ key, label, Icon, onClick }) => (
          <button
            key={key}
            type="button"
            onClick={onClick}
            data-testid={`settings-row-${key}`}
            className="w-full min-h-[44px] flex items-center gap-3 rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-left transition-colors"
          >
            <Icon size={18} className="shrink-0 text-slate-400 dark:text-slate-500" />
            <span className="flex-1 text-[14px] font-medium text-slate-800 dark:text-slate-100">{label}</span>
            <ChevronRight size={16} className="shrink-0 text-slate-300 dark:text-slate-600" />
          </button>
        ))}

        {/* 主题：一行切换（浅色/深色）——替代侧栏主题按钮，触控 ≥44px */}
        <div className="w-full min-h-[44px] flex items-center gap-3 rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3">
          {theme === 'dark'
            ? <Moon size={18} className="shrink-0 text-slate-400 dark:text-slate-500" />
            : <Sun size={18} className="shrink-0 text-slate-400 dark:text-slate-500" />}
          <span className="flex-1 text-[14px] font-medium text-slate-800 dark:text-slate-100">主题</span>
          <div className="flex items-center gap-1" role="group" aria-label="主题切换">
            <button
              type="button"
              onClick={() => { if (theme !== 'light') onToggleTheme(); }}
              aria-pressed={theme === 'light'}
              data-testid="settings-theme-light"
              className={cn(
                'min-h-[44px] px-3 rounded-lg text-[13px] font-medium transition-colors',
                theme === 'light' ? 'bg-indigo-600 text-white' : 'text-slate-500 dark:text-slate-400',
              )}
            >
              浅色
            </button>
            <button
              type="button"
              onClick={() => { if (theme !== 'dark') onToggleTheme(); }}
              aria-pressed={theme === 'dark'}
              data-testid="settings-theme-dark"
              className={cn(
                'min-h-[44px] px-3 rounded-lg text-[13px] font-medium transition-colors',
                theme === 'dark' ? 'bg-indigo-600 text-white' : 'text-slate-500 dark:text-slate-400',
              )}
            >
              深色
            </button>
          </div>
        </div>

        {/* 登出：桌面宿主隐藏（15.5 直通裁决——桌面无认证面） */}
        {!isTauriHost() && (
          <button
            type="button"
            onClick={() => setShowLogoutConfirm(true)}
            data-testid="settings-logout"
            className="w-full min-h-[44px] flex items-center gap-3 rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-left transition-colors"
          >
            <LogOut size={18} className="shrink-0 text-red-500" />
            <span className="flex-1 text-[14px] font-medium text-red-600 dark:text-red-400">登出</span>
          </button>
        )}
      </div>

      {showLogoutConfirm && (
        <Modal onClose={() => setShowLogoutConfirm(false)} width="w-[400px]" ariaLabel="确认登出">
          <div className="p-6 space-y-5">
            <h3 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100">退出登录？</h3>
            <p className="text-[13px] text-slate-500 dark:text-slate-400 leading-relaxed">
              登出后将返回登录页，需要重新输入访问令牌。
            </p>
            {logoutError && <p role="alert" className="text-[13px] text-red-600 dark:text-red-400">{logoutError}</p>}
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowLogoutConfirm(false)}
                disabled={isLoggingOut}
                className="px-5 py-2.5 max-md:min-h-[44px] border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 rounded-lg text-[13px] font-medium hover:bg-slate-50 dark:hover:bg-slate-700 disabled:opacity-60"
              >
                取消
              </button>
              <button
                onClick={() => { void handleLogout(); }}
                disabled={isLoggingOut}
                className="px-5 py-2.5 max-md:min-h-[44px] rounded-lg text-[13px] font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-60"
              >
                {isLoggingOut ? '处理中...' : '确认登出'}
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
