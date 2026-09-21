# Epic 17 Context: 自托管部署与运维（Self-Hosted Deployment & Ops）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让用户在自有 VPS/NAS 上以 `docker compose up` 一键部署 EgoSync 云端版并长期运维：TLS 自动签发、服务端密钥安全注入、7×24 常驻工作循环（时区正确、升级重启不重复触发）、备份/恢复闭环、CI 镜像发布。本史诗完成后闭合 FR-44 部署侧验收（Docker 一键含完整引擎 / 单实例单用户 / 单事实源 / 重启自动恢复 / 桌面云端各自独立部署）与 FR-47 全部五条验收。云端版为自托管单用户形态：完整引擎（Rust server + opencode + SQLite）与全部数据运行于用户自有服务器，官方零持有、零中转、零落地用户数据；多租户是显式 Non-Goal，不为多用户预留任何抽象。

## Stories

- Story 17.1: Docker 化部署、密钥注入与 TLS
- Story 17.2: 常驻工作循环硬化（FR-47）
- Story 17.3: 备份恢复、CI/CD 与发布

## Requirements & Constraints

**部署与自愈**
- 部署产物含完整引擎（server binary + opencode + SQLite），无需安装客户端即可经浏览器使用；新用户从零到浏览器可访问的步骤须完整（域名解析、compose 启动、首访 setup）。
- 单实例单用户；同一用户同一时刻仅一个事实源实例，无多实例同步路径。
- 服务器重启后实例自动恢复、数据配置无损；升级是拉新镜像 tag 重启的两步操作。
- 资源预算：$5-10/月（1C1G 级）VPS 承载，常驻内存占用须在文档标注的预算内验证。

**常驻工作循环（FR-47 五条全部）**
- 无任何客户端在线时循环照常执行；晨间简报/周复盘按计划时间生成并落库留存，下次访问可见。
- 停机期间无新建议（诚实代价）；停机跨过计划时刻则该次跳过，不补发不堆积。
- 常驻生成的建议仍走"待确认→用户确认"，不豁免确认纪律；桌面版"应用运行时执行"行为不变。

**密钥与 TLS**
- 全程 TLS（Caddy 自动 HTTPS 或用户自有反代，文档两种路径都给）；HTTP 明文访问被重定向或拒绝。
- LLM Key 仅存于服务端内存与 /data/secrets.json 两处；任何 API 响应不下发给浏览器（响应形状测试断言无 key 字段）。
- 密钥缺失返回指明重录路径的结构化错误，禁止泛型 "secret not found"；密钥与事件 payload 明文永不入日志。

**数据主权**
- 备份双路径：逻辑级导出/导入 + 卷级停机复制；官方服务零参与（无遥测、无云存储）；导出→新实例导入→数据完整可抽查；导入原子性，失败不半写。

## Technical Decisions

**镜像与 compose 拓扑**
- 多阶段 Dockerfile（前端 dist → server 构建 → 运行时层），运行时层含 opencode 二进制 + tzdata + ca-certificates；opencode 版本 pin 且与 engine 同 tag。
- opencode **并入 server 镜像**而非独立容器（保 sidecar 生命周期代码零改动，agent_bridge 仍走 127.0.0.1 同容器）——此裁决覆盖早期提案的三容器表述；compose 实际两服务：`egosync-server` + `caddy`（可选反代，用户已有反代可去除）；WEB 静态由 server 内嵌服务。
- 双 SQLite 库挂命名卷 /data（WAL 单容器单写者）；opencode 路径经 `EGOSYNC_OPENCODE_PATH` 注入（桌面传 resources 路径，同一注入点）。
- Caddyfile：`request_body max_size 50MB`（与 server body 上限对齐）+ `flush_interval -1` 作 SSE 冗余保险（Caddy 对 SSE 本自动刷新）。
- `restart: unless-stopped` + 启动幂等（migrations 幂等前滚）；compose healthcheck 用存活级 healthz，深度级防误杀不用于存活探针。
- TZ env 透传，简报时间 = 容器时区；HTTP/1.1 反代下 SSE 每域名 6 连接上限须写入部署文档提示。

**SecretStore 服务端适配器（实现 Epic 15 冻结的 trait）**
- 双通道：固定名引导 secret 走 env；UUID 型 api_key_ref 走 /data/secrets.json（0600 明文——"加密"措辞已废除，安全边界与 env 同级）。
- 读取顺序**文件优先、env 兜底**（env 仅在文件缺失时生效，防 env 静默遮蔽文件值）；写路径永远落 secrets.json（env 不可写，文件是运行时唯一可写事实源）。
- env 键映射冻结：变量名 = `EGOSYNC_SECRET_{api_key_ref}` **原样区分大小写**拼接（api_key_ref = `llm_{uuid}_api_key` 全小写）。
- secrets.json 不进镜像层、不进 git（.dockerignore 验证）；桌面→云端迁移时 LLM Key 须重录一次（keyring 不导出）。
- 导入完成后对每个 api_key_ref 探测可达性（现状三处各自拼错误文案，收敛为一处）。

**FR-47 硬化（引擎级变更，桌面同步生效）**
- `scheduler_triggers` 持久化小表替换三处内存去重（last_triggered_map / last_briefing_trigger_date / last_review_trigger_week 均为循环局部变量）；bigrock 既有 DB 去重做**替换迁移**（含数据迁移，migration 编号顺延）；不允许多套去重机制并存。
- 表结构：触发键（job 类型 + date+hhmm / week 周期标识）+ 时区维度 + last_triggered_at。
- **时间源三分表**（防"顺手一致化"）：持久化一律 UTC RFC3339（现状禁改 Local）/ 调度判定用容器 Local / 前端渲染用浏览器 TZ；三分边界代码注释与架构文档双重标注。
- 时区变更后首个周期可能跳过或重复一次（键含时区维度的权衡——宁可一次跳过/重复也不静默双发或漏发整天），部署文档明示。
- 桌面"重启窗口内不再重复触发"是**行为变化非回归**，release note 加注，收口时不得误判。

**备份/CI/发布**
- `/api/export`、`/api/import` 为 HTTP 流端点（data_export 能力的云端双落点，command 面将其列 desktop-only）；导出包含密钥引用清单不含密钥值，与桌面导出包同格式跨形态互导；导入后触发密钥可达性探测。
- 卷级备份：compose down → 复制 /data 卷 → compose up；运行中复制的 WAL 一致性风险必须文档明示，不静默允许。
- `.github/workflows/server-docker.yml`：push tag/main 多阶段构建并推送镜像仓库。
- tests/e2e 新增 `web` 模式（wdio 驱动浏览器连本地 server）；桌面模式零改动、双模式共存；导出导入往返用例纳入 web 模式或 server cargo test。
- 深度 healthz：`GET /healthz?deep=1` 返回 DB 连通 + opencode 存活 + migrations 状态（升级前巡检）。
- CI 三链路并行阻塞：桌面 `npm run test:all`（含传输对等套件）+ server cargo test + web e2e 冒烟，任一红即阻塞合并。
- 服务端结构：`server/` 目录（path 依赖 egosync-engine），`src/{main,routes,sse,auth,secret_store,static_files}.rs`；环境变量一律 `EGOSYNC_` 前缀大写（EGOSYNC_TOKEN / EGOSYNC_OPENCODE_PATH / EGOSYNC_SECRET_* / EGOSYNC_DATA_DIR / EGOSYNC_STATIC_DIR）；日志 tracing JSON 到 stdout。

## Cross-Story Dependencies

- 前置：整个 E17 依赖 Epic 15 交付（引擎 crate、server binary、传输对等）；E17 与 E16 可并行。
- 17.1 实现 15.1 冻结的 SecretStore trait 服务端适配器；17.3 复用 15.1 平移的 data_export/data_import 纯逻辑；CI 三链路依赖 15.5 对等测试套件。
- 史内强顺序：17.1 → 17.2 → 17.3，每步以桌面全绿收口。
- 17.3 的 `/api/import` 完成后触发 17.1 的密钥可达性探测；17.1 部署文档"从零到浏览器可访问"指向 16.1 首访 setup 向导（文档耦合，非代码依赖）。
- 17.2 为引擎级变更，桌面版同步获得持久化去重（需 release note 加注），不阻塞 16.x。
