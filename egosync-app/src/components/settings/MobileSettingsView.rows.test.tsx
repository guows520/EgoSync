// web 版手机浏览器五症状自适应修复·症状③：移动设置列表行钉孔（新建
// 文件——MobileSettingsView.browser.test.tsx 一行不改，冻结规矩「新测试
// 一律新建文件」）。
//
// 2026-09-26 owner 授权：移除「通知」行（与「调度时间」落点完全重复，桌面
// 本就无该项；敲门通知声音经「调度时间」tab 仍可达）。本测试钉：
// - 4 个内容入口 settings-row-{llm,mcp,scheduler,data} 在场；
// - settings-row-notification 不在场；
// - 主题 / 登出行不受影响（侧栏控件搬迁落点不变）。

import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetTransportForTests } from '@/transport';
import { MobileSettingsView } from './MobileSettingsView';

describe('MobileSettingsView 设置行（web 移动自适应修复）', () => {
  const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    // 浏览器分支（登出场渲染的前提）
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
  afterEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
  });

  it('4 个内容入口在场、「通知」行不在场', () => {
    render(
      <MobileSettingsView
        onOpenSettings={vi.fn()}
        theme="light"
        onToggleTheme={vi.fn()}
      />,
    );

    for (const key of ['llm', 'mcp', 'scheduler', 'data']) {
      expect(screen.getByTestId(`settings-row-${key}`)).toBeInTheDocument();
    }
    expect(screen.queryByTestId('settings-row-notification')).not.toBeInTheDocument();
  });

  it('主题 / 登出行不受影响', () => {
    render(
      <MobileSettingsView
        onOpenSettings={vi.fn()}
        theme="light"
        onToggleTheme={vi.fn()}
      />,
    );

    expect(screen.getByTestId('settings-theme-light')).toBeInTheDocument();
    expect(screen.getByTestId('settings-theme-dark')).toBeInTheDocument();
    expect(screen.getByTestId('settings-logout')).toBeInTheDocument();
  });
});
