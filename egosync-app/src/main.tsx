import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import { AuthGate } from './components/auth/AuthGate'
import { ButlerSettingsGroupedDemo } from './components/butler/ButlerSettingsGroupedDemo'
import { isTauriHost, setTransportBoot } from './transport'
import type { DesktopBootConfig } from './transport'
import { applyBootConfig } from './appMode'
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
// checking 态 / App ready 时各自移除；引导失败兜底移除防白屏）。
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

bootstrap().finally(() => {
  render()
  // 引导完成即撤 splash 的兜底：正常路径 AuthGate/App 按各自语义移除；
  // 引导异常（壳命令 reject 后的 fail-safe 渲染）不至于让 splash 永盖。
  // 仅 Tauri 宿主执行——浏览器宿主 checking 态仍由品牌 splash 覆盖
  // （16.1 语义零回归），AuthGate/App 按既有机制各自移除。
  if (isTauriHost()) {
    const splash = document.getElementById('egosync-splash');
    if (splash) {
      splash.classList.add('hidden');
      setTimeout(() => splash.remove(), 400);
    }
  }
})
