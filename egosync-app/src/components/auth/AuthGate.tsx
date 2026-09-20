// Story 16.1：AuthGate——web 入口认证门（setup 向导 → 登录 → 401 全局拦截 → 登出）。
//
// 状态机：checking / setup / login / offline / ready。
// - **桌面宿主直通**：isTauriHost() ⇒ 初始即 ready（零 fetch、零事件订阅，
//   桌面行为零变化——15.5 交付的宿主门控只守护不重做）；
// - **gate 先于任何事件订阅**（硬约束）：children（App）仅在 ready 态挂载
//   ——未认证时零 useEngineEvent/SSE 订阅，杜绝未认证 401 的 SSE 重建循环；
// - `auth:unauthorized`（invoke 401 / 主动登出发射）⇒ 失效认证态缓存 +
//   回登录页（App 卸载即内存态清空——浏览器不落业务数据，无清理面）；
// - checking 态由 index.html 的 #egosync-splash 覆盖（z-9999）；离开
//   checking 进入 setup/login/offline 时移除 splash（同 App.tsx 机制）；
//   ready 态的 splash 移除归 App 自身（行为不变）。

import { ReactNode, useEffect, useState } from 'react';
import { getAuthStatus, getTransport, invalidateAuthStatusCache, isTauriHost } from '@/transport';
import { HttpTransportError } from '@/transport';
import { LoginView } from './LoginView';
import { SetupView } from './SetupView';
import {
  AUTH_RATE_LIMIT_MESSAGE,
  NETWORK_UNREACHABLE_MESSAGE,
  SERVER_ERROR_MESSAGE,
} from './authErrors';

type GateState = 'checking' | 'setup' | 'login' | 'offline' | 'ready';

export function AuthGate({ children }: { children: ReactNode }) {
  // 桌面宿主直通：gate 不 fetch、不订阅（状态机仅浏览器宿主运转）
  const [state, setState] = useState<GateState>(() =>
    isTauriHost() ? 'ready' : 'checking'
  );
  /** 离线态呈现文案（网络失败与限流退避两种成因）。 */
  const [offlineMessage, setOfflineMessage] = useState<string>(NETWORK_UNREACHABLE_MESSAGE);

  // status 检查（浏览器宿主）：checking → ready / setup / login / offline
  useEffect(() => {
    if (state !== 'checking') return;
    let cancelled = false;
    (async () => {
      try {
        const status = await getAuthStatus();
        if (cancelled) return;
        if (status.authenticated) setState('ready');
        else if (status.setupRequired) setState('setup');
        else setState('login');
      } catch (e) {
        if (cancelled) return;
        setOfflineMessage(checkFailureText(e));
        setState('offline');
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [state]);

  // auth:unauthorized 全局拦截：缓存失效 + 回登录页（App 卸载即内存清空）。
  // 浏览器宿主的前端本地事件总线订阅（Tauri 宿主 gate 直通不订阅）。
  useEffect(() => {
    if (isTauriHost()) return;
    const unlisten = getTransport().on('auth:unauthorized', () => {
      invalidateAuthStatusCache();
      setState('login');
    });
    return unlisten;
  }, []);

  // 离开 checking 态（进入 setup/login/offline）时移除 splash（同 App.tsx
  // 机制——splash z-9999 会盖住认证界面；ready 态归 App 自身处理）
  useEffect(() => {
    if (state === 'checking' || state === 'ready') return;
    const splash = document.getElementById('egosync-splash');
    let removeTimer: ReturnType<typeof setTimeout> | undefined;
    if (splash) {
      splash.classList.add('hidden');
      removeTimer = setTimeout(() => splash.remove(), 400);
    }
    // StrictMode 双触发/快速状态切换下清理悬空定时器（不操作已分离节点）
    return () => clearTimeout(removeTimer);
  }, [state]);

  /** 离线重试：回 checking 重新拉取 status（缓存只在 200 成功时写入，
   * 失败路径无脏缓存——重试必然真实重发）。 */
  const handleRetry = () => setState('checking');

  if (state === 'ready') {
    return <>{children}</>;
  }

  if (state === 'checking') {
    // 真实浏览器由 #egosync-splash 覆盖；无 splash 环境（测试）的兜底呈现
    return (
      <div className="h-screen flex items-center justify-center bg-[#F1F3F5] dark:bg-slate-800">
        <div className="text-slate-400 text-[14px]">正在检查登录状态...</div>
      </div>
    );
  }

  if (state === 'offline') {
    return (
      <div className="h-screen flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800">
        <div className="max-w-md text-center space-y-6 p-8">
          <div className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            {offlineMessage}
          </div>
          <button
            type="button"
            onClick={handleRetry}
            className="px-6 py-3 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity"
          >
            重试
          </button>
        </div>
      </div>
    );
  }

  if (state === 'setup') {
    return <SetupView onSuccess={() => setState('ready')} onSwitchToLogin={() => setState('login')} />;
  }

  return <LoginView onSuccess={() => setState('ready')} onSwitchToSetup={() => setState('setup')} />;
}

/** 检查失败文案：限流退避 / 服务端 5xx / 网络失败（I/O 矩阵——统一常量，与登录面同源）。 */
function checkFailureText(error: unknown): string {
  if (error instanceof HttpTransportError) {
    if (error.status === 429) return AUTH_RATE_LIMIT_MESSAGE;
    if (error.status >= 500) return SERVER_ERROR_MESSAGE;
  }
  return NETWORK_UNREACHABLE_MESSAGE;
}
