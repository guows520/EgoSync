import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ButlerView } from './ButlerView';

// Story 16.4 布局修复（2026-09-26 人类指令）小屏钉孔：
// ① tab 打开时对话区 max-md:hidden、工作区 max-md:flex-1 全屏——点
//    「仪表盘/任务」只显示对应页面（替代原 58/42 堆叠）；
// ② 移动视口下来源消息跳转先关 tab 回对话（对话区 hidden 时跳转不可见）；
//    桌面视口不关（保持既有双栏行为零变化）。
// 范式同 ButlerView.test.tsx（ChatStream/ButlerWorkspacePanel mock，
// 不触发 service 层）；matchMedia 打桩控制 isMobileViewport 判定。
vi.mock('../chat/ChatStream', () => ({
  ChatStream: ({ sourceNavigationTarget }: any) => (
    <div data-testid="butler-source-target">{sourceNavigationTarget?.messageId ?? 'none'}</div>
  ),
}));

vi.mock('./ButlerWorkspacePanel', () => ({
  ButlerWorkspacePanel: ({ currentTab, onSourceMessageClick }: any) => (
    <div>
      <div data-testid="butler-current-tab">{currentTab}</div>
      <button
        type="button"
        onClick={() => onSourceMessageClick?.({ conversationId: 'conv-butler-source', messageId: 'msg-butler-source', roleId: null })}
      >
        点击管家来源记录
      </button>
    </div>
  ),
}));

const renderButler = () =>
  render(
    <ButlerView
      roles={[]}
      archivedRoles={[]}
      onRestoreRole={vi.fn()}
      onViewChange={vi.fn()}
      onUpdateRole={vi.fn()}
      onRoleSourceNavigation={vi.fn()}
      sourceNavigationTarget={null}
      onSourceNavigationHandled={vi.fn()}
      onOpenTask={vi.fn()}
      onTasksApiReady={vi.fn()}
      knockNotifications={[]}
      onDismissKnock={vi.fn()}
      chatRefreshTrigger={0}
    />,
  );

/** 主区行容器（flex-1 flex flex-col md:flex-row—— ButlerView 头部同样含
 *  flex-col md:flex-row，故以 flex-1 前缀消歧） */
const mainRow = (container: HTMLElement) =>
  container.querySelector('.flex-1.flex-col.md\\:flex-row') as HTMLElement;

/** 打桩 matchMedia：仅 max-width: 767px 查询返回 matches=true（jsdom 恒 false） */
const stubMobileViewport = () => {
  vi.stubGlobal(
    'matchMedia',
    (query: string) => ({
      matches: query.includes('max-width: 767px'),
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }),
  );
};

describe('ButlerView 小屏 tab 全屏化（Story 16.4 布局修复）', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('tab 打开时对话区 max-md:hidden、工作区 max-md:flex-1 全屏（桌面 65/35 钉孔不动）', () => {
    const { container } = renderButler();

    fireEvent.click(screen.getByRole('button', { name: /仪表盘/ }));

    const chatPane = mainRow(container).children[0];
    expect(chatPane).toHaveClass('max-md:hidden');
    expect(chatPane).toHaveClass('md:w-[65%]');
    const workspacePane = mainRow(container).children[1];
    expect(workspacePane).toHaveClass('max-md:flex-1');
    expect(workspacePane).toHaveClass('md:w-[35%]');
    expect(screen.getByTestId('butler-current-tab')).toHaveTextContent('dashboard');
  });

  it('移动视口下来源跳转先关 tab 回对话，再传定位目标（对话区 hidden 时跳转可见）', () => {
    stubMobileViewport();
    const { container } = renderButler();

    fireEvent.click(screen.getByRole('button', { name: /仪表盘/ }));
    fireEvent.click(screen.getByRole('button', { name: '点击管家来源记录' }));

    // 工作区已卸载（tab 关闭）——对话区回到全屏，定位目标随之可见
    expect(screen.queryByTestId('butler-current-tab')).not.toBeInTheDocument();
    expect(mainRow(container).children).toHaveLength(1);
    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('msg-butler-source');
  });

  it('桌面视口下来源跳转保持工作区打开（既有双栏行为零变化）', () => {
    // 不打桩：jsdom matchMedia 恒 matches=false ⇒ isMobileViewport()=false
    renderButler();

    fireEvent.click(screen.getByRole('button', { name: /仪表盘/ }));
    fireEvent.click(screen.getByRole('button', { name: '点击管家来源记录' }));

    expect(screen.getByTestId('butler-current-tab')).toHaveTextContent('dashboard');
    expect(screen.getByTestId('butler-source-target')).toHaveTextContent('msg-butler-source');
  });

  /// 移动端去重（2026-09-26 人类指令，spec-web-mobile-tab-dedup）：面板 tab 条
  /// 小屏退场后，关闭 X 上移头部第一行右端——tab 打开时渲染；点击经 toggle
  /// 语义关闭工作区回对话（与再点当前 tab 等价）；md:hidden 桌面零变化。
  it('tab 打开时头部渲染关闭钮，点击关闭工作区回对话', () => {
    renderButler();

    fireEvent.click(screen.getByRole('button', { name: /仪表盘/ }));
    const closeButton = screen.getByTestId('butler-close-tab');
    expect(closeButton).toHaveAttribute('aria-label', '关闭');
    expect(closeButton).toHaveClass('md:hidden');

    fireEvent.click(closeButton);
    expect(screen.queryByTestId('butler-current-tab')).not.toBeInTheDocument();
  });

  it('tab 未打开时头部不渲染关闭钮', () => {
    renderButler();
    expect(screen.queryByTestId('butler-close-tab')).not.toBeInTheDocument();
  });

  /// 盲猎补漏（位置钉孔，与角色侧对等）：X 必须在头部第一行（标题行）内、
  /// 且不在 tab 行——「不放 tab 行」是 spec 决策（tab 行 4 钮 + X 在 375px
  /// 溢出约 10px），无本钉孔时误挪进 tab 行全绿漏网。
  it('关闭钮位于头部第一行标题行内，不在 tab 行', () => {
    renderButler();

    fireEvent.click(screen.getByRole('button', { name: /仪表盘/ }));
    const closeButton = screen.getByTestId('butler-close-tab');
    const titleRow = (screen.getByText('数字分身管家').closest('div') as HTMLElement);
    expect(titleRow.contains(closeButton)).toBe(true);
    const tabRow = (screen.getByRole('button', { name: /仪表盘/ }).parentElement as HTMLElement);
    expect(tabRow.contains(closeButton)).toBe(false);
  });
});
