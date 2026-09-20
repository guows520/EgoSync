// Story 16.1：Sidebar web-only 登出按钮的宿主门控与登出链路测试。
//
// 浏览器分支范式 = 删桩-恢复-`__resetTransportForTests()` 三件套
// （TitleBar.browser.test.tsx 同款）。登出无对应 invoke 命令（纯浏览器
// REST 语义），故宿主分支用 isTauriHost 探测而非 capabilities 门控。
//
// 覆盖：
// - 浏览器分支：登出按钮渲染；点击 → authService.logout()（REST）→
//   发射 auth:unauthorized（AuthGate 回登录页的信号源）；
// - 网络失败降级：logout reject 仍发射事件（不把用户锁死在失效界面）；
// - 桌面分支（默认桩）：登出入口不渲染（桌面无认证面——15.5 直通裁决）。

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetTransportForTests, getTransport } from '@/transport';
import { Sidebar } from './Sidebar';
import { authService } from '../../services/authService';
import type { Role } from '../../types/role';

vi.mock('../../services/authService', () => ({
  authService: {
    logout: vi.fn().mockResolvedValue(undefined),
  },
}));

const baseRole: Role = {
  id: 'role-1',
  name: '产品经理',
  icon: 'briefcase',
  color: '#4F46E5',
  goal: '打磨产品节奏',
  personalityPrompt: '',
  status: 'active',
  energy: 72,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

function renderSidebar() {
  return render(
    <Sidebar
      roles={[baseRole]}
      currentView="chat"
      onViewChange={vi.fn()}
      isSettingsOpen={false}
      onOpenSettings={vi.fn()}
      onCloseSettings={vi.fn()}
      onAddRole={vi.fn()}
      onArchiveRole={vi.fn()}
      onDeleteRole={vi.fn()}
      theme="light"
      onToggleTheme={vi.fn()}
      onEditRole={vi.fn()}
      isNotifOpen={false}
      onToggleNotif={vi.fn()}
    />
  );
}

describe('Sidebar 登出入口（Story 16.1 web-only）', () => {
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

    it('登出按钮渲染于底栏（主题切换旁）', () => {
      renderSidebar();
      expect(screen.getByRole('button', { name: '登出' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /切换深色模式/ })).toBeInTheDocument();
    });

    it('点击登出：调 authService.logout 并发射 auth:unauthorized', async () => {
      const received: unknown[] = [];
      const unlisten = getTransport().on('auth:unauthorized', (payload) => received.push(payload));

      renderSidebar();
      fireEvent.click(screen.getByRole('button', { name: '登出' }));

      await waitFor(() => {
        expect(authService.logout).toHaveBeenCalledTimes(1);
      });
      // auth:unauthorized 经前端本地总线扇出（AuthGate 监听回登录页）
      await waitFor(() => {
        expect(received).toHaveLength(1);
      });
      unlisten();
    });

    it('logout 网络失败降级：仍发射 auth:unauthorized（不锁死在失效界面）', async () => {
      vi.mocked(authService.logout).mockRejectedValue(new TypeError('network down'));
      const received: unknown[] = [];
      const unlisten = getTransport().on('auth:unauthorized', (payload) => received.push(payload));

      renderSidebar();
      fireEvent.click(screen.getByRole('button', { name: '登出' }));

      await waitFor(() => {
        expect(received).toHaveLength(1);
      });
      unlisten();
    });
  });

  describe('桌面分支（默认桩）', () => {
    it('登出入口不渲染（桌面宿主无认证面——15.5 直通裁决）', () => {
      renderSidebar();
      expect(screen.queryByRole('button', { name: '登出' })).not.toBeInTheDocument();
      // 底栏其余控件不受影响
      expect(screen.getByRole('button', { name: /切换深色模式/ })).toBeInTheDocument();
    });
  });
});
