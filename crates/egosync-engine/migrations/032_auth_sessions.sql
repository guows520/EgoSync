-- Story 15.4: 服务端单用户认证会话表（架构决策 #5 会话持久化落点）。
-- login 校验通过后换发随机会话令牌（uuid v4），SHA256 哈希入库；
-- 每请求校验 = 主键索引查找 + 哈希比对（亚毫秒）。
-- 多行 = 多会话（多浏览器/多窗口语义平移）；env/库态凭据切换不失效
-- 已发 Cookie（会话生命周期独立于引导凭据形态）。

CREATE TABLE IF NOT EXISTS auth_sessions (
    token_hash TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_seen_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
