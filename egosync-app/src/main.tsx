import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import { AuthGate } from './components/auth/AuthGate'
import { ButlerSettingsGroupedDemo } from './components/butler/ButlerSettingsGroupedDemo'
import { isTauriHost } from './transport'
import './index.css'

const params = new URLSearchParams(window.location.search)
const showSettingsDemo = window.location.pathname === '/settings-demo' || params.has('settings-demo')

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {/* Story 16.1：AuthGate 包裹 App——浏览器宿主先过认证门（setup/登录/
        401 拦截/登出），桌面宿主直通。settings-demo 演示分支留在 gate 外
        （独立演示页无认证语义），但仅桌面宿主渲染——web 宿主未认证访问
        会满屏 401 错误态，SPA 回退对其不可达。gate 先于任何事件订阅生效：
        App 仅在 ready 态挂载（未认证 ⇒ 零 useEngineEvent/SSE 订阅）。 */}
    {showSettingsDemo ? (isTauriHost() ? <ButlerSettingsGroupedDemo /> : null) : (
      <AuthGate>
        <App />
      </AuthGate>
    )}
  </React.StrictMode>,
)
