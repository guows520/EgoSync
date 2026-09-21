// Story 16.3：远程桌面 UI 门控测试（spec AC「能力隐藏（选工作目录禁用并
// 说明）」+ 远程态桌面 = 浏览器等价物）。
//
// 覆盖：
// - GlobalSettingsModal：远程桌面下 desktop-only 门控点隐藏（companion
//   侧栏入口 / 数据导出导入区 / 远程模式 tab 在）；浏览器宿主零远程 tab；
// - ChatStream：远程桌面下工作目录选择改禁用+说明（非隐藏——AC 指定
//   禁用并说明）；本地桌面零回归（原入口可见）；
// - ConnectionStatus：远程模式 REMOTE 徽标呈现；本地无徽标零回归。

import { render, screen, fireEvent } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GlobalSettingsModal } from './GlobalSettingsModal';
import { ConnectionStatus } from '../layout/ConnectionStatus';
import { ChatStream } from '../chat/ChatStream';
import {
  __resetTransportForTests,
  getTransport,
  setTransportBoot,
} from '@/transport';
import {
  __resetDesktopModeForTests,
  setDesktopMode,
} from '../../appMode';

const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

vi.mock('../../services/llmConfigService', () => ({
  llmConfigService: {
    list: vi.fn().mockResolvedValue([]),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    setDefault: vi.fn(),
    testConnection: vi.fn(),
    listModels: vi.fn(),
    listModelsByParams: vi.fn(),
  },
}));

vi.mock('../../services/mcpService', () => ({
  mcpService: {
    list: vi.fn().mockResolvedValue([]),
    listForRole: vi.fn(),
    listAvailableForRole: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    test: vi.fn(),
    addToRole: vi.fn(),
    removeFromRole: vi.fn(),
  },
}));

vi.mock('../../services/dataService', () => ({
  dataService: {
    dataExport: vi.fn(),
    dataDestroy: vi.fn(),
    pickImportFile: vi.fn(),
    dataImport: vi.fn(),
  },
}));

vi.mock('../../services/schedulerService', () => ({
  schedulerService: {
    getTimes: vi.fn().mockResolvedValue({ moderate: [], proactive: [] }),
    updateTimes: vi.fn(),
  },
}));

vi.mock('../../services/appService', () => ({
  appService: {
    isFirstLaunch: vi.fn(),
    completeOnboarding: vi.fn(),
    isLlmConfigured: vi.fn(),
    getSetting: vi.fn().mockResolvedValue('false'),
    setSetting: vi.fn(),
  },
}));

// ChatStream 挂载即拉可选 Skill——与 ChatStream.test.tsx 同款 mock。
// 不 mock 时会走全局 invoke 桩，其 null 载荷经 setAvailableSkills 击穿
// ChatInput（曾致一次未处理 TypeError——测试健壮性修复）。
vi.mock('../../services/skillService', () => ({
  skillService: {
    listSelectableForScope: vi.fn().mockResolvedValue([]),
  },
}));

/** ChatStream 最小驱动（工作目录门控仅需挂载输入区——conversation 非必需 prop）。 */
function renderChatStream() {
  return render(<ChatStream role={null} />);
}

describe('远程桌面 UI 门控（Story 16.3）', () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    __resetDesktopModeForTests();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
  });

  function bootRemote() {
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://instance.example.com', remoteToken: 'tk' });
    setDesktopMode('remote');
  }

  describe('GlobalSettingsModal 门控点', () => {
    it('远程桌面：companion 入口隐藏（远端不可达 companion_*——与浏览器同语义）', () => {
      bootRemote();
      render(<GlobalSettingsModal onClose={() => {}} />);
      expect(screen.queryByRole('button', { name: '手机伴侣' })).not.toBeInTheDocument();
      // 远程模式 tab 在（切换回本地的入口）
      expect(screen.getByRole('button', { name: '远程模式' })).toBeInTheDocument();
    });

    it('远程桌面：数据 tab 的导出/导入区隐藏（desktop-only）+ 销毁区保留（web-ok）', () => {
      bootRemote();
      render(<GlobalSettingsModal onClose={() => {}} />);
      fireEventClick('数据与隐私');
      expect(screen.queryByRole('button', { name: /导出存档/ })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /导入存档/ })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /销毁所有数据/ })).toBeInTheDocument();
    });

    it('远程桌面：远程模式 tab 渲染 remote 态分区（连接信息）', () => {
      bootRemote();
      render(<GlobalSettingsModal onClose={() => {}} />);
      fireEventClick('远程模式');
      expect(screen.getByTestId('remote-mode-section')).toHaveAttribute('data-mode', 'remote');
    });

    it('本地桌面：远程模式 tab 渲染 local 态（配置+切换）——既有门控零回归', () => {
      setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
      setDesktopMode('local');
      render(<GlobalSettingsModal onClose={() => {}} />);
      fireEventClick('远程模式');
      expect(screen.getByTestId('remote-mode-section')).toHaveAttribute('data-mode', 'local');
      // 本地桌面：companion 与导出导入照常可见（既有断言的等价覆盖）
      expect(screen.getByRole('button', { name: '手机伴侣' })).toBeInTheDocument();
      fireEventClick('数据与隐私');
      expect(screen.getByRole('button', { name: /导出存档/ })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /导入存档/ })).toBeInTheDocument();
    });

    it('浏览器宿主：远程模式 tab 不渲染（模式切换是桌面壳语义）', () => {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
      __resetTransportForTests();
      render(<GlobalSettingsModal onClose={() => {}} />);
      expect(screen.queryByRole('button', { name: '远程模式' })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: '手机伴侣' })).not.toBeInTheDocument();
    });
  });

  describe('ChatStream 工作目录门控（禁用+说明——AC 指定）', () => {
    it('远程桌面：选择入口改禁用+说明（不隐藏——AC 冻结款）', () => {
      bootRemote();
      renderChatStream();
      // 禁用说明呈现
      const hint = screen.getByTestId('working-directory-remote-hint');
      expect(hint).toHaveTextContent('工作目录选择仅在本地模式可用');
      // 原「选择工作目录」按钮不渲染
      expect(screen.queryByRole('button', { name: '选择工作目录' })).not.toBeInTheDocument();
    });

    it('本地桌面：原选择入口可见（零回归）', () => {
      setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
      setDesktopMode('local');
      renderChatStream();
      expect(screen.queryByTestId('working-directory-remote-hint')).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: '选择工作目录' })).toBeInTheDocument();
    });
  });

  describe('ConnectionStatus REMOTE 徽标', () => {
    it('远程桌面：REMOTE 徽标呈现（AC：REMOTE 标识+三态连接呈现）', () => {
      bootRemote();
      render(<ConnectionStatus />);
      const badge = screen.getByTestId('remote-mode-badge');
      expect(badge).toHaveTextContent('远程');
      const status = screen.getByTestId('connection-status');
      expect(status).toHaveAttribute('data-remote', 'true');
    });

    it('本地桌面：无 REMOTE 徽标（零回归）', () => {
      setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
      setDesktopMode('local');
      render(<ConnectionStatus />);
      expect(screen.queryByTestId('remote-mode-badge')).not.toBeInTheDocument();
      expect(screen.getByTestId('connection-status')).not.toHaveAttribute('data-remote');
    });

    // [评审轮2 U6] useConnectionState 惰性初值钉死：传输订阅即回调会把
    // 初值当帧自愈——隔离订阅（不回调）才能钉住惰性表达式。回归形态
    // = 改回 isTauriHost()?online（16.2 修过的首帧闪错误在线态）。
    it('远程桌面：useConnectionState 惰性初值 connecting（订阅回调前不闪 online）', () => {
      bootRemote();
      const spy = vi
        .spyOn(getTransport(), 'onConnectionStateChange')
        .mockImplementation(() => () => {});
      try {
        render(<ConnectionStatus />);
        expect(screen.getByTestId('connection-status')).toHaveAttribute('data-state', 'connecting');
      } finally {
        spy.mockRestore();
      }
    });

    it('本地桌面：useConnectionState 惰性初值 online（零回归）', () => {
      setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
      setDesktopMode('local');
      const spy = vi
        .spyOn(getTransport(), 'onConnectionStateChange')
        .mockImplementation(() => () => {});
      try {
        render(<ConnectionStatus />);
        expect(screen.getByTestId('connection-status')).toHaveAttribute('data-state', 'online');
      } finally {
        spy.mockRestore();
      }
    });
  });
});

/** 点击设置侧栏按钮（按可访问名）。 */
function fireEventClick(name: string) {
  fireEvent.click(screen.getByRole('button', { name }));
}
