// Story 16.4：App 移动形态测试——侧栏/底栏互斥切换、角色 tab 两级、
// 设置 tab 打开桌面同款 GlobalSettingsModal（initialTab 落点）。
//
// 范式：App.test.tsx 同款 mock 集（壳命令/服务层/useEngineEvent），但
// Sidebar/BottomTabBar/RoleListPanel/MobileSettingsView 与 RoleView+
// RoleHeader 走真实组件（Only 叶子 ChatStream/RoleWorkspacePanel 打桩）——
// 互斥语义断言真类名（max-md:hidden/md:hidden 成对）与真实两级状态机。
// jsdom 不匹配媒体查询 ⇒ md:hidden 元素仍在 DOM：断言走「条件类 +
// 条件渲染」双通道（列表根为条件渲染——不在场即不在 DOM）。

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import App from './App';
import { roleService } from './services/roleService';
import { useEngineEvent } from './hooks/useEngineEvent';
import { __resetTransportForTests } from './transport';
import type { Role } from './types/role';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    show: vi.fn().mockResolvedValue(undefined),
    isMaximized: vi.fn().mockResolvedValue(false),
    onResized: vi.fn().mockResolvedValue(() => {}),
    minimize: vi.fn().mockResolvedValue(undefined),
    toggleMaximize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock('./services/appService', () => ({
  appService: {
    isFirstLaunch: vi.fn(),
    completeOnboarding: vi.fn(),
    isLlmConfigured: vi.fn(),
  },
}));

vi.mock('./services/roleService', () => ({
  roleService: {
    create: vi.fn(),
    list: vi.fn(),
    listArchived: vi.fn(),
    update: vi.fn(),
    archive: vi.fn(),
    restore: vi.fn(),
    delete: vi.fn(),
  },
}));

vi.mock('./hooks/useEngineEvent', () => ({
  useEngineEvent: vi.fn(),
}));

vi.mock('./components/butler/ButlerView', () => ({
  ButlerView: () => <div data-testid="butler-view">管家视图</div>,
}));

// 叶子打桩：RoleView + RoleHeader 真实（角色 tab 两级/「⋯」菜单的真路径）
vi.mock('./components/chat/ChatStream', () => ({
  ChatStream: () => <div data-testid="role-chat-stream">对话流</div>,
}));
vi.mock('./components/role/RoleWorkspacePanel', () => ({
  RoleWorkspacePanel: () => <div data-testid="role-workspace-panel">工作区</div>,
}));

vi.mock('./components/settings/GlobalSettingsModal', () => ({
  GlobalSettingsModal: ({ initialTab }: any) => (
    <div data-testid="settings-modal" data-initial-tab={initialTab ?? 'llm'}>全局设置</div>
  ),
}));

vi.mock('./components/modals/WeeklyReviewModal', () => ({
  WeeklyReviewModal: () => <div>复盘</div>,
}));
vi.mock('./components/modals/TaskModal', () => ({
  TaskModal: () => <div>任务</div>,
}));
vi.mock('./components/modals/AddRoleModal', () => ({
  AddRoleModal: () => <div>添加角色</div>,
}));
vi.mock('./components/notifications/NotificationPanel', () => ({
  NotificationPanel: () => <div>通知</div>,
}));
vi.mock('./components/onboarding/OnboardingView', () => ({
  OnboardingView: () => <div>引导视图</div>,
}));

const createdRole: Role = {
  id: 'role-fitness',
  name: '健身教练',
  icon: 'dumbbell',
  color: '#10B981',
  goal: '保持稳定训练',
  personalityPrompt: '',
  status: 'active',
  energy: 100,
  skillsConfig: '{}',
  proactivityLevel: 'moderate',
  archivedAt: null,
  createdAt: '2026-05-30T00:00:00Z',
  updatedAt: '2026-05-30T00:00:00Z',
};

describe('App 移动形态（Story 16.4）', () => {
  const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(roleService.list).mockResolvedValue([createdRole]);
    vi.mocked(roleService.listArchived).mockResolvedValue([]);
    vi.mocked(useEngineEvent).mockImplementation(() => {});
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    document.documentElement.classList.remove('dark');
  });

  async function renderApp() {
    const view = render(<App />);
    await waitFor(() => expect(roleService.list).toHaveBeenCalledTimes(1));
    return view;
  }

  it('侧栏与底栏互斥：Sidebar max-md:hidden / BottomTabBar md:hidden 成对，管家主区在场', async () => {
    await renderApp();

    const sidebar = screen.getByRole('navigation', { name: '角色导航' });
    expect(sidebar).toHaveClass('max-md:hidden');
    expect(sidebar).toHaveClass('w-16');

    const tabBar = screen.getByTestId('bottom-tab-bar');
    expect(tabBar).toHaveClass('md:hidden');

    expect(screen.getByTestId('butler-view')).toBeInTheDocument();
    expect(screen.getByTestId('bottom-tab-butler')).toHaveAttribute('aria-current', 'page');
  });

  it('角色 tab 两级：列表根 → 点行进详情 → 再点底部角色 tab 回根', async () => {
    const { container } = await renderApp();

    // 第一级：底部「角色」tab → 角色列表根（管家固定首行 + 角色行 + 「＋」）
    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    expect(screen.getByTestId('role-list-butler')).toBeInTheDocument();
    expect(screen.getByTestId('role-list-role-role-fitness')).toBeInTheDocument();
    expect(screen.getByTestId('role-list-add')).toBeInTheDocument();
    // 根呈现时管家视图让位（max-md:hidden——小屏）；当前行高亮管家
    expect(screen.getByTestId('butler-view').parentElement).toHaveClass('max-md:hidden');

    // 第二级：点角色行 → 进详情（RoleView 真渲染 + RoleHeader 头部）；
    // 管家视图随 currentView 切换整体卸载（无残留）
    fireEvent.click(screen.getByTestId('role-list-role-role-fitness'));
    expect(screen.queryByTestId('role-list-butler')).not.toBeInTheDocument();
    expect(screen.queryByTestId('butler-view')).not.toBeInTheDocument();
    expect(screen.getByText('健身教练')).toBeInTheDocument();
    expect(screen.getByTestId('bottom-tab-roles')).toHaveAttribute('aria-current', 'page');

    // 跨断点兜底：列表根呈现期间详情 wrapper 挂 max-md:hidden（小屏让位，
    // 桌面宽度下该类不生效 ⇒ 角色详情仍可见，不留空白主区）
    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    expect(screen.getByTestId('role-list-butler')).toBeInTheDocument();
    // RoleView 根（真实组件，animate-in fade-in）的父级 = App 的条件 wrapper
    const roleViewRoot = container.querySelector('.animate-in.fade-in');
    expect(roleViewRoot, '角色详情根应在场（max-md:hidden 仅小屏裁剪）').not.toBeNull();
    expect(roleViewRoot!.parentElement).toHaveClass('max-md:hidden');
  });

  it('角色详情「⋯」菜单：切换角色直达回列表根（线框屏 2b）', async () => {
    await renderApp();

    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    fireEvent.click(screen.getByTestId('role-list-role-role-fitness'));

    fireEvent.click(screen.getByTestId('role-more-menu'));
    fireEvent.click(screen.getByTestId('role-menu-switch'));

    // 回列表根：RoleListPanel 条件渲染在场 + 详情让位
    expect(screen.getByTestId('role-list-butler')).toBeInTheDocument();
  });

  it('角色详情「⋯」菜单：新建角色直达（AddRoleModal 打开）', async () => {
    await renderApp();

    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    fireEvent.click(screen.getByTestId('role-list-role-role-fitness'));
    fireEvent.click(screen.getByTestId('role-more-menu'));
    fireEvent.click(screen.getByTestId('role-menu-add'));

    expect(screen.getByText('添加角色')).toBeInTheDocument();
  });

  it('设置 tab：桌面同款 GlobalSettingsModal 按 initialTab 落点 + 主题/登出行在场', async () => {
    // 浏览器宿主（登出行门控 isTauriHost——与 Sidebar 同语义）
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    await renderApp();

    fireEvent.click(screen.getByTestId('bottom-tab-settings'));
    expect(screen.getByTestId('settings-row-llm')).toBeInTheDocument();
    expect(screen.getByTestId('settings-row-scheduler')).toBeInTheDocument();
    expect(screen.getByTestId('settings-theme-light')).toBeInTheDocument();
    expect(screen.getByTestId('settings-logout')).toBeInTheDocument();

    // 模型服务 → llm 落点；调度时间 → scheduler tab（敲门通知声音所在——
    // 2026-09-26 移除与之落点重复的「通知」行，owner 授权同型改写本守门钉孔）
    fireEvent.click(screen.getByTestId('settings-row-llm'));
    expect(screen.getByTestId('settings-modal')).toHaveAttribute('data-initial-tab', 'llm');
    fireEvent.click(screen.getByTestId('settings-row-scheduler'));
    expect(screen.getByTestId('settings-modal')).toHaveAttribute('data-initial-tab', 'scheduler');
    fireEvent.click(screen.getByTestId('settings-row-data'));
    expect(screen.getByTestId('settings-modal')).toHaveAttribute('data-initial-tab', 'data');

    // 主题一行切换：深色 → documentElement 落 dark（index.html FOUC 同机制）
    fireEvent.click(screen.getByTestId('settings-theme-dark'));
    await waitFor(() => {
      expect(document.documentElement.classList.contains('dark')).toBe(true);
    });

    // 设置面 → 直点底部「角色」tab：回管家上下文 + 列表根（防叠屏回归）
    fireEvent.click(screen.getByTestId('bottom-tab-roles'));
    expect(screen.queryByTestId('settings-row-llm')).not.toBeInTheDocument();
    expect(screen.getByTestId('role-list-butler')).toBeInTheDocument();
    expect(screen.getByTestId('bottom-tab-roles')).toHaveAttribute('aria-current', 'page');
  });

  it('断点收敛：matchMedia 翻桌面宽度时移动专属状态还原子（BREAKPOINT_EDGE 无残留）', async () => {
    // per-test stub（test-setup 全局 matches:false 不动）：可翻的桌面媒体查询
    // （引用持有者承载 listener——直接变量赋值会被 TS 收敛为 null 类型）
    const mediaRef: { isDesktop: boolean; onChange: (() => void) | null } = { isDesktop: false, onChange: null };
    vi.stubGlobal('matchMedia', (query: string) => ({
      get matches() { return mediaRef.isDesktop; },
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: (_: string, fn: () => void) => { mediaRef.onChange = fn; },
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }));
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    await renderApp();

    // 移动态：点设置 tab 进移动设置面
    fireEvent.click(screen.getByTestId('bottom-tab-settings'));
    expect(screen.getByTestId('settings-row-llm')).toBeInTheDocument();

    // 放大窗口：媒体查询翻桌面（jsdom 手动驱动 change 回调）⇒ 设置面还原子
    mediaRef.isDesktop = true;
    mediaRef.onChange?.();
    await waitFor(() => {
      expect(screen.queryByTestId('settings-row-llm')).not.toBeInTheDocument();
    });
    expect(screen.getByTestId('butler-view')).toBeInTheDocument();
  });
});
