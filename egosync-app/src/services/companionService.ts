// 手机伴侣 service（Story 12.2）——封装全部 invoke，组件不直接调用 invoke。
// QR 图渲染封闭在本 service 内（npm qrcode，后端只产出 payload）。

import { invoke } from '@tauri-apps/api/core';
import QRCode from 'qrcode';
import type { CompanionStatus, PairedDevice, QrPayload } from '../types/companion';

export const companionService = {
  generateQr: () => invoke<QrPayload>('pairing_generate_qr'),

  confirmPairing: () => invoke<PairedDevice>('pairing_confirm'),

  listPairedDevices: () => invoke<PairedDevice[]>('paired_device_list'),

  removePairedDevice: (deviceId: string) =>
    invoke<void>('paired_device_remove', { deviceId }),

  getStatus: () => invoke<CompanionStatus>('companion_get_status'),

  /** 读取中继服务器地址（null = 未配置，中继承载禁用）。 */
  getRelayAddr: () => invoke<string | null>('companion_get_relay_addr'),

  /** 配置中继服务器地址（如 ws://relay.example.com:7333）；传 null 清除配置。 */
  setRelayAddr: (relayAddr: string | null) =>
    invoke<void>('companion_set_relay_addr', { relayAddr }),

  /** 将 payload JSON 渲染为 SVG 字符串（调用方以 dangerouslySetInnerHTML 挂载）。 */
  renderQrSvg: (payloadText: string) =>
    QRCode.toString(payloadText, { type: 'svg', margin: 1, width: 220 }),
};
