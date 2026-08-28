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

  /** 将 payload JSON 渲染为 SVG 字符串（调用方以 dangerouslySetInnerHTML 挂载）。 */
  renderQrSvg: (payloadText: string) =>
    QRCode.toString(payloadText, { type: 'svg', margin: 1, width: 220 }),
};
