# Epic 17 Context: 自托管部署与运维（Self-Hosted Deployment & Ops）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让用户在自有 VPS/NAS 上以 `docker compose up` 一键部署 EgoSync 云端版并长期运维：自动 TLS、服务端密钥安全注入（浏览器永远见不到密钥）、7×24 常驻工作循环（时区正确、升级/重启不重复触发）、备份/恢复闭环与 CI 镜像发布。本 Epic 闭合 FR-44 的部署侧验收（Docker 一键含完整引擎、单实例单用户、单事实源、重启自动恢复）与 FR-47 全部五条验收。云端版为自托管单用户形态，官方零持有零落地，多租户是显式 Non-Goal；前置是 Epic 15 已交付的无头引擎、server binary 与双传输对等基建。

## Stories

- Story 17.1: Docker 化部署、密钥注入与 TLS
- Story 17.2: 常驻工作循环硬化（FR-47）
- Story 17.3: 备份恢复、CI/CD 与发布

## Requirements & Constraints

**部署与自愈**

- 一条 `docker compose up -d` 完成部署，产物含完整引擎（server binary + opencode + SQLite）；从零到浏览器可访问的步骤完整成文（域名解析、compose 启动、首访 setup）。
- 单实例单用户；同一用户同一时刻仅一个事实源实例，不存在任何多实例同步路径。
- 服务器重启后实例自愈、数据与配置无损；升级 = compose pull → up -d 两步；资源预算 $5-10/月（1C1G 级）VPS 承载。
- 全程 TLS（自动 HTTPS 或用户自有反代，两种路径都给文档）；HTTP 明文访问被重定向或拒绝。

**常驻工作循环（FR-47 五条全部）**

- 无任何客户端在线时循环照常执行；晨间简报/周复盘按计划时间生成并落库留存，下次访问可见。
- 停机期间无新建议（诚实代价）；停机跨过计划时刻则该次跳过，不补发不堆积。
- 常驻生成的建议仍走「待确认→用户确认」，确认纪律不因常驻豁免；桌面版「应用运行时执行」行为不变。

**密钥边界**

- LLM Key 仅存在于服务端内存与 /data/secrets.json 两处；任何 API 响应不下发浏览器（响应形状测试断言无 key 字段）；密钥与事件 payload 明文永不入日志。
- 密钥缺失返回指明重录路径的结构化错误，禁止泛型 "secret not found"。

**数据主权**

- 备份双路径：逻辑级导出/导入 + 卷级停机复制；官方服务零参与（无遥测、无云存储）。
- 导出→新实例导入→数据完整；导入原子，失败不半写。

## Technical Decisions

**镜像与 compose 拓扑**

- 多阶段 Dockerfile（前端 dist → server 构建 → 运行时层）；运行时层含 opencode 二进制 + tzdata + ca-certificates；opencode 版本 pin 且与 engine 同 tag。
- opencode 并入 server 镜像而非独立容器（保 sidecar 生命周期代码零改动，agent_bridge 仍走 127.0.0.1 同容器网络）；compose 实际两服务：`egosync-server` + `caddy`（可选反代，已有反代用户可去除）；WEB 静态由 server 内嵌服务。
- 双 SQLite 库挂命名卷 /data（WAL，单容器单写者）；TZ env 透传；opencode 路径经 `EGOSYNC_OPENCODE_PATH` 注入（与桌面同一注入点）。
- Caddyfile：`request_body max_size 50MB`（与 server body 上限对齐）+ `flush_interval -1` 作 SSE 冗余保险；HTTP/1.1 反代下 SSE 每域名 6 连接上限写入部署文档。
- `restart: unless-stopped` + 启动幂等（migrations 自动前滚）；healthz 两级——存活级供 compose 探针，深度级（`?deep=1`：DB 连通 + opencode 存活 + migrations 状态）供升级前巡检，不得用作存活探针。
- WEB 静态零第三方域：字体经 @fontsource-variable 自托管随 dist 分发，CSP 不含任何 http(s) 外链源（契约测试守门）。

**SecretStore 服务端适配器（实现引擎侧冻结的 trait）**

- 双通道：固定名引导 secret 走 env（如 EGOSYNC_TOKEN）；UUID 型 api_key_ref 走 /data/secrets.json（0600 明文——「加密」措辞已废除，安全边界与 env 同级）。
- 读取顺序文件优先、env 兜底（防 env 静默遮蔽文件值）；写路径永远落 secrets.json（env 不可写，文件是运行时唯一可写事实源）。
- env 键映射冻结：变量名 = `EGOSYNC_SECRET_{api_key_ref}` 原样区分大小写拼接（api_key_ref = `llm_{uuid}_api_key` 全小写，「前缀大写」等转换读法非法）。
- secrets.json 不进镜像层、不进 git；桌面→云端迁移 LLM Key 须重录一次（keyring 不随导出迁移）；导入完成后对每个 api_key_ref 探测可达性。

**调度硬化（引擎级变更，桌面同步生效）**

- `scheduler_triggers` 持久化小表替换全部内存去重（规划口径三处，实现期实测五处——另两处 bigrock 规划提醒/周五检查同款失忆）；bigrock 现状 DB 去重做替换迁移含数据迁移（migration 034，早期编号 032 已过时）；`q2_reminders` 现状已 DB 持久化，保持不动。表结构：一行-per-scope，UNIQUE(job, scope, tz_offset)，行存最新 cycle + last_triggered_at + trigger_count；tz_offset 取触发时刻 Local 偏移，DST 跳变视同一次 TZ 变更。
- 时间源三分表（防「顺手一致化」）：持久化一律 UTC RFC3339（现状禁改 Local）/ 调度判定用容器 Local（TZ env 生效）/ 前端渲染用浏览器 TZ；三分边界代码注释与架构文档双重标注。
- TZ 变更后首个周期可能跳过或重复一次触发（键含时区维度的权衡：宁可一次跳过/重复，不静默双发或漏发整天），部署文档明示。
- 桌面「重启窗口内不再重复触发」是行为变化非回归，release note 加注，验收不得误判。

**备份、CI 与发布**

- `/api/export`、`/api/import` 为 HTTP 流端点，复用 data_export/data_import 纯逻辑（command 注册表中列 desktop-only，同一能力双落点）；导出包含密钥引用清单不含密钥值，与桌面导出包同格式、跨形态可互导；导入完成后触发密钥可达性探测。
- 卷级备份：compose down → 复制 /data 卷 → compose up；运行中复制的 WAL 一致性风险必须文档明示。
- `.github/workflows/server-docker.yml`：push tag/main 触发多阶段镜像构建并推送镜像仓库，镜像 tag 与 engine 版本同步。
- tests/e2e 新增 `web` 模式（wdio 驱动浏览器连本地 server：启动 → 认证 → 核心流程断言 + 导出导入往返用例）；桌面模式零改动、双模式共存。
- CI 三链路并行阻塞：桌面 `npm run test:all`（含传输对等套件）+ server cargo test + web e2e 冒烟，任一红即阻塞合并。
- server 代码位于 `server/`（path 依赖 egosync-engine），布局 `src/{main,routes,sse,auth,secret_store,static_files}.rs`；环境变量一律 `EGOSYNC_` 前缀；日志 tracing JSON 到 stdout。

## Cross-Story Dependencies

- 整个 Epic 依赖 Epic 15 交付（引擎 crate、server binary、双传输对等套件）；与 Epic 16 可并行。
- 史内强顺序 17.1 → 17.2 → 17.3，每步以桌面测试全绿收口。
- 17.1 实现引擎侧冻结的 SecretStore trait 服务端适配器；17.3 的 /api/import 完成后触发 17.1 落地的密钥可达性探测。
- 17.1 部署文档「首访 setup」指向 16.1 的首访向导（文档耦合，非代码依赖）。
- 17.2 是引擎级变更，桌面版同步获得持久化去重（不阻塞 16.x）。
