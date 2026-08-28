// 手机伴侣类型（Story 12.2）——镜像 src-tauri models/companion.rs（serde camelCase）

export interface PairedDevice {
  id: string;
  deviceName: string;
  devicePubkey: string;
  pairedAt: string;
  lastSeenAt: string;
}

export interface QrPayload {
  /** V1 中继未部署（Story 12.3），恒为 null */
  relayAddr: string | null;
  desktopStaticPubkey: string;
  relayId: string;
  pairingNonce: string;
}

export interface ConnectedDeviceInfo {
  deviceId: string;
  deviceName: string;
  origin: string;
  since: string;
}

export interface PendingPairing {
  deviceName: string;
  devicePubkey: string;
  createdAt: string;
}

export interface CompanionStatus {
  listening: boolean;
  port: number | null;
  connected: ConnectedDeviceInfo | null;
  pairedDevice: PairedDevice | null;
  pendingPairing: PendingPairing | null;
}
