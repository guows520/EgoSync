-- Story 15.4 违约修复：15.4 评审修复提交 3a5c85c 曾对已应用的迁移 032 原地删列
-- （last_seen_at），违反 sqlx「已应用迁移不可变」契约——历史开发库
-- （v32 记录原版 checksum 7cabf4b156f5…）启动即报
-- 「migration 32 was previously applied but has been modified」，应用完全不可用。
-- 修复方式：032 已字节级还原为 3d0f3b6 原版（恢复 checksum 匹配），本迁移以
-- 030_llm_provider_extended.sql 的重建范式（CREATE new + INSERT SELECT + DROP +
-- RENAME）合规重做 YAGNI 删列：显式列清单 SELECT 对「有列历史库」与「无列窗口库」
-- 两种血统都成立，且不依赖 SQLite >= 3.35 的 DROP COLUMN；事务内原子完成、行全量保留。
-- 终态恰两列 token_hash + created_at（删活跃度列理由见 15.4 评审修复 #7：
-- 校验读路径保持纯读零写放大，created_at 即会话建立时间）。
--
-- 窗口态库手工修复指引（3a5c85c 与本修复之间新建的库——v32 记录 checksum 为
-- cc552550a4fd4b64a8e8255559d47945c29ab83fc1361406745888db25faf58bfd25e40f6a6970311a19b40b7732627f、
-- auth_sessions 无 last_seen_at 列；sqlx 在 v32 校验处即报错，本迁移不会自动救）：
--   方案 A（保留数据）：对 egosync.db 执行
--     UPDATE _sqlx_migrations SET checksum = X'7CABF4B156F54E053CA7E43F87438DC57F55259854A12FE1A17173AB26A96FC605BE6F0F4C79875133EEEDD90E1CB15A' WHERE version = 32;
--     （X'...' 为 032 原版完整 sha384 hex。）重启后 v32 校验通过、本迁移正常应用；
--     重建按显式列清单搬行，无列血统同样成立，终态 schema 与数据均不受影响。
--   方案 B（数据可弃）：删除 egosync.db 后重新初始化。

CREATE TABLE auth_sessions_new (
    token_hash TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO auth_sessions_new (token_hash, created_at)
SELECT token_hash, created_at
FROM auth_sessions;

DROP TABLE auth_sessions;

ALTER TABLE auth_sessions_new RENAME TO auth_sessions;
