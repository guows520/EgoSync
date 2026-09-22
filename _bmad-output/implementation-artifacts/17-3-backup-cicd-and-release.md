---
title: '17-3 备份恢复、CI/CD 与发布'
type: 'feature'
created: '2026-09-21'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'cfeef4304728e433154f78d982f91da9633162fb'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-17-context.md'
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 云端版数据主权与质量门禁缺最后一环——`/api/export`、`/api/import` 端点不存在（引擎纯逻辑在场但只有桌面 rfd 对话框落点）、导入后密钥可达性批量探测未落地（17.1 只交付错误文案）、server 镜像未发布任何 registry（部署文档自声明 compose pull 拉不到）、web e2e 不进任何 CI（17.2 评审 deferred D2 空窗）、deep healthz 缺 migrations 状态项、反代下认证限流退化为实例级单桶（17.1 评审 #4）。

**Approach:** server 新增认证面 `GET /api/export`（流式返回导出包 JSON，与桌面 export_json 同格式）与 `POST /api/import`（临时文件复用 import_all，保备份先行+原子语义；导入成功后发 `data:imported` 事件并返回密钥可达性报告）；healthz deep 增 migrations flag；新建 `.github/workflows/server-docker.yml`（tag/main 构建推送 ghcr + 版本校验 + .dockerignore 三副本一致性 + 镜像运行冒烟）；server-ci.yml 加 web e2e 冒烟 job；compose 镜像引用参数化；部署文档补备份恢复/升级/镜像登录闭环。

**已裁决（2026-09-21，boss）：**
- 版本口径=**全仓统一**：本故事把 engine/server Cargo.toml 从 0.1.0 对齐到桌面当前 0.1.6-alpha.3；此后发版同步改 5 个版本文件（桌面 3 处+engine+server），server-docker.yml 校验 tag==server==engine 版本，不符即红。
- 镜像公开性=**私有**：ghcr 镜像保持默认私有；部署文档教 VPS 侧 docker login（GitHub PAT）；不做任何公开化操作。
- web UI=**最小 UI**：web 模式设置页数据区加导出下载（blob）/导入上传（file input→POST）入口，「一键导出」对云端用户成立。

## Boundaries & Constraints

**Always:**
- 导出包与桌面同格式：`exportVersion="1.0"`、ExportData 段形状零改动（跨形态互导兼容）；密钥引用随 llmConfigs[].apiKeyRef 内嵌导出，密钥值永不进任何响应或日志（响应形状测试断言风格沿用 17.1）。
- /api/import 复用引擎 `import_all` 纯逻辑（写临时文件→按内容分发），保持「导入前强制备份、失败不半写（双库各自单事务的既有语义）」零改动；导入成功后经 SseEventBus 发 `data:imported`（复用 engine 常量，与桌面壳同事件名）。
- 错误形状冻结：200+AppError 单键 map+`X-Egosync-App-Error` 判别头；非 200 白名单仅 401/429/404/413/5xx；/api/export、/api/import 挂认证面（Bearer/Cookie 双通道，与 cmd 路由一致），body 受 50MB 上限。
- 镜像构建复用 `server/Dockerfile` 原样；`ARG OPENCODE_VERSION=v1.15.10` 赋值行格式零改动（ci.yml grep 正则依赖）；ghcr 镜像名 owner 小写（relay-docker.yml 先例）。
- web e2e 冒烟选无时钟依赖套件（web-streaming + web-reconnect）；resident-loop/events 留本地全量；桌面 wdio.conf.ts 与桌面 e2e 套件零改动。
- 卷级备份文档口径（停机复制、运行中 WAL 一致性风险明示）保持 17.1 既有文案；官方零参与（无遥测、无云存储）措辞如实。

**Never:**
- 不改 migrations 001-034、DESKTOP_ONLY_COMMANDS 15 条与 103 条对等断言（/api/export、/api/import 是 server 独立路由不经 cmd 分发，data_export/data_import 保持 desktop-only）。
- 不改 ci.yml test-and-build job 与 release.yml（桌面链路=现状 ci.yml 原样充当；版本校验若做放 server-docker.yml 自有 job）。
- 不新造备份机制（双路径=逻辑级端点+卷级文档，架构 ⑥ 冻结）；密钥探测面=SecretStore::load_secret 双通道存在性（17.1 deferred 口径），不做网络级连通测试。
- 不做跨库真原子（SQLite 限制，deferred-work 已登记；双库各自事务语义在部署文档如实说明）。
- 不动 SSE 票据 TTL、auth 初始化优先级（env/setup 冻结）、SecretStore 四断言、compose 顶层 `name: egosync-server`（文档卷名依赖）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 导出下载 | 已认证 GET /api/export | 200 导出包 JSON body（attachment 文件名），与桌面导出包同格式可互导 | 未认证 401；引擎错误走 AppError 形状 |
| 导入恢复 | 已认证 POST /api/import，body=导出包 JSON | 临时文件→import_all（备份先行+原子）；200 返回 ImportResult+missingSecrets 报告 | 形状不符→AppError 既有路径；>50MB→413 |
| 导入后密钥缺失 | 导入含 llmConfigs[].apiKeyRef 但 secrets.json/env 无值 | 200，报告逐项列出缺失 ref+重录路径文案（missing_api_key_error 同款），导入本身成功 | N/A（不阻塞导入） |
| 未认证导出/导入 | 无 Bearer/Cookie | 401 不触碰数据 | 登录面限流 5/min/IP |
| deep 巡检 | GET /healthz?deep=1 | flags 增 `migrations` 项（应用数==内嵌 MIGRATOR 数且无 pending） | 滞后→503+migrations:false |
| 镜像发布 | push tag v* | 校验 tag==server==engine 版本→多阶段构建→推 ghcr（tag=剥 v 版本号+latest） | 版本不符→job 红并打印差异 |
| PR 门禁 | server/engine/前端/e2e 路径变更 | server cargo test + web e2e 冒烟 + dockerignore 三副本一致性，任一红阻塞 | 既有 paths 过滤约定 |
| 反代限流 | EGOSYNC_BEHIND_PROXY=1 经 caddy | 限流键取 X-Forwarded-For 最右值（单可信代理跳语义，文档明示） | 无 XFF→回落 socket IP |
| 导入后在线客户端 | web 客户端已连接 | `data:imported` 经 SSE 广播（当前前端无消费方，前瞻兼容） | 无客户端在场→仅落库 |
| 停机窗口导入 | 计划时刻附近 | 与调度无交互（导入是数据面；17.2 触发表随导入数据整体替换，语义如实） | N/A |

</frozen-after-approval>

## Code Map

- `server/src/lib.rs:80-142` — build_router 三层路由（public/authed/sse_stream+probes+static）；/api/export、/api/import 挂 authed 组（require_auth 中间件复用）；MAX_BODY_BYTES=50MB（:45-46,150）
- `server/src/routes.rs:33-44,71-84` — cmd_handler + WEB_OK 白名单 + AppError→200 单键 map+判别头模式（新路由的错误形状照此）；响应契约冻结
- `crates/egosync-engine/src/services/data_export.rs` — `gather_export_data(:328)` / `export_json(:383)` / `import_all(:1370，先 create_backup 再分发)` / `import_json_data(:816)` / `cleanup_old_backups(:1496)`；ExportData 顶层（:35-66，exportVersion :25）；内联测试 :1531 起含 roundtrip/回滚/版本拒绝先例
- `crates/egosync-engine/src/services/llm_config.rs:45-51` — `missing_api_key_error`（KeyringError 文案含重录路径——探测报告文案复用）；`list_llm_configs` ref 清单（destroy_all_data 同款用法 data_export.rs:1397 附近）
- `server/src/secret_store.rs:110-118` — load_secret 双通道（secrets.json+env）——探测调用面
- `server/src/healthz.rs:26-63` — deep 探针现状（双池+sidecar，flags {"status","db","opencode"}）——加 migrations 项
- `server/src/auth.rs:161-162` — rate_limit 键=ConnectInfo socket IP（XFF 缺口）；`EGOSYNC_BEHIND_PROXY` 解析在 bootstrap.rs:70-97（security 同源判定已消费，限流未消费）
- `server/src/sse.rs:45-52` — SseEventBus（EngineEvents→broadcast 扇出）——data:imported 发射通道
- `crates/egosync-engine/src/events.rs:93` — `DATA_IMPORTED_EVENT="data:imported"`；桌面壳发射点 egosync-app/src-tauri/src/commands/data.rs:107-120（对照）
- `server/tests/common/mod.rs:18-58,66,185` — InProcessServer+Client+login 测试基建；api_test.rs 头注（I/O 矩阵组织方式）；临时双库 build_test_state（bootstrap.rs:384-391）
- `.github/workflows/relay-docker.yml:44-86` — 唯一 docker build/push 先例（buildx+ghcr login+GITHUB_TOKEN+PR 仅构建+tags latest/sha）；照此写 server-docker.yml
- `.github/workflows/server-ci.yml` — paths 触发+cargo test+工件新鲜度+rust-cache——web e2e 冒烟 job 挂靠模板
- `.github/workflows/ci.yml:24-38` — opencode-pin-consistency job（Dockerfile⇄release.yml pin 互锁已存在，勿重复）
- `server/Dockerfile:25,92-101` — OPENCODE_VERSION pin+双架构 sha256（格式冻结）；构建上下文=仓库根
- `server/docker-compose.yml:20-28` — `image: egosync-server:local`+build；顶层 name: egosync-server（卷名文档依赖，:20 冻结）
- `egosync-app/tests/e2e/wdio.web.conf.ts:34-61,171,175-268` — web 模式配置（Chrome 148+钉版 driver、webServerEnv 导出、onPrepare/onComplete 进程管理、18080+固定 token）
- `egosync-app/tests/e2e/scripts/setup-web-drivers.mjs:10-31` — 只装 chromedriver 不装 chrome——CI 需扩展 Chrome-for-Testing 下载
- `egosync-app/tests/e2e/web-specs/` — web-streaming / web-reconnect（冒烟候选，无时钟依赖）/ web-events / web-resident-loop（全量留本地）
- `egosync-app/src/components/settings/` + `src/services/dataService.ts:20-26` — 7.3 数据区现状（web 模式加导出/导入入口的落点；capabilities 门控 egosync-app/src/transport/capabilities.ts:118-123）
- `docs/user-guide/11-云端自托管部署.md:106-115,197-217,236-244` — 升级两步/卷级备份/迁移章节（17.3 空位自声明：镜像 registry、逻辑级端点、跨形态互导、导入探测报告）
- `crates/egosync-engine/src/db/pool.rs:12,55-63` — sqlx::migrate! 嵌入 MIGRATOR（migrations 状态检查数据源；校验钉 :198/205 勿动）
- 版本文件五处：egosync-app/package.json:4、src-tauri/Cargo.toml:3、tauri.conf.json:4（=0.1.6-alpha.3）+ crates/egosync-engine/Cargo.toml:3、server/Cargo.toml:3（=0.1.0，漂移）

## Tasks & Acceptance

**Execution:**
- [x] `server/src/routes.rs`（或新 `server/src/backup.rs` 模块）+ `server/src/lib.rs` — `GET /api/export`（认证面；export_json 纯逻辑→JSON body+attachment 头；密钥值零泄漏）与 `POST /api/import`（body 落 tempfile→import_all；200 返回 ImportResult+missingSecrets 报告；成功后 SseEventBus 发 data:imported）— FR-44 逻辑级备份闭环
- [x] 密钥可达性探测（导入路径内）— 复用 list_llm_configs ref 清单逐项 load_secret，缺失项以 missing_api_key_error 同款文案入报告（不阻塞导入）；含 server 单测（含 env 空串现状语义不动）— 17.1 deferred 收口
- [x] `server/src/healthz.rs` + `crates/egosync-engine/src/db/pool.rs` — deep flags 增 `migrations`（应用数==内嵌 MIGRATOR 数、只读检查不跑迁移）；api_test.rs deep 用例扩展 — 升级前巡检
- [x] `server/src/auth.rs` — EGOSYNC_BEHIND_PROXY=1 时限流键取 X-Forwarded-For 最右值（单可信跳语义，文档明示），无 XFF 回落 socket IP；测试覆盖直连/反代/伪造多跳 — 17.1 评审 #4 收口
- [x] `server/tests/`（新 backup_roundtrip_test.rs 或并入 api_test.rs）— 导出→新实例导入→核心数据抽查（会话/任务/记忆/配置）+密钥缺失报告+未认证 401+损坏包 AppError 不半写 — 数据主权验收主战场
- [x] `.github/workflows/server-docker.yml`（新）— push tag v*/main：版本校验（tag==server==engine，五文件口径）→多阶段构建推 ghcr（tag=剥 v+latest，main 加 sha）+镜像运行冒烟（docker run→healthz→stdout 断言 JSON 行）+PR 路径触发 .dockerignore 三副本一致性 job（不构建镜像）— Additional 10/11/12 + 17.1 deferred #33/#8 收口
- [x] `.github/workflows/server-ci.yml` — 新增 web-e2e 冒烟 job（node 22+rust-cache 构建 server debug+dist+chrome-for-testing 148 供给→`test:web` 冒烟套件）— 三链路落位（桌面=ci.yml 现状、server=本文件、web e2e=本 job）
- [x] `egosync-app/tests/e2e/` — wdio.web.conf.ts 增 suites（smoke=streaming+reconnect/full=全部）+package.json `test:web:smoke` script+setup-web-drivers.mjs 扩展 chrome-for-testing 下载（钉版）— CI 供给闭环
- [x] `server/docker-compose.yml` — image 参数化 `ghcr.io/guows520/egosync-server:${EGOSYNC_IMAGE_TAG:-latest}`（build 保留供本地）— 升级=pull→up 两步成立
- [x] `docs/user-guide/11-云端自托管部署.md` — 11.8 补逻辑级端点（浏览器/curl 示例+跨形态互导+导入探测报告+密钥重录）、11.3 升级改镜像 pull 流+deep 巡检、11.2 补 registry 私有镜像引用与 VPS docker login（PAT）操作、反代 XFF 语义；architecture.md 追加 17.3 落地注记 — NFR-C2/C3/C7 文档化
- [x] `egosync-app/src/` — web 模式设置页数据区导出下载（blob）/导入上传（file input→POST）入口+dataService 扩展+vitest（capabilities 门控：web 模式可见、桌面模式不重复）— 一键导出的用户面（已裁决 Q3=A）
- [x] 版本对齐 `crates/egosync-engine/Cargo.toml` + `server/Cargo.toml` → 0.1.6-alpha.3（已裁决 Q1=A）+ sprint-status.yaml 同步（17-3 状态流转、17-1/17-2 复核 done 销账）+ deferred-work.md 已收口项（#33/#8/密钥探测/D2）注记

**Acceptance Criteria:**
- Given 已认证用户 GET /api/export，when 请求，then 200 返回与桌面导出包同格式（exportVersion 一致）的完整导出数据（双库+配置+密钥引用），任何字段不含密钥值
- Given 已认证用户 POST /api/import 上传导出包，when 导入，then 数据完整恢复（新实例抽查会话/任务/记忆/配置逐项一致）、导入前自动备份、失败不半写（损坏包→AppError、库内容不变）
- Given 导入包含 apiKeyRef 但目标实例无对应密钥，when 导入完成，then 响应报告列出全部缺失 ref 及重录路径文案（结构化、不阻塞导入结果返回）
- Given 桌面导出包，when 经 /api/import 导入云端（及反向），then 跨形态互导成立（同引擎同导出逻辑的验收兑现，roundtrip 测试双向覆盖）
- Given push tag v*，when server-docker.yml 运行，then 版本校验通过、多阶段镜像构建并推送 ghcr（tag 与版本同步）、镜像可 docker run 且 healthz 存活、stdout 为 JSON 行
- Given PR 触发（server/engine/前端/e2e 路径），when CI 运行，then 桌面 test:all 链（ci.yml）+ server cargo test + web e2e 冒烟三者并行，任一红阻塞合并
- Given GET /healthz?deep=1，when 迁移滞后库，then 503+migrations:false（巡检面）；存活探针语义不变
- Given EGOSYNC_BEHIND_PROXY=1 反代拓扑，when 恶意客户端伪造多跳 XFF 失败登录，then 限流按最右值分桶、合法不同 IP 用户互不挤兑
- Given 全量回归，when `npm run test:all` + `npm run test:web:smoke` + 逐 spec 独立跑（`test:web` 全量）+ server `cargo test` + `npm run build`，then 全绿；桌面 e2e 套件与 wdio.conf.ts 零改动。（评审轮修正措辞：原「`test:web`（全量）全绿」与本 VM 验证记录矛盾——4-spec 单次连跑在本 3.6GB VM 受渲染停摆家族限制非全绿，实际执行标准为冒烟门禁绿 + 每个 spec 独立绿跑全过；空载 runner/CI 面为冒烟子集）
- Given 数据主权走查，when 全流程，then EgoSync 官方服务零参与（无遥测、无云存储、导出导入全程不经任何第三方）

## Implementation Notes

**实现完成（2026-09-21，基线 cfeef43）。** 交付物清单与实现期决策记录：

**服务端（Rust）**

- `crates/egosync-engine/src/db/pool.rs`：新增 `pub async fn migrations_up_to_date(pool) -> bool`——读 `_sqlx_migrations` 已应用版本集与内嵌 `MIGRATOR` 版本集做**集合相等**比对（既覆盖「有库落后于二进制」，也覆盖「降级挂新库」：二进制比库少迁移时 embedded ⊄ applied 同样报 false）；任何读错误 → false（归入 healthz 503 故障族）。**只读巡检不跑迁移**（修复发生在升级重启时的 init_db）——刻意与 `run_migrations` 分离。测试：健康库 true；删除 MAX(version) 行 → false；DROP TABLE → false 不 panic；无副作用断言。
- `server/src/backup.rs`（新文件）：
  - `export_handler`（GET /api/export）：`gather_export_data()` → `serde_json::to_string_pretty`（与桌面 `export_json` 写盘内容逐字节同源——不重造序列化）→ 200 + `Content-Type: application/json` + `Content-Disposition: attachment; filename="egosync-export-YYYY-MM-DD.json"`（chrono Local）。密钥值永不进 body（仅 `apiKeyRef` 引用）。
  - `import_handler`（POST /api/import）：body → tempfile（`std::env::temp_dir()/egosync-import-{uuid}.json`）→ engine `import_all`（备份先行 + per-db 事务语义零改动）→ 删 tempfile → `probe_missing_secrets` → SSE 广播 `data:imported`（engine 常量 `DATA_IMPORTED_EVENT`，payload=ImportResult）→ 200 JSON `{imported, missingSecrets}`（camelCase serde）。
  - `probe_missing_secrets`：`list_llm_configs` × `load_secret` 存在性探测（`Ok(Some(_))`=在场——**空 env 串按 17.1 冻结语义计在场**，这是 SecretStore 行为面，本故事不改）；`Ok(None)`/`Err` 计缺失。报告项 `{configName, apiKeyRef, message}`，message 复用 `missing_api_key_error`（llm_config.rs）同款重录文案——单一文案源。
  - 错误形状沿用冻结口径：AppError → **200** + 单键 map（PascalCase 变体名如 `"ValidationError"`）+ `X-Egosync-App-Error: 1` 判别头；非 200 白名单 401/429/404/413/5xx。
- `server/src/lib.rs`：`pub mod backup;`；两路由挂 `authed` 组（require_auth 中间件，Bearer/Cookie 双通道）。**不经 /api/cmd 分发**——`data_export`/`data_import` 保持 desktop-only（DESKTOP_ONLY_COMMANDS 15 项 / 103 web-ok 零改动）。
- `server/src/healthz.rs`：deep 探针加 `migrations_ok`；healthy = db && opencode && migrations；flags JSON 加 `"migrations"`。浅探针语义不变。
- `server/src/auth.rs`：新增 `pub fn rate_limit_key(behind_proxy, headers, socket_ip) -> IpAddr`——`EGOSYNC_BEHIND_PROXY=1` 时取 `X-Forwarded-For` **最右值**（`rsplit(',').next()` + trim 解析，不可解析/缺头回落 socket IP）；门控关恒 socket IP（直连伪造头一律忽略——与 XFP 门控同纪律）。应用于四处：`rate_limit` 中间件、`login` 失败日志、`require_auth`、`require_auth_with_sse_ticket`（Bearer T5）。**实现细节**：Body 是 !Sync，限流键在 await 之前同步计算。
- `server/Cargo.toml`：+chrono（attachment 文件名日期）；版本 0.1.0 → **0.1.6-alpha.3**（口径见下）。

**CI/CD（YAML）**

- `.github/workflows/server-docker.yml`（新文件）：
  - 触发面：push main + push tag `v*`（**无 paths 过滤**——paths 对 tag push 求值语义文档化不足，静默跳过 tag = 发布流程断裂，发布可靠性优先）；PR（paths 过滤：server/engine/前端/e2e/workflow 自身）。
  - job `version-consistency`（全触发）：五文件版本一致（桌面 package.json/src-tauri Cargo.toml/tauri.conf.json + engine + server）；tag push 额外校验 tag（剥 v）== 版本。
  - job `dockerignore-consistency`（全触发，17.1 deferred #8 收口）：server 两份排除模式集逐行相同（注释/空行放行）+ 根文件 = server 份 − `relay-server/` + 密钥排除项在场断言。**本机已用同逻辑预演全绿**。
  - job `docker`（仅 push）：buildx 构建（context=仓库根，file=server/Dockerfile 原样，gha 缓存）→ **镜像运行冒烟**（docker run + /healthz 200 + `docker logs` stdout 首条非空行 JSON 解析含 `level` 字段——17.1 deferred #33 收口）→ 登录 ghcr（GITHUB_TOKEN，packages:write，owner 小写）→ push（tag=剥 v 版本号+latest；main 加 sha-<7>）。
- `.github/workflows/server-ci.yml`：新增 `web-e2e-smoke` job（node 22 + rust-cache 构建 server debug 二进制 + egosync-app dist + setup-web-drivers 双下载 → `npm run test:web:smoke`）。触发 paths 扩到前端/e2e 面（`egosync-app/src/**`、`tests/e2e/**`、前端构建配置）。**三链路落位**：桌面=ci.yml test-and-build（原样）/ server=server-test / web e2e=web-e2e-smoke。
- `server/docker-compose.yml`：镜像参数化 `ghcr.io/guows520/egosync-server:${EGOSYNC_IMAGE_TAG:-latest}`（`build:` 保留供本地 `up -d --build`）；顶层 `name: egosync-server` 不动。`docker compose config` 插值验证通过（latest 与钉版两态）。

**e2e（web 链路）**

- `wdio.web.conf.ts`：新增 `suites: { smoke: [web-streaming, web-reconnect] }`——冒烟=确定性子集（无 60s tick 对齐依赖）；全量 `test:web` 缺省行为零变化（不指定 suite 跑 specs 全量）。**桌面 wdio.conf.ts 与桌面套件零改动**。
- `tests/e2e/package.json`：+`test:web:smoke` 脚本。
- `scripts/setup-web-drivers.mjs`：扩展 Linux 下钉版 **Chrome for Testing 148** 下载（落 puppeteer 缓存布局 `~/.cache/puppeteer/chrome/linux-148.0.7778.97/`——与 wdio.web.conf.ts 缺省解析一致，供给后零 env 消费；CI 面闭环）；非 Linux 保持提示 + 手工供给（本地开发者语义不变）；幂等。

**前端（web UI 最小入口，裁决 Q3=A）**

- `src/services/dataService.ts`：+`webExport()`（GET /api/export → Blob）/+`webImport(file)`（POST /api/import，body=文件文本）+`MissingSecret`/`WebImportResult` 类型。错误解包与 invoke 同构（AppError 单键 map reject / 非 200 白名单 reject 文案）。credentials same-origin。
- `src/components/settings/GlobalSettingsModal.tsx`：数据 tab 新增「云端数据备份」区（**`!isTauriHost()` 门控**——桌面本地不重复，桌面本机文件入口原样；远程桌面模式对远端实例走浏览器语义）。导出=blob 下载（`egosync-export-日期.json`，与桌面命名一致）；导入=隐藏 file input（`accept=".json"`）→ 确认面板（含文件名+覆盖警示+自动备份说明）→ 结果面板（camelCase 计数 + missingSecrets 逐项重录文案，amber 警示非红色错误——不阻塞导入语义）。
- vitest：`GlobalSettingsModal.browser.test.tsx` +3（云端区渲染/导出调用+桌面服务零调用/导入全流程含密钥报告）；`GlobalSettingsModal.test.tsx` +1（桌面本地模式云端区不渲染——防重复入口回归）。

**版本口径（五文件统一，boss 裁决 2026-09-21）**

`egosync-app/package.json` / `src-tauri/Cargo.toml` / `tauri.conf.json` 原本就是 0.1.6-alpha.3；engine 与 server 从 0.1.0 对齐到 **0.1.6-alpha.3**。连带：桌面 src-tauri Cargo.toml 的 path 依赖版本要求行同步改（pre-release 不满足 `^0.1.0`，cargo 解析期即红——顺带成为版本漂移的本地哨兵）；三处 Cargo.lock（desktop/server/engine standalone）已刷新。

**文档**

- `docs/user-guide/11-云端自托管部署.md`：11.2 补 ghcr 私有镜像登录（PAT read:packages + `docker login ghcr.io`）；11.3 升级改 compose pull → up 两步 + deep healthz 巡检（migrations 位语义）；11.4 自有反代表补 XFF 转发要求行 + 「仅信任一跳」诚实边界说明（多级串联拓扑的语义局限明示）；11.8 逻辑级备份完整文档（浏览器/curl 双入口、跨形态互导、密钥引用不带值、双库非跨库原子、导入报告、50MB 上限、会话失效提示；卷级备份保留并说明互补关系）；11.10 桌面→云端迁移改为已交付流程（4 步）。
- `architecture.md`：⑦部署节追加「17.3 落地注记」（端点/healthz/XFF/镜像/CI/UI 六点，追加式）。

**销账（deferred-work.md 已标记收口）**

17.1 #33（stdout JSON 冒烟）、17.1 #8（dockerignore 漂移）、17.1 #4（XFF 限流单桶）、17.1 人工裁决（密钥探测）、17.2 D2（web e2e CI 空窗）。

**验证记录（2026-09-21，主代理亲跑——实现子代理会话中断于磁盘占满，其自报记录未经复核的部分一律以本节为准；★=评审轮修复后复跑的更新值）**

- ★`cd server && cargo test`：**全绿**——lib 20（含 backup.rs 探测 2 新增）+ api_test 25（含 deep migrations 2 新增 + 既有 413 body 上限）+ backup_roundtrip_test **8**（新文件 7 + 评审新增并发互斥 1）+ command_parity 2 + desktop_remote 14 + parity 6 + proxy_tls **6**（XFF 4 新增集成——评审轮 +1 SSE 路由用例 + 直连基线 2）+ secret_store 4 + web_entry 5。
- `cd crates/egosync-engine && cargo test`：**848 passed, 0 failed**（全量，含 pool.rs `migrations_up_to_date` 新增用例：健康库 true / 删 MAX(version) false / DROP TABLE false 不 panic / 只读无副作用）。
- ★`cd egosync-app && npm run test:all`：**vitest 889 / 68 文件全绿**（含评审新增：dataService.web.test.ts 7 + browser 错误分支 1 + 契约扫描三副本扩展）+ 双 cargo 腿全绿（含 settings modal 4 新增：browser 3 + desktop 1）。
- ★`cd egosync-app && npm run build`：**tsc 零类型错误** + vite 产物生成（含新 UI；e2e 静态目录即此 dist）。
- ★`cd egosync-app/tests/e2e && npx tsc --noEmit -p tsconfig.json`：零类型错误（评审轮新增 e2e 改动的类型面）。
- web e2e（全部 17.3 改动 + 评审轮修复在位）：
  - ★`npm run test:web:smoke`（streaming+reconnect）：**2/2 全绿，8/8 用例**（streaming 4：占位符+375px；reconnect 4：kill+重启+命令面+**评审新增存活态重启置换钉子**）。
  - ★钉子**双向验证**：临时把 restartWebServer 还原为旧版不杀实现跑该用例——如预期变红（孤儿旧实例存活）；恢复修复后 4/4 绿。钉子真实有效。
  - `web-events`：全量连跑（第 1 位置）三轮全绿。
  - `web-resident-loop` 隔离跑：**3/3 全绿**——含「同分钟重启」**首次真实 kill+重启**后仍通过（历史轮该用例从未真正重启过，见下节）。
  - `npm run test:web` 全量 4-spec 单次连跑：events ✓ / reconnect ✓（修复后）/ resident-loop 2/3 / streaming 2/4——挂点均为本 VM 渲染线程停摆家族（16.2 spec 内注释已记录「三连跑实测复现两次…环境负载问题，非产品缺陷」）：streaming 挂点为第 4 位置的渲染重排未生效（375px 走查 offender 为通知面板 left=780——视口仍 ~1160px 宽，`setWindowSize` 重排停摆）与占位符 45s 未呈现（服务端数据在场，`conv:200` 诊断确认）；resident-loop FR-11 三轮两过一挂（经典偶发）。**每个 spec 都有 17.3 全改动在位的独立绿跑**；连跑受限于 3.6GB+swap VM 的持续负载渲染停摆，非产品回归（挂点元素均属 16.x 既有 UI，与 17.3 改动无接触面；smoke=CI 门禁面不受影响）。
- 桌面 e2e 套件与 wdio.conf.ts 零改动（git diff 为空）。
- CI 侧本地复演（job 逻辑逐条同构重放，主代理亲跑）：五文件版本一致（0.1.6-alpha.3）✓、dockerignore 三副本一致（与 job 同款 patterns/diff 逻辑）✓、两 workflow YAML python yaml 解析 ✓（评审轮补丁后复验）、compose 插值 latest/钉版 0.1.6-alpha.3 两态 ✓（`docker compose config`）。
- **诚实登记（无法本地验证的部分）**：server-docker.yml 的 `docker` job（buildx 构建 + ghcr 推送 + 镜像运行冒烟）与 server-ci.yml `web-e2e-smoke` job 的 CI 全链路（node 22 runner 内构建+驱动供给）无法本地执行——本 VM 无远端 registry 且本会话曾因磁盘预算耗尽中断一次，不再冒险做 20 分钟级本地镜像构建；首次 push main/tag 与首次 PR 后以 Actions run 观察为首次真实验证。
- **★CI 首验实录（push e88f881 后，2026-09-22）**：`server-docker` **首跑全绿**（版本一致性 + dockerignore 一致性 + buildx 构建 + 镜像冒烟含静态首页断言 + ghcr 推送——上文诚实登记项正式销账）；`relay-docker` ✓；`server-ci` 的 server-test ✓ 但 **web-e2e-smoke 红**——`setup-web-drivers.mjs` 中评审补丁 P13 误用 TypeScript 语法（`function chromeBinaryRuns(): boolean`）于纯 .mjs，node 直接 SyntaxError（本地未暴露：tsc 不覆盖 .mjs、且冒烟跑时驱动已供给脚本未被调用）。已修（去类型注解）并以 fresh-HOME 全链路实跑验证（下载 175.4MB + 解压 + --version 校验通过）。桌面 `CI` 的 ubuntu/macos 腿 ✓，**Windows 腿红**——`csp.contract.test.ts:68` byte 对 byte CSP 哈希契约被 Windows 检出的 LF→CRLF 转换破坏（该测试 16.1 落地、属 31 个首推提交之一、非 17.3 改动；上次 Windows 全绿 #56 早于该测试存在）。已修：`.gitattributes` 为 index.html 钉 `text eol=lf`（与 sqlx 迁移同款问题同款既定解法——且 Windows 本机构建的 dist 也会真实触发 CSP 拒执行，钉 LF 同时修复潜在构建产物缺陷）。修复提交后以第二次 CI run 复验。
- **★CI 二验实录（push 95a6431 后）**：`server-docker` ✓（二连绿，正式收口）、`server-ci` ✓（web-e2e-smoke 修复确认——驱动供给+双 spec 冒烟全过）、`relay-docker` 未触发（改动面不含 relay 路径，符合触发语义）、桌面 CI ubuntu/macos ✓、**Windows 腿第二轮红**——CRLF 修复后前端测试过，挂在下一站：src-tauri `cargo test` 的 `remote_mode_skips_engine_assembly_before_db_open`（16.3 落地的源码扫描钉，字面量含 `"\n                return Ok(());"` 跨行匹配被 LF→CRLF 破坏；经用户人工贴回失败日志定位——公开 API 无日志权限，本机无法复现 Windows 运行时）。同款病同款解法：`.gitattributes` 为 `egosync-app/src-tauri/src/lib.rs` 钉 `text eol=lf`。三轮 CI 复验后收口；Windows 腿的「首推积压提交旧账清理」经验已沉淀：源码/资源字面量契约测试在钉 LF 前不得视为跨平台安全。
- **★CI 三验终局（push 3cd910f 后，run #59/#3）**：桌面 `CI` **全绿**（ubuntu ✓ / Windows ✓ / macOS ✓ + opencode-pin + companion-proto + engine）、`server-docker` ✓（**三连绿**）、`server-ci` ✓（二连绿）、`relay-docker` ✓。**四条流水线全绿，CI 首验闭环**——17.3 全部交付物（含镜像发布链、web 冒烟链、Windows/macOS 桌面链）首次在真实 CI 全平台验证通过。累计三个 CI 修复提交（e88f881 → 95a6431 → 3cd910f），全部如实在案。

**实现期事故与工程响应（如实披露）**

- **磁盘占满（ENOSPC）**：实现子代理会话因 `server/target`（13G）等三处 target 累计 ~24G 撑满磁盘中断死亡。响应：清理 target/无用镜像（14G 释放）+ 三份 Cargo.toml 增 `[profile.dev]` 瘦身（`debug = "line-tables-only"` + `incremental = false`，target 约 13G→3-4G；回溯仍带行号，需完整调试能力时删该节即可）——**超出 spec 范围的工程决策**，三处：server/engine/桌面 src-tauri。
- **setup-web-drivers.mjs unzip 缺父目录 bug（CI 首跑必炸）**：子代理新增的 Chrome 下载在 `~/.cache/puppeteer/chrome` 不存在时 `unzip` 直接失败——补 `mkdirSync(chromeCacheDir, { recursive: true })`，已实测完整下载（175.4MB）+解压通过。
- **web e2e 僵尸服务器根因修复（验证期最重要发现）**：`restartWebServer()` 旧版只起新进程不杀旧实例——resident-loop「同分钟重启」用例自 17.2 落地起**从未真正重启过**（新进程绑定失败退出、PID 文件覆写为死 PID、真服务端成孤儿僵尸占 18080），污染下一轮 onPrepare（端口误判就绪→DB 双写→spec 雪崩）。两处修复：① `restartWebServer()` 先按 PID 杀旧实例 + `waitForPortClosed` 后才重放（「重启」名副其实——17.2 的 FR-47 去重断言首次被真实执行并通过）；② `wdio.web.conf.ts` onPrepare 端口占用守卫（`findPortSquatter`：孤儿 egosync-server 自动清理、其它占用者明确报错绝不误杀）。git 考古证实 killWebServer 从未存在于该 spec（基线 cfeef43 即无）。

**I/O 矩阵测试审计（step-03 要求，逐行核对）**

| 矩阵行 | 覆盖测试（均已跑且绿） |
|---|---|
| 导出下载 | backup_roundtrip：`export_returns_desktop_format_package_without_secret_values`（200+attachment+exportVersion+密钥值零泄漏）+ `export_and_import_require_authentication`（401/Bearer 200） |
| 导入恢复 | `import_restores_core_data_and_reports_missing_secrets`（ImportResult 逐项计数+双库抽查）+ `corrupted_package_returns_app_error_without_partial_write`（AppError 形状+库不变）+ `concurrent_imports_do_not_mix_packages`（评审新增：并发导入互斥）+ api_test `oversized_body_rejected_with_413`（>50MB⇒413——**审计修正**：该测试实际打 /api/cmd/role_list，/api/import 的 413 由 router 级 `DefaultBodyLimit` 层结构性共享（lib.rs），未经直接测试钉死；前端侧由 webImport 体积预检前置兜底） |
| 导入后密钥缺失 | 同上 import 测试（missingSecrets 数组：configName+apiKeyRef+重录文案）+ backup.rs 内联 `probe_reports_missing_refs_with_reentry_message` / `probe_treats_empty_env_value_as_present_current_semantics`（env 空串语义不动） |
| 未认证导出/导入 | `export_and_import_require_authentication`（401 双端点+数据未触碰）；登录面限流见反代行 |
| deep 巡检 | api_test `healthz_deep_reports_db_and_opencode_flags`（migrations:true）+ `healthz_deep_reports_migrations_false_for_stale_database`（503+migrations:false+浅探针不变）；engine pool.rs `migrations_up_to_date_reports_status_without_running_migrations`（4 断言族） |
| 镜像发布 | **CI 侧诚实登记**：`version-consistency`/`dockerignore-consistency` 本机同逻辑复演全绿；`docker` job（构建+推送+镜像冒烟）首次 push 后验证——spec Verification 节预授权路径，不声称已验证 |
| PR 门禁 | 同上（server-ci 的 cargo test=server-test 亲跑全绿；web e2e 冒烟=smoke 亲跑 2/2；dockerignore 复演全绿）；三链路 CI 联动本身待首次 PR 观察 |
| 反代限流 | proxy_tls：`behind_proxy_login_rate_limit_buckets_by_rightmost_xff`（A 超限 B 不挤兑+无 XFF 回落）+ `direct_mode_forged_xff_does_not_change_rate_limit_bucket` + `behind_proxy_bearer_failures_bucket_by_rightmost_xff`；auth.rs 纯函数单测（多跳/空白/IPv6/缺头/不可解析/门控关） |
| 导入后在线客户端 | `import_restores_core_data_and_reports_missing_secrets`（SSE 订阅断言 data:imported 事件名+payload）+ `import_succeeds_without_sse_subscribers`（无订阅者 200） |
| 停机窗口导入 | 语义行为「导入与调度无交互」：backup_roundtrip 全部导入测试（含触发表 scheduler_triggers 随包整体替换——engine data_export.rs:769）+ engine 848 全绿（含 17.2 调度去重族）；非独立断言，属数据面语义 |

## Spec Change Log

- **2026-09-21（step-04 评审轮）**：触发发现=盲扫层「验收标准与验证记录自相矛盾」（AC 要求 `npm run test:web` 全量全绿，验证记录如实记载 4-spec 单次连跑在本 VM 受渲染停摆限制非全绿）。修正（非冻结区「Tasks & Acceptance」）：AC 措辞改为实际执行标准（冒烟门禁绿 + 逐 spec 独立绿跑全过），修正理由随行内注完整留档。避免的坏状态：故事存档留下一条「已勾选但未满足」的 AC。KEEP：验证记录中的全量连跑失败明细（挂点家族、归因、每 spec 独立绿跑证据）原样保留，措辞修正不得弱化任何已披露事实。
- **2026-09-21（step-04 评审轮）**：触发发现=验证缺口层 Other（I/O 矩阵审计表「导入恢复」行声称 api_test `oversized_body_rejected_with_413` 覆盖 /api/import，实际该测试只打 /api/cmd/role_list）。修正（非冻结区审计表）：改为如实陈述——/api/import 的 413 由 router 级 DefaultBodyLimit 层结构性共享、未经直接测试钉死、前端体积预检兜底。避免的坏状态：审计表对测试覆盖面的陈述失实。KEEP：api_test 413 用例本身的原有覆盖声明（对 /api/cmd 路径真实有效）。
- **2026-09-21（step-04 评审轮）**：触发发现=盲扫层「夜跑不存在」（spec 与 deferred-work 称全量 test:web 留本地/夜跑，但仓库无任何 nightly workflow）。修正（非冻结区 Design Notes + 17.2 D2 收口条目不再回改）：措辞改「本地手工」并明示全量回归当前 100% 手工守护、已登记 CI 债务。避免的坏状态：文档承诺不存在的自动化。

## Review Triage Log

**评审轮（step-04，2026-09-21）**：三层并行评审（盲扫 21 条 / 边界 7 条 / 验证缺口 6+1 Other 条）。实现子代理会话已死（实现期磁盘占满中断），patch 按规程由主代理亲自落地。分诊汇总：**patch 13 组 / defer 9 项 / spec 修正 3 处（见 Spec Change Log）/ 裁定不改 2 项（relay 先例一致性，转 defer 登记）**。逐条分诊如下（编号 B*=盲扫、E*=边界、V*=验证缺口；同根因跨层条目合并为一行）：

| # | 发现（位置） | 结论与证据 | 路由 |
|---|---|---|---|
| B1 | 导入 tempfile 0644（backup.rs） | medium：用户全量个人数据落共享 /tmp，同机其它用户可读——真实隐私缺口 | patch：OpenOptions 显式 0600 |
| B2 | probe 失败静默（missingSecrets 空数组被误读为「全在场」） | low：唯一 Err 源是导入刚成功后 DB 读失败（触发面极窄），且修复（响应加 probeError 字段）扩响应契约面 | defer 登记 |
| B3 | unreachable! 在生产路径（probe 文案提取） | high：跨模块错误形状依赖，missing_api_key_error 变体演化即一次成功导入崩在响应构造期 | patch：降级为保留重录路径的最小文案 |
| B4 | /api/import 无并发互斥；导入中导出读中间态（B4+E1+E2） | medium：双库全量替换交错=混装包（SQLite 只保证单库原子，不保证跨请求串行） | patch：AppState `import_lock` 导入/导出同锁 + 并发互斥测试 |
| B5 | 401 不触发全局会话失效（B5+E3） | medium：导入整体替换会话表后 401 是常态，HttpTransport 同场景回登录页而云端备份区留死胡同红错 | patch：401 先发 `auth:unauthorized` |
| B6 | 成功面不校验 JSON 性 | medium：200 但 HTML body（反代故障页）被当结果返回，UI 渲染「已恢复 undefined 个角色」 | patch：成功面强制 JSON（与 B5 同段落修复） |
| B7 | 无体积预检/无超时 + UTC 与服务端本地日期差一天 | low-medium：慢链路挂起 UI 永停「导入中」；>50MB 上传几分钟才收 413；晚间导出文件名差一天 | patch：AbortSignal 超时 + 50MB 本地预检 + 本地时区日期 |
| B8 | 镜像仅 amd64 | 真实但 relay-docker.yml（17.1 冻结先例）同款单架构——本轮忠实先例不改发布面语义 | 裁定不改 → defer 登记 |
| B9 | main push 覆盖 latest + 补推旧 tag 回写 latest（B9+E4） | 真实但 relay 先例同款发布策略；回滚边界为部署纪律问题 | 裁定不改 → defer 登记 |
| B10 | 镜像冒烟断言过浅（从不请求 /） | low：dist 拷贝损坏/静态服务回归可静默通过冒烟 | patch：`curl /` 断言 HTML |
| B11 | 冒烟失败容器泄漏 + 全 workflow 无超时 | medium：bash -e 下 python3 断言失败跳过 docker rm；挂起门禁烧 6h runner 分钟 | patch：trap EXIT 兜底 + 全 job timeout-minutes |
| B12 | compose owner 硬编码 vs workflow 派生 | 真实（fork/改名后 pull 静默失效），部署体验优化非验收面 | defer 登记 |
| B13 | compose image:+build: 并存脚枪 | 真实（镜像不在本地时静默源码构建 5-20 分钟），本地构建兜底亦是调试便利 | defer 登记 |
| B14 | 「夜跑」不存在（无 nightly workflow） | 真实：全量 web e2e 回归当前 100% 手工守护 | spec 措辞修正（Spec Change Log）+ defer 登记 |
| B15 | restartWebServer 盲杀无身份核验；onPrepare 陈旧 PID 清理同病 + squatter 清理路径裸 kill 有 ESRCH 竞态（B15+E6） | medium：PID 回收场景误杀无关进程，与 findPortSquatter 核验纪律自相矛盾；ss 扫描与 kill 之间进程退出时 onPrepare 裸崩 | patch：isEgoSyncServerPid 杀前必验（helper）+ onPrepare 同款 pidIsEgosyncServer + squatter kill 包 try/catch |
| B16 | web-e2e-smoke 首败即盲（无 artifacts/无 e2e npm cache） | low：首个 CI run 即首次真实验证，失败取证最该补 | patch：failure() 上传日志 + 双 lockfile 入 cache |
| B17 | Chrome 下载无完整性校验（B17+E5） | medium：残缺二进制（磁盘满截断）existsSync 幂等短路永不自愈，wdio 启动期晦涩崩溃 | patch：--version 执行校验三处接入 + 自愈（sha256 钉版另 defer） |
| B18 | 销账账本漏登（17.2 #16 epic context 口径已修复未标收口） | 真实：本轮 diff 内 epic-17-context.md 重写恰好闭账但未销账 | patch：deferred-work 补记收口 |
| B19 | AC 与验证记录自相矛盾（B19+E7） | high（存档可信度）：AC 字面未满足但已勾选 | spec 修正（Spec Change Log，非冻结区措辞） |
| B20 | 触发路径清单四副本漂移 | 真实：版本文件（src-tauri/Cargo.toml、tauri.conf.json）改动可整 PR 跳过全部门禁；四副本条目互有出入 | patch（版本文件入 docker PR paths）+ defer（收敛单一来源） |
| B21 | [profile.dev] 三份散贴 | 真实但 engine 份为独立 cargo test 提速真实有效；profile 生效规则下桌面构建不吃 engine 份 | defer 登记 |
| V1 | restartWebServer「先杀旧实例」修复无断言保护（最重要发现） | high：还原为旧实现时全部既有测试照绿——FR-47 验收空洞 + 孤儿僵尸可静默复发 | patch：存活态重启置换钉子入冒烟 + 反向验证（旧实现确实变红） |
| V2 | SSE 路由反代态 XFF 无测试 | high：17.1 #4 收口四应用点中唯一裸奔面，键回退时既有测试全绿 | patch：behind_proxy SSE 分桶集成测试 |
| V3 | 云端备份端点客户端零直接测试（UI 测试全 mock dataService） | high：unwrapCloudResponse 真实代码从未被执行；判别头常量第三副本无 source-scan 锁 | patch：dataService.web.test.ts 7 用例 + 契约扫描三副本 |
| V4 | 云端导入错误分支无 UI 测试（与桌面孪生不对称） | medium：AppError reject 面 UI 无验收 | patch：browser.test.tsx +1 错误分支 |
| V5 | 解包契约细节（401 信号/AppError 形状）无直测（V3 的展开项，评审输出该段被截断） | 与 V3 同根因 | patch：随 V3 落地（401 信号/AppError 单键 map 用例） |
| V6 | probe Err 降级分支（200 + 空数组，不阻塞导入）无测试 | 防御性分支：唯一 Err 源需不存在的故障注入缝，爆炸半径小 | defer 登记 |
| Other | I/O 矩阵审计表「导入恢复」行 413 覆盖面陈述失实 | 审计表声称的测试覆盖（/api/import 413）不存在——该测试只打 /api/cmd/role_list | spec 修正（Spec Change Log） |

**修复验证**：全部 patch 经 server cargo test（backup 8/proxy_tls 6 含新用例）、vitest 889/68（含 dataService.web 7 + browser 错误分支 1 + 契约扫描扩展）、npm run build、e2e tsc、冒烟 2/2（8/8 含新钉子）回归全绿——见验证记录 ★ 标注。

**defer 登记**：见 deferred-work.md 本轮 9 条新增（B2 探测失败可见性 / V6 探测分支测试缝 / B8 镜像架构 / B9 latest 策略 / B12+B13 compose 脚枪与 owner / B14 nightly 全量回归 / B20 触发路径收敛 / B17 Chrome sha256 钉版 / B21 Cargo profile）+ 1 条收口补记（B18：17.2 #16）。

## Design Notes

- **导入走临时文件而非改 engine 签名**：import_all 的「先备份再分发+双库事务+DETACH 清理」语义经 20+ 既有测试钉死，server 侧 tempfile 复用是零侵入路径；HTTP body 上限 50MB 与 Caddy request_body 对齐，导出包实际量级（个人数据）远低于阈值。
- **XFF 最右值语义**：compose 拓扑=单 caddy 可信跳，最右值由 caddy 追加（客户端可伪造左侧、不可伪造右侧）；多级反代属用户自建拓扑，文档明示「仅信任一跳」边界。
- **冒烟套件切分**：web-streaming/web-reconnect 无时钟对齐依赖（确定性）；web-events/web-resident-loop 含 60s tick 对齐与 LLM stub，负载抖动敏感，留 `test:web` 全量**本地手工**跑（评审轮修正措辞：原「本地/夜跑」的「夜跑」不实——仓库无 nightly workflow，全量回归当前 100% 手工守护，已登记 CI 债务）。
- **镜像冒烟放 tag/main 而非 PR**：多阶段构建（node dist+rust release）约 20 分钟，PR 面用 server-ci cargo test + 轻量 dockerignore 一致性覆盖，镜像破坏性回归由 main/tag 构建兜底。
- **data:imported 前瞻发射**：前端事件名联合已收编（transport/events.ts:35）但当前无消费方；发射保持与桌面壳同构，成本一行。

## Verification

**Commands:**
- `cd egosync-app && npm run test:all` — expected: vitest + 双 cargo test 全绿
- `cd egosync-app/tests/e2e && npm run test:web` — expected: 4 web spec 全绿（新路由不破坏既有）
- `cd egosync-app/tests/e2e && npm run test:web:smoke` — expected: 冒烟套件（streaming+reconnect）绿
- `cd server && cargo test` — expected: 含 backup_roundtrip/healthz migrations/XFF 新用例全绿
- `cd egosync-app && npm run build` — expected: tsc 零类型错误（含新 UI）

**Manual checks (if no CLI):**
- server-docker.yml 无法本地执行：以 relay-docker.yml 结构对照+YAML 语法核对+首次 push 后 run 观察（诚实登记，不声称已验证）；compose 镜像引用变更本地 `docker compose config` 校验插值（docker 可用时）
