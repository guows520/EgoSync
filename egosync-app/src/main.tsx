import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.tsx'
import { AuthGate } from './components/auth/AuthGate'
import { ButlerSettingsGroupedDemo } from './components/butler/ButlerSettingsGroupedDemo'
import { isTauriHost, setTransportBoot } from './transport'
import type { DesktopBootConfig } from './transport'
import { applyBootConfig, isRemoteDesktop } from './appMode'
import { desktopGetBootConfig } from './services/desktopModeService'
import './index.css'

const params = new URLSearchParams(window.location.search)
const showSettingsDemo = window.location.pathname === '/settings-demo' || params.has('settings-demo')

// ── Story 16.3：渲染前异步引导 ──
// Tauri 宿主先经壳命令 desktop_get_boot_config 读模式（desktop-mode.json
// 事实源），再按模式注入 transport boot（remote ⇒ HttpTransport 指向远端）
// 与 appMode；浏览器宿主零壳调用（相对路径语义不变）。
//
// 等待期由 index.html 的 #egosync-splash 覆盖（z-9999——AuthGate 离开
// checking 态 / App ready 时各自移除；仅 settings-demo 演示分支在渲染后
// 兜底移除——它没有 gate/App 生命周期）。
async function bootstrap(): Promise<DesktopBootConfig | null> {
  if (!isTauriHost()) return null;
  try {
    const config = await desktopGetBootConfig();
    setTransportBoot(config);
    applyBootConfig(config);
    return config;
  } catch (e) {
    // 壳命令失败（IPC 异常等）fail-safe：按 local 落地（数据在本地——
    // 回退到有数据的一侧；desktop-mode.json 缺失同语义）
    console.error('桌面引导配置读取失败（回退本地模式）:', e);
    const fallback: DesktopBootConfig = { mode: 'local', remoteUrl: null, remoteToken: null };
    setTransportBoot(fallback);
    applyBootConfig(fallback);
    return fallback;
  }
}

function render() {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      {/* Story 16.1：AuthGate 包裹 App——浏览器宿主先过认证门（setup/登录/
          401 拦截/登出），桌面宿主直通。Story 16.3：远程桌面同走完整
          状态机（去直通——令牌重录/离线逃生口在此面）。settings-demo
          演示分支留在 gate 外（独立演示页无认证语义），但仅桌面宿主渲染
          ——web 宿主未认证访问会满屏 401 错误态，SPA 回退对其不可达。
          gate 先于任何事件订阅生效：App 仅在 ready 态挂载（未认证 ⇒
          零 useEngineEvent/SSE 订阅）。 */}
      {showSettingsDemo ? (isTauriHost() ? <ButlerSettingsGroupedDemo /> : null) : (
        <AuthGate>
          <App />
        </AuthGate>
      )}
    </React.StrictMode>,
  )
}

// Story 16.4：PWA Service Worker 注册唯一入口（外部模块——index.html 禁加
// inline script，CSP hash byte 契约钉死恰好 1 个内联脚本）。
// - 仅生产构建注册（dev 期 HMR/模块图每次都变，SW 缓存只会干扰开发）；
// - 仅浏览器宿主注册（桌面 Tauri 壳由 TauriTransport 供数，不走 SW——
//   桌面渲染/行为零影响）；
// - 注册失败（隐私模式等）静默降级：应用功能全可用，仅无离线外壳
//   （I/O 矩阵 SW_FAIL）。缓存纪律见 public/sw.js 头注释——业务数据
//   零落盘（NFR-C3）。
function registerServiceWorker() {
  if (!import.meta.env.PROD || isTauriHost()) return;
  try {
    void navigator.serviceWorker.register('/sw.js').catch(() => {
      // 隐私模式/不支持 SW：静默降级，不阻断引导
    });
  } catch {
    // navigator.serviceWorker 缺席（极端环境）——同上静默降级
  }
}

bootstrap().finally(() => {
  render()
  // [评审轮2 U12] splash 兜底仅限 settings-demo 演示分支：它无
  // AuthGate/App 生命周期（无人移除 splash——不撤则永盖）。正式应用
  // 路径的 splash 由 AuthGate（离开 checking 进 setup/login/offline）与
  // App（ready）按各自语义移除——原 400ms 无条件兜底在远程模式实为
  // 主路径（checking 网络往返远超 400ms，品牌启动屏被换成灰底文案）。
  if (showSettingsDemo && isTauriHost()) {
    const splash = document.getElementById('egosync-splash');
    if (splash) {
      splash.classList.add('hidden');
      setTimeout(() => splash.remove(), 400);
    }
  }
  // [评审轮2 U1] 远程桌面：认证界面（离线屏/令牌重录/初始化向导）全部
  // 发生在 App 挂载之前——App.tsx 的 show() 对远程态不可达（gate 未过
  // 则 App 永不挂载，窗口 visible:false 全程隐藏，用户看到「应用没
  // 打开」）。引导完成即显示窗口：splash 仍在覆盖（gate 离开 checking
  // 才撤），用户先见品牌屏再见认证界面；本地态保持 App ready 后首显
  //（防首帧闪白——既有设计零变化）。
  if (isRemoteDesktop()) {
    getCurrentWindow().show().catch(() => {});
  }
  // Story 16.4：SW 注册（render 之后——不抢首屏资源；内部按 PROD +
  // 浏览器宿主门控）
  registerServiceWorker()
})
