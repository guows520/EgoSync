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
//
// Story 16.3 远程桌面分支：**去直通**——远程桌面跑完整状态机（与浏览器
// 等价 + 桌面专属逃生口）：
// - keyring 令牌在引导期已注入传输（main.tsx setTransportBoot）；
//   getAuthStatus 的 Bearer 判定分流 ready / setup / login；
// - login 态 = RemoteLoginView（验证→keyring→重验——非 Cookie 登录）；
// - setup 态 = RemoteSetupView（[T1] 绝对 URL 直连初始化——不复用相对
//   路径的浏览器 SetupView；含切回本地逃生口）；
// - 401 拦截 ⇒ RemoteLoginView（令牌重录）；offline 态附带「切回本地」
//   逃生口（I/O 矩阵）；
// - 本地桌面直通语义零变化（isRemoteDesktop 门控）。

import { ReactNode, useEffect, useState } from 'react';
import { getAuthStatus, getTransport, getTransportBoot, invalidateAuthStatusCache, isTauriHost } from '@/transport';
import { HttpTransportError } from '@/transport';
import { isRemoteDesktop } from '../../appMode';
import { remoteModeSaveConfig, remoteModeRestart } from '../../services/desktopModeService';
import { LoginView } from './LoginView';
import { RemoteLoginView } from './RemoteLoginView';
import { RemoteSetupView } from './RemoteSetupView';
import { SetupView } from './SetupView';
import {
  AUTH_RATE_LIMIT_MESSAGE,
  NETWORK_UNREACHABLE_MESSAGE,
  SERVER_ERROR_MESSAGE,
} from './authErrors';

type GateState = 'checking' | 'setup' | 'login' | 'offline' | 'ready';

export function AuthGate({ children }: { children: ReactNode }) {
  // 桌面宿主直通：gate 不 fetch、不订阅（状态机仅浏览器宿主运转）。
  // 远程桌面（16.3）：去直通——完整状态机（桌面壳 + 远端客户端）。
  const remote = isRemoteDesktop();
  const [state, setState] = useState<GateState>(() =>
    isTauriHost() && !remote ? 'ready' : 'checking'
  );
  /** 离线态呈现文案（网络失败与限流退避两种成因）。 */
  const [offlineMessage, setOfflineMessage] = useState<string>(NETWORK_UNREACHABLE_MESSAGE);
  /** 切回本地的确认与执行态（远程桌面逃生口）。 */
  const [isSwitchingLocal, setIsSwitchingLocal] = useState(false);

  // status 检查（浏览器宿主 / 远程桌面）：checking → ready / setup / login / offline
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
  // 浏览器宿主与远程桌面的前端本地事件总线订阅（本地桌面 gate 直通不订阅）。
  useEffect(() => {
    if (isTauriHost() && !remote) return;
    const unlisten = getTransport().on('auth:unauthorized', () => {
      invalidateAuthStatusCache();
      setState('login');
    });
    return unlisten;
  }, [remote]);

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

  /** 切回本地（远程桌面逃生口）：save(local) + restart——I/O 矩阵
   * 「远程不可达→切回本地」。remoteUrl 保留（下次切换预填——Rust 命令
   * 契约「local 态可保留」）；失败如实呈现（不静默——重启失败可重试）。
   *
   * [T3 修订] 失败后 rethrow：RemoteLoginView 侧 catch 生效（错误就地
   * 可见 + finally 复位按钮）——原实现内部吞错使 Promise 永不 reject，
   * RemoteLoginView 的 catch 成死代码、isSwitching 永不复位（按钮停在
   * 「正在切回本地...」永久禁用）。离线屏路径错误已由 offlineMessage
   * 呈现（经 handleOfflineSwitchToLocal 包裹吞掉 unhandled rejection）。 */
  const handleSwitchToLocal = async () => {
    setIsSwitchingLocal(true);
    try {
      await remoteModeSaveConfig({ mode: 'local', remoteUrl: getTransportBoot()?.remoteUrl ?? null });
      await remoteModeRestart();
      // restart 不返回（进程退出重启）——此处实际不可达
    } catch (e) {
      setIsSwitchingLocal(false);
      setOfflineMessage(`切回本地失败：${e instanceof Error ? e.message : String(e)}`);
      throw e;
    }
  };

  /** 离线屏切回本地入口：错误经 offlineMessage 呈现（本视图消费面），
   * rethrow 的 unhandled rejection 就地吞掉（[T3] 配套）。 */
  const handleOfflineSwitchToLocal = () => {
    void handleSwitchToLocal().catch(() => undefined);
  };

  if (state === 'ready') {
    return <>{children}</>;
  }

  if (state === 'checking') {
    // 真实浏览器由 #egosync-splash 覆盖；无 splash 环境（测试）的兜底呈现
    return (
      <div className="h-screen supports-[height:100dvh]:h-dvh flex items-center justify-center bg-[#F1F3F5] dark:bg-slate-800">
        <div className="text-slate-400 text-[14px]">正在检查登录状态...</div>
      </div>
    );
  }

  if (state === 'offline') {
    return (
      <div className="h-screen supports-[height:100dvh]:h-dvh flex flex-col items-center justify-center bg-[#F1F3F5] dark:bg-slate-800">
        <div className="max-w-md text-center space-y-6 p-8">
          <div className="text-slate-600 dark:text-slate-300 text-[14px] leading-relaxed">
            {offlineMessage}
          </div>
          <div className="flex items-center justify-center gap-3">
            <button
              type="button"
              onClick={handleRetry}
              className="px-6 py-3 bg-slate-800 dark:bg-indigo-600 text-white rounded-xl text-[14px] font-medium shadow-sm hover:opacity-90 transition-opacity"
            >
              重试
            </button>
            {/* 远程桌面离线逃生口（I/O 矩阵「远程不可达」行）——仅远程态呈现。
                [T3] handleSwitchToLocal 失败会 rethrow（供 RemoteLoginView
                分流）——离线屏路径经 handleOfflineSwitchToLocal 包裹吞掉
                rejection（错误已由 offlineMessage 呈现）。 */}
            {remote && (
              <button
                type="button"
                onClick={handleOfflineSwitchToLocal}
                disabled={isSwitchingLocal}
                className="px-6 py-3 border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 rounded-xl text-[14px] font-medium hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors disabled:opacity-50"
              >
                {isSwitchingLocal ? '正在切回...' : '切回本地模式'}
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (state === 'setup') {
    // [T1 修订] 远程桌面专属 setup 流：远端 setupRequired 时**不得**复用
    // 浏览器 SetupView（相对路径 fetch 在 tauri://localhost 源下不可达
    // 远端——提交必失败的死胡同界面）；RemoteSetupView 走绝对 URL 直连
    // 初始化（无 Bearer——未初始化实例无令牌可验），成功后令牌即主令牌
    // （keyring → 传输换令牌 → 重验）；setup 态含「切回本地」逃生口
    //（I/O 矩阵「可中途切回本地」）。
    if (remote) {
      return (
        <RemoteSetupView
          remoteUrl={getTransportBoot()?.remoteUrl ?? ''}
          onSuccess={() => setState('ready')}
          onSwitchToLogin={() => setState('login')}
          onSwitchToLocal={handleSwitchToLocal}
        />
      );
    }
    return <SetupView onSuccess={() => setState('ready')} onSwitchToLogin={() => setState('login')} />;
  }

  // login 态：远程桌面 = 令牌重录（验证→keyring→重验）；浏览器 = Cookie 登录
  if (remote) {
    // 引导传输的远端地址（模式文件持久值——RemoteLoginView 展示与保存同源）
    const remoteUrl = getTransportBoot()?.remoteUrl ?? '';
    return (
      <RemoteLoginView
        remoteUrl={remoteUrl}
        onSuccess={() => setState('ready')}
        onSwitchToLocal={handleSwitchToLocal}
      />
    );
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
