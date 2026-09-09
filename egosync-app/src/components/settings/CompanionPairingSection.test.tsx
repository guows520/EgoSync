// T-S6（SPEC qr-and-unbind-semantics §1/§2）：桌面二维码生命周期测试。
// WHY Finding 5：`companion:paired` 事件只刷新设备列表不清码——已消费的码
// 仍显示为可扫，与「二维码单次有效」文案自相矛盾；且有效期内无重新生成
// 入口，用户只能干等 300s 倒计时。

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { CompanionPairingSection } from './CompanionPairingSection';
import { companionService } from '../../services/companionService';
import type { CompanionStatus } from '../../types/companion';

vi.mock('../../services/companionService', () => ({
  companionService: {
    listPairedDevices: vi.fn(),
    getStatus: vi.fn(),
    generateQr: vi.fn(),
    renderQrSvg: vi.fn(),
    getRelayAddr: vi.fn(),
    confirmPairing: vi.fn(),
    removePairingDevice: vi.fn(),
  },
}));

// 事件钩子替身：记录 handler 供测试模拟后端事件（companion:paired 等）
const eventHandlers = new Map<string, (payload: unknown) => void>();
vi.mock('../../hooks/useTauriEvent', () => ({
  useTauriEvent: (eventName: string, handler: (payload: unknown) => void) => {
    eventHandlers.set(eventName, handler);
  },
}));

const qrPayloadFixture = {
  relayAddr: null,
  desktopStaticPubkey: 'a'.repeat(64),
  relayId: 'b'.repeat(16),
  pairingNonce: 'nonce-1',
};

const mockRefresh = () => {
  vi.mocked(companionService.listPairedDevices).mockResolvedValue([]);
  vi.mocked(companionService.getStatus).mockResolvedValue({
    listening: true,
    port: 47000,
  } as unknown as CompanionStatus);
  vi.mocked(companionService.getRelayAddr).mockResolvedValue(null);
};

const mockGenerate = (nonce: string) => {
  vi.mocked(companionService.generateQr).mockResolvedValue({ ...qrPayloadFixture, pairingNonce: nonce });
  vi.mocked(companionService.renderQrSvg).mockResolvedValue('<svg data-testid="qr"></svg>');
};

/** 生成并等待二维码显示（生成是异步两步：generateQr + renderQrSvg）。 */
const generateAndShow = async () => {
  fireEvent.click(screen.getByRole('button', { name: '生成配对二维码' }));
  await waitFor(() => expect(screen.getByRole('img', { name: '手机伴侣配对二维码' })).toBeInTheDocument());
};

describe('CompanionPairingSection 二维码生命周期（T-S6）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventHandlers.clear();
    mockRefresh();
    mockGenerate('nonce-1');
  });

  it('配对成功事件立即清码并标记「已被使用」+ 重新生成为主操作', async () => {
    // WHY（SPEC §2）：companion:paired（首配/换绑确认）即消费 nonce——
    // 继续显示旧码会让旁人重扫得到「配对请求已提交」假象（实为结构化拒绝）。
    render(<CompanionPairingSection />);
    await generateAndShow();

    expect(eventHandlers.get('companion:paired')).toBeDefined();
    // act 包裹：事件回调在 React 渲染周期外触发，需显式刷新（后端事件同构）
    act(() => {
      (eventHandlers.get('companion:paired') as (p: unknown) => void)({ deviceId: 'd1' });
    });

    // 清码：二维码图消失，代之以「已被使用」状态与主操作按钮
    expect(screen.queryByRole('img', { name: '手机伴侣配对二维码' })).not.toBeInTheDocument();
    expect(screen.getByText(/二维码已被使用（配对成功）/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重新生成二维码' })).toBeInTheDocument();
    // 不再显示可扫文案（旧码不可扫）
    expect(screen.queryByText(/请用手机伴侣 App 扫描此二维码/)).not.toBeInTheDocument();

    // 已消费态再点重新生成 → 新码出现（状态复位）
    fireEvent.click(screen.getByRole('button', { name: '重新生成二维码' }));
    await waitFor(() => expect(screen.getByRole('img', { name: '手机伴侣配对二维码' })).toBeInTheDocument());
    expect(screen.queryByText(/二维码已被使用/)).not.toBeInTheDocument();
  });

  it('显示态提供重新生成入口且需二次确认（旧码立即失效）', async () => {
    // WHY（SPEC §1）：有效期内换码是常见诉求（码泄露/扫错手机）；直接换码
    // 会打断进行中的扫码——破坏性操作必须二次确认。
    render(<CompanionPairingSection />);
    await generateAndShow();

    fireEvent.click(screen.getByRole('button', { name: '重新生成二维码' }));
    expect(screen.getByText('重新生成二维码？')).toBeInTheDocument();
    expect(screen.getByText(/重新生成后当前二维码立即失效，正在进行的扫码将无法完成配对/)).toBeInTheDocument();

    // 确认前不触发生成（误触保护）
    expect(companionService.generateQr).toHaveBeenCalledTimes(1); // 仅首次显示
    fireEvent.click(screen.getByRole('button', { name: '取消' }));
    expect(companionService.generateQr).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('img', { name: '手机伴侣配对二维码' })).toBeInTheDocument();

    // 确认后调用既有生成路径换新码
    mockGenerate('nonce-2');
    fireEvent.click(screen.getByRole('button', { name: '重新生成二维码' }));
    fireEvent.click(screen.getByRole('button', { name: '重新生成' }));
    await waitFor(() =>
      expect(companionService.generateQr).toHaveBeenCalledTimes(2),
    );
  });
});
