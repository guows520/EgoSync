// Story 16.4：BottomTabBar 移动形态测试（真类名断言——jsdom 不匹配媒体查询，
// max-md:/md:hidden 属 CSS 层裁剪；互斥语义由存在性 + 类对 + 切换行为守门）。

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BottomTabBar } from './BottomTabBar';

describe('BottomTabBar（Story 16.4 移动端底部 tab）', () => {
  it('渲染管家/角色/设置三个 tab（UX 初案结构固定）', () => {
    render(
      <BottomTabBar
        activeTab="butler"
        onButlerTab={vi.fn()}
        onRolesTab={vi.fn()}
        onSettingsTab={vi.fn()}
      />,
    );

    expect(screen.getByTestId('bottom-tab-butler')).toHaveTextContent('管家');
    expect(screen.getByTestId('bottom-tab-roles')).toHaveTextContent('角色');
    expect(screen.getByTestId('bottom-tab-settings')).toHaveTextContent('设置');
  });

  it('与侧栏互斥：md:hidden 退场（≥768px 侧栏接管；Sidebar 侧 max-md:hidden 成对）', () => {
    render(
      <BottomTabBar
        activeTab="butler"
        onButlerTab={vi.fn()}
        onRolesTab={vi.fn()}
        onSettingsTab={vi.fn()}
      />,
    );

    const nav = screen.getByTestId('bottom-tab-bar');
    expect(nav).toHaveClass('md:hidden');
    // 底部安全区避让（viewport-fit=cover 的 env 模式，无 inset 设备退化为 1rem）
    expect(nav.className).toContain('pb-[max(1rem,env(safe-area-inset-bottom))]');
  });

  it('触控目标 ≥44px（min-h-[44px] + flex-1 全宽）', () => {
    render(
      <BottomTabBar
        activeTab="roles"
        onButlerTab={vi.fn()}
        onRolesTab={vi.fn()}
        onSettingsTab={vi.fn()}
      />,
    );

    for (const key of ['butler', 'roles', 'settings'] as const) {
      const tab = screen.getByTestId(`bottom-tab-${key}`);
      expect(tab).toHaveClass('min-h-[44px]', 'flex-1');
    }
  });

  it('激活态标记 aria-current=page（当前 tab 高亮 + a11y 可达）', () => {
    render(
      <BottomTabBar
        activeTab="settings"
        onButlerTab={vi.fn()}
        onRolesTab={vi.fn()}
        onSettingsTab={vi.fn()}
      />,
    );

    expect(screen.getByTestId('bottom-tab-settings')).toHaveAttribute('aria-current', 'page');
    expect(screen.getByTestId('bottom-tab-butler')).not.toHaveAttribute('aria-current');
  });

  it('点击各 tab 上抛对应回调（切换语义）', () => {
    const onButlerTab = vi.fn();
    const onRolesTab = vi.fn();
    const onSettingsTab = vi.fn();
    render(
      <BottomTabBar
        activeTab="butler"
        onButlerTab={onButlerTab}
        onRolesTab={onRolesTab}
        onSettingsTab={onSettingsTab}
      />,
    );

    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    fireEvent.click(screen.getByTestId('bottom-tab-settings'));
    fireEvent.click(screen.getByTestId('bottom-tab-butler'));

    expect(onRolesTab).toHaveBeenCalledTimes(1);
    expect(onSettingsTab).toHaveBeenCalledTimes(1);
    expect(onButlerTab).toHaveBeenCalledTimes(1);
  });
});
