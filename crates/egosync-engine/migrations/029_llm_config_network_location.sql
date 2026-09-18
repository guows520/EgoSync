-- Story 8.7: 为 llm_configs 添加网络位置字段
-- internal = 内网直连（绕过代理），external = 外网（走系统代理）
-- 历史记录默认 external，保持升级后代理行为不变
ALTER TABLE llm_configs ADD COLUMN network_location TEXT NOT NULL DEFAULT 'external'
    CHECK(network_location IN ('internal', 'external'));
