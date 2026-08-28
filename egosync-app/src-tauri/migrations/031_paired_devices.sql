-- Story 12.2: 已配对手机设备表（单对单语义：db 层 upsert 前清空旧记录）
CREATE TABLE IF NOT EXISTS paired_devices (
    id TEXT PRIMARY KEY NOT NULL,
    device_name TEXT NOT NULL,
    device_pubkey TEXT NOT NULL UNIQUE,
    paired_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_paired_devices_device_pubkey ON paired_devices(device_pubkey);
