// Story 16.4 评审修复：MobileSettingsView 登出链路测试（新建文件）。
//
// 范式 = Sidebar.browser.test.tsx 浏览器分支三件套（删 __TAURI_INTERNALS__
// 桩 + __resetTransportForTests() + getTransport().on 订阅 auth:unauthorized）
// ——登出入口从侧栏搬迁到移动设置 tab 后，链路语义必须与侧栏逐项一致：
// - 点击登出行 → 确认弹窗 → authService.logout()（REST）→ 发射
//   auth:unauthorized（AuthGate 回登录页的信号源）；
// - 429（认证面限流）：不本地登出（不发射事件）+ 展示限流文案；
// - 桌面宿主（Tauri 桩）：登出行不渲染（15.5 直通裁决——桌面无认证面）。

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetTransportForTests, getTransport, HttpTransportError } from '@/transport';
import { MobileSettingsView } from './MobileSettingsView';
import { authService } from '../../services/authService';

vi.mock('../../services/authService', () => ({
  authService: {
    logout: vi.fn().mockResolvedValue(undefined),
  },
}));

function renderMobileSettings() {
  return render(
    <MobileSettingsView
      onOpenSettings={vi.fn()}
      theme="light"
      onToggleTheme={vi.fn()}
    />,
  );
}

describe('MobileSettingsView 登出链路（Story 16.4 浏览器分支）', () => {
  const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  describe('浏览器分支', () => {
    beforeEach(() => {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
      vi.mocked(authService.logout).mockReset().mockResolvedValue(undefined);
    });
    afterEach(() => {
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
      __resetTransportForTests();
    });

    it('登出行渲染于设置列表（侧栏控件搬迁落点）', () => {
      renderMobileSettings();
      expect(screen.getByTestId('settings-logout')).toBeInTheDocument();
    });

    it('点击登出 → 确认 → authService.logout 并发射 auth:unauthorized', async () => {
      const received: unknown[] = [];
      const unlisten = getTransport().on('auth:unauthorized', (payload) => received.push(payload));

      renderMobileSettings();
      fireEvent.click(screen.getByTestId('settings-logout'));
      // 确认弹窗（防误触——裁决「带确认」）
      const confirmButton = await screen.findByRole('button', { name: '确认登出' });
      fireEvent.click(confirmButton);

      await waitFor(() => {
        expect(authService.logout).toHaveBeenCalledTimes(1);
      });
      await waitFor(() => {
        expect(received).toHaveLength(1);
      });
      unlisten();
    });

    it('logout 429（限流）：展示限流文案且不本地登出（不发射 auth:unauthorized）', async () => {
      vi.mocked(authService.logout).mockRejectedValue(new HttpTransportError(429, { error: 'rate limited' }));
      const received: unknown[] = [];
      const unlisten = getTransport().on('auth:unauthorized', (payload) => received.push(payload));

      renderMobileSettings();
      fireEvent.click(screen.getByTestId('settings-logout'));
      fireEvent.click(await screen.findByRole('button', { name: '确认登出' }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('登出请求过于频繁，请稍后再试');
      });
      // 429 ⇒ 服务端会话仍存活，本地登出会造成「刷新静默复登」——不发射事件
      expect(received).toHaveLength(0);
      unlisten();
    });

    it('logout 网络失败降级：仍发射 auth:unauthorized（不锁死在失效界面）', async () => {
      vi.mocked(authService.logout).mockRejectedValue(new TypeError('network down'));
      const received: unknown[] = [];
      const unlisten = getTransport().on('auth:unauthorized', (payload) => received.push(payload));

      renderMobileSettings();
      fireEvent.click(screen.getByTestId('settings-logout'));
      fireEvent.click(await screen.findByRole('button', { name: '确认登出' }));

      await waitFor(() => {
        expect(received).toHaveLength(1);
      });
      unlisten();
    });
  });

  describe('桌面分支（默认 Tauri 桩）', () => {
    it('登出行不渲染（桌面宿主无认证面——15.5 直通裁决）', () => {
      renderMobileSettings();
      expect(screen.queryByTestId('settings-logout')).not.toBeInTheDocument();
      // 其余设置入口不受影响
      expect(screen.getByTestId('settings-row-llm')).toBeInTheDocument();
    });
  });
});
