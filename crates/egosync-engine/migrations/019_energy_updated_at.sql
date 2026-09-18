-- Story 4.8: 为 roles 表添加 energy_updated_at 列，记录能量值最后计算时间
ALTER TABLE roles ADD COLUMN energy_updated_at TEXT;
