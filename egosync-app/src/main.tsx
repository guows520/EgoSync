import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import { ButlerSettingsGroupedDemo } from './components/butler/ButlerSettingsGroupedDemo'
import './index.css'

const params = new URLSearchParams(window.location.search)
const showSettingsDemo = window.location.pathname === '/settings-demo' || params.has('settings-demo')

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {showSettingsDemo ? <ButlerSettingsGroupedDemo /> : <App />}
  </React.StrictMode>,
)
