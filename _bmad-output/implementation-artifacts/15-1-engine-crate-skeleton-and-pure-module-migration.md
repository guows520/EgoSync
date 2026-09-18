---
title: '引擎 crate 骨架、宿主接缝与纯模块平移（Story 15.1）'
type: 'refactor'
created: '2026-09-17'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '9979f3e3c998c49714627482af09fe1e019e4dfb'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-15-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** EgoSync 全部业务逻辑物理耦合在 Tauri 壳（egosync-app/src-tauri）内，云端托管赛道需要引擎脱离 Tauri 运行，但当前没有 `crates/egosync-engine` 物理形态；keyring 密钥读写与 sidecar 二进制路径是横在后续解耦故事前的两个宿主卡点。

**Approach:** 新建无 tauri/keyring 依赖的 `crates/egosync-engine` crate，将依赖闭合的纯逻辑模块（db/、models/、llm/、error.rs、migrations/ 与 12 个纯 services）逐字节平移入内；就位两条宿主接缝——engine 定义 `SecretStore` trait（桌面壳提供 keyring 实现并注入）、sidecar 路径经入参注入（现状 `SidecarManager::new(resource_dir, port)` 已满足，仅需平移）；src-tauri 以 path 依赖 + `pub use` 回引，commands/ 与留守模块的 `crate::` 路径引用零改动，桌面行为零变化。

## Boundaries & Constraints

**Always:** 平移文件内容逐字节等价，仅允许两类改动——import 路径调整与接缝调用替换；engine Cargo.toml 物理不声明 tauri 与 keyring（CI 断言兜底），其余依赖按迁移集实际用量声明且版本与 src-tauri 一致；仓库根禁建 Cargo workspace（companion-proto 先例）；内联 `#[cfg(test)]` 测试随文件迁移且全部通过；收口 = `npm run test:all` 全绿 + tests/e2e 全量全绿，跳过任何一项即未完成（e2e 部分经 2026-09-17 用户裁决：环境级阻断证据豁免——基线对照实验证明先于本故事，另立独立工作项修复 e2e 平台，见 deferred-work.md）。

**Never:** 不引入 EngineEvents/Handle 注入（15.2/15.3 范围）；不迁移 event_router（15.2 AC 点名归 15.2）；memory_pipeline、suggestion_generator、task_decomposition、notification_service、energy_calculator 五个 service 因传递依赖 agent_engine/task_classifier 延至 15.2/15.3——AC「全部无 tauri/keyring 依赖的 services」按依赖传递闭包解读（2026-09-17 用户裁决）；不动 companion_* 四件套、commands/ 层、lib.rs 宿主逻辑；不建 server binary；不借迁移顺手重构。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 新库初始化 | 全新环境 `npm run tauri dev` | 双池正常创建，31 个迁移全部应用 | 迁移失败照旧显式报错 |
| 既有 dev 库升级 | 迁移前已存在的 egosync.db | 启动后 `_sqlx_migrations` 行数不变、无 dirty、checksum 一致、迁移不重跑 | 不一致即收口失败 |
| 密钥读写经接缝 | create/update/delete/test LLM 配置 | 行为与现状一致：api_key_ref=`llm_{uuid}_api_key` 落库，明文只进 keyring | KeyringError 路径保持现状 |
| sidecar 路径注入 | 桌面壳传 resource_dir（dev 为 None） | 二进制解析顺序不变：resources→cmd shim→PATH | 解析失败照旧降级 |
| 数据销毁 | destroy_all_data | keyring 条目删除时序不变（DB 事务成功后） | 删除失败 best-effort 语义不变 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/` -- 平移源：`db/`(20 文件)、`models/`(21)、`llm/`(4)、`error.rs`、`migrations/`(31 SQL)、services 12 件：agent_bridge、agent_config、butler_config、dashboard_service、llm_config、mcp_server、memory_query、role_config、skill_registry、sidecar、data_export、secret_store(仅改写为 trait)。内部 `crate::db/models/llm/error/services::` 引用在 engine 内同构成立，平移零 import 调整。
- `services/sidecar.rs` -- 已无 tauri 依赖（resource_dir 为 `Option<PathBuf>` 入参，`new()` :237；仅注释提及 tauri）。⚠ :1183 测试 `test_nsis_preinstall_hook_configured_and_path_based` 经 `CARGO_MANIFEST_DIR` 读 tauri.conf.json（NSIS 安装钩子断言，桌面专属），必须迁至壳 `tests/packaging_config.rs`；:993 等自引用 `src/services/sidecar.rs` 的测试随文件平移后路径依然成立。
- `services/secret_store.rs` -- keyring 唯一直接引用点（SERVICE_NAME="com.egosync.app" :3；save/load/delete :33/:43/:53，同步自由函数，key 校验 :6）。留守不动。
- `services/llm_config.rs` -- 密钥消费点：create :122 save、update :177、delete :203、test_connection :218、sync_default_to_opencode :262、list_models :359。全部改经 trait。
- `services/data_export.rs` -- 无 tauri/rfd；仅 `destroy_all_data` :1432 经 secret_store 删密钥，签名加接缝参数。
- 留守 services（20）-- agent_engine、delegate_bridge、scheduler、review/briefing/q2/bigrock×2、mission_inferrer、task_classifier、task_deadline/protection_watch、companion×4、memory_pipeline、suggestion_generator、task_decomposition、notification_service、energy_calculator。
- `src/lib.rs` -- :11-16 模块声明区改为回引：`pub use egosync_engine::{db, error, models};` + 私有 `use egosync_engine::llm;`（llm 可见性语义保持）；:182 `sync_default_to_opencode` 调用点、:198 sidecar 构造、:248-252 manage 区为接缝注入位。
- `src/services/mod.rs` -- 删 12 个 `pub mod`，加 `pub use egosync_engine::services::{…12 项}`；留守模块声明保留。
- `src/commands/llm_config.rs` -- 6 个涉密 command 加 `State<'_, KeyringSecretStore>` 参数并传 `&dyn SecretStore`；refresh_runtime :13 线程化传递。
- `crates/companion-proto/Cargo.toml` -- crate 形态先例（name/lib name/path 依赖/自带 Cargo.lock，无 workspace）。
- `.github/workflows/ci.yml` :224-236 -- companion-proto 独立 job 为 engine CI job 模板。
- `egosync-app/package.json` :12 -- `test:all` 现为 `vitest run && cd src-tauri && cargo test`。

## Tasks & Acceptance

**Execution:**
- [x] `crates/egosync-engine/` -- 新建 crate（Cargo.toml：name=egosync-engine，lib name=egosync_engine，依赖与 src-tauri 同版本，物理无 tauri/keyring；lib.rs 声明 db/models/llm/error/services）-- 引擎物理形态
- [x] `crates/egosync-engine/migrations/` + `src/{db,models,llm}/` + `src/error.rs` -- 自 src-tauri 平移（pool.rs 的 `migrate!("./migrations")` :12 与 `include_str!("../../migrations/…")` :132 相对路径在新 crate 根同构成立）-- 纯逻辑归位
- [x] `crates/egosync-engine/src/services/` -- 平移 12 件（见 Code Map；逐字节等价，仅接缝处改动）-- 泛域纯逻辑归位
- [x] `crates/egosync-engine/src/services/secret_store.rs` -- 新写 `pub trait SecretStore: Send + Sync`（save/load/delete(&self, key, value)→与现自由函数同签名同错误语义）-- 接缝一
- [x] `src-tauri/src/services/secret_store_keyring.rs` -- 新写 `KeyringSecretStore` 实现 trait（委托既有自由函数）；lib.rs `app.manage(KeyringSecretStore)` 注入 -- 接缝一桌面侧
- [x] `crates/egosync-engine/src/services/{llm_config,data_export}.rs` -- 涉密函数加 `&dyn SecretStore` 参数并替换调用（接缝调用替换豁免）；commands/llm_config.rs、commands/data.rs、lib.rs 调用点传参 -- 接缝贯通
- [x] `src-tauri/tests/packaging_config.rs` -- 承接 sidecar 的 NSIS 配置测试（原 :1182 起，读壳 tauri.conf.json）-- 测试归属桌面壳
- [x] `egosync-app/src-tauri/{Cargo.toml,src/lib.rs,src/services/mod.rs}` -- path 依赖 + `pub use` 回引 + 删除已迁声明；llm 由私有 `use` 保持 `crate::llm` 语义 -- 调用方零改动
- [x] `.github/workflows/ci.yml` -- 仿 companion-proto job 增 engine job：cargo test + grep 断言 Cargo.toml 无 tauri/keyring -- 物理封禁兜底
- [x] `egosync-app/package.json` -- `test:all` 追加 engine crate 的 cargo test -- 收口一体化
- [x] 既有 dev 库验证 -- 迁移前后 `_sqlx_migrations` 快照比对（行数/dirty/checksum）-- 迁移一致性

**Acceptance Criteria:**
- Given 仓库根，when 查看 `crates/egosync-engine/`，then Cargo.toml 不声明 tauri/keyring 且 CI 断言通过；src-tauri 以 `version+path` 双写引用；仓库根无 workspace
- Given src-tauri 全部 `crate::` 引用，when `npm run tauri dev`，then 正常启动、commands 与留守模块零改动
- Given 既有 dev 库，when 启动，then `_sqlx_migrations` 无 dirty、已应用迁移不重跑（checksum 一致）
- Given engine 内联测试随文件迁移，when engine cargo test，then 全部通过
- Given 桌面全量回归，when `npm run test:all`（EXIT=0 实测）+ tests/e2e（2026-09-17 用户裁决：环境级阻断证据豁免，基线对照见 Implementation Notes），then 全绿/豁免成立

## Implementation Notes

- **平移规模核对**（git rename 检测，评审补丁后）：113 个文件——83 个 R100 纯重命名（含 31 件 migrations，评审后为真移动：旧 `src-tauri/migrations/` 已删除，单一事实源落位 engine）；9 新增；16 修改；4 个受控重命名（R087/R095/R097×2/R098 = settings/tasks/sidecar/llm_config/data_export 五件中的带差异者）。修正口径：services 实际 11 件平移（3 件带受控差异）+ secret_store.rs 为新写 trait + 壳侧 secret_store_keyring.rs 为新写实现，spec 任务行「平移 12 件」的口径以此为准。
- **评审补丁（2026-09-17，六组，详见 Review Triage Log）**：①迁移真移动（删旧目录 + 18 处 include_str 路径重接，其中 7 处为多行形态）；②CI 断言强化（TOML 节式 + Cargo.lock 传递闭包双断言）+ engine job 加 rust-cache；③llm_config 接缝行为测试（create→`llm_{id}_api_key` 落库形状 + delete 同步清除）；④destroy 测试预置密钥并断言删空；⑤sprint-status 时间戳格式；⑥engine rust-version=1.78（lock v4 门槛）。补丁后全量重跑：`npm run test:all` EXIT=0（vitest 440 + 壳 440 + engine 444，444 含新接缝测试）。
- **规划期遗漏的依赖被实现期捕获**：models/companion_command.rs 使用 companion_proto 常量（规划期 grep 仅扫 services 未扫 models），engine 因此声明 companion-proto path 依赖（纯 crate，无 tauri/keyring，不破坏物理封禁）。
- **迁移一致性直接验证**：基线二进制（9979f3e，`egosync_lib::db::pool`）建库 → 故事二进制（`egosync_engine::db::pool`）升级同一库：31 迁移、无 dirty、逐行 checksum 一致、零重跑。
- **启动冒烟**：release 二进制 xvfb 启动成功，日志确认引擎生产路径工作（sidecar 解析顺序 resources→PATH 保持、delegate bridge、scheduler、keyring 不可用时按既有语义降级）——此冒烟同时是 State 注入装配成功的执行级证据（应用未 panic、bridge 正常监听）。
- **⚠ e2e 验证被先于本故事的环境级缺陷阻断（非 15.1 回归，证据链完整，2026-09-17 用户裁决：证据豁免，另立工作项）**：
  1. **基线对照实验**：基线（9979f3e，无任何 15.1 改动）release 构建在同一 spec（role-crud）同一位置以完全相同错误失败——`IPC app_complete_onboarding failed: Origin header is not a valid URL`（tauri 2.11.2 `ipc/protocol.rs:495` 的 Origin 校验拒绝）。
  2. **诊断**：WebDriver 自动化上下文落在 `about:blank`（origin "null"，页面文本 "Could not connect to localhost: Connection refused"），而非应用真实页面——WebKitGTK 2.52.6（本机）× tauri-driver 2.0.6 × wry 0.55.1 的自动化握手错位。
  3. **历史佐证**：2026-06-29 b0590e1「ci: 暂时跳过所有 E2E 阶段（tauri-driver 平台兼容性问题）」——e2e 层自落地次日起就在 CI 被官方禁用至今。
  4. **附带发现（仅记录，未修）**：e2e beforeSession 在 Linux 用 `pkill -f egosync` 自杀（命令行子串命中 wdio 自身路径）；beforeSession 擦的 DB 目录（~/.config/com.egosync.app）与应用实际数据目录（~/.local/share/com.egosync.desktop）不符，DB 隔离从未生效。
- **⚠ 范围外一行修复（披露）**：`egosync-app/tests/e2e/wdio.conf.ts` 两处 `pkill -f egosync` → `pkill -x egosync`（精确进程名，对齐 Windows 分支 `/im` 语义）。这是让 e2e 可在本机「执行」的最小修复——修复前本地运行在 beforeSession 秒级自杀。该修复未改变任何断言语义。修复后 e2e 达到「会话建立、应用启动、测试执行」，随后止于上述环境级 Origin 缺陷（与基线一致）。
- **验证局限披露**：本机无 keyring/dbus 服务，KeyringSecretStore 的 roundtrip 断言走了「跳过」分支（ValidationError 语义断言已执行）；`npm run tauri dev` 完整形态（vite dev server 交互）未跑，以 release 二进制启动冒烟 + 440 vitest + cargo test 覆盖等价面。

## Review Triage Log

三路评审（盲扫 12 项 / 边缘 4 项 / 验证缺口 4+2 项，2026-09-17，diff=/tmp/diff-15-1-review.diff，5296 行）。裁决均经本人复核（关键指控以 diff -rq / grep / python 解锁比对直接验证）：

| # | 来源 | 位置 | 裁决 | 证据 | 路由 |
|---|---|---|---|---|---|
| 1 | 盲扫 | src-tauri/migrations/ 双源 | high | diff -rq 两目录逐字节相同且旧目录 31 件留存；引擎 pool 只读新份 | G1→patch |
| 2 | 边缘 | 同上（漂移触发） | high | 同 1：只改一份即静默分叉 | G1→patch |
| 3 | 验缺 | 同上（18 夹具读旧目录、无一致性钉子） | high | 18 处 include_str 实测（chat 3/agent_engine 10/memory_pipeline 5） | G1→patch |
| 4 | 验缺-其他 | 文档未披露复制非移动 | high | 同 1；Implementation Notes 计数口径一并修正 | G1→patch |
| 5 | 盲扫 | ci.yml grep 可绕过+证据源弱 | medium | 评审者以节式声明实测绕过；Cargo.lock（303 包零 tauri/keyring）未被断言 | G2→patch |
| 6 | 边缘 | TOML 节式绕过 | medium | 同 5 | G2→patch |
| 7 | 验缺 | grep 对节式失明（rc=2 亦放行） | medium | 同 5 | G2→patch |
| 8 | 盲扫 | llm_config 六接缝函数零行为测试 | medium | grep 实测 crate 内测试仅纯函数；接缝误接线全绿 | G3→patch |
| 9 | 盲扫 | destroy 密钥删除无护栏 | medium | 读测试实况：InMemorySecretStore 注入但零断言 | G4→patch |
| 10 | 验缺 | destroy 假库从未被断言清空 | medium | 同 9；IO 矩阵「删除时序不变」行无守护 | G4→patch |
| 11 | 盲扫 | keyring roundtrip 全自动化环境静默跳过 | medium | 本机/CI 均无 dbus+keyring；eprintln 在通过测试中不可见 | defer |
| 12 | 盲扫 | 双 lock 解析版本分歧 | medium | 实测 131/272 共享包分歧；无 workspace 下无对齐机制（companion-proto 同属性） | defer |
| 13 | 盲扫 | engine CI job 无 rust-cache | low | 属实；直接更正（3 行，对齐主 job 惯例） | G2→patch |
| 14 | 盲扫 | AGENTS.md/project-context 未反映分层 | medium | 属实；修复须改 agent-context 文件 | defer |
| 15 | 盲扫 | sprint-status +0800 格式回归 | low | 属实，本故事引入 | G5→patch |
| 16 | 盲扫 | 测试手抄 schema DDL 第三份 | medium | 壳侧 setup_protection_test_db 实况；共享 migrate! 助手属测试基建重构，沿用既有模式 | defer |
| 17 | 盲扫 | AppError::KeyringError 烧进引擎契约 | medium | error.rs 逐字节平移的既定后果；改名违反冻结块，15.2/15.4 错误形状冻结时决策 | defer |
| 18 | 盲扫 | 故事文档计数口径 12 vs 11+1 | — | 修正已存 Implementation Notes（准确口径），fix=改 spec → 按规则拒绝 | reject |
| 19 | 边缘 | e2e 会话孤儿 opencode sidecar | maybe-false | 本机无 opencode 二进制（启动日志降级）不可达；完整环境成立，先于本故事 | defer |
| 20 | 边缘 | engine 无 rust-version 且 lock v4 需 cargo≥1.78 | low | 属实（src-tauri 1.77.2 / engine lock v4）；一行直接更正 | G6→patch |
| 21 | 验缺 | 桌面 State 注入链路无执行级验证 | medium | 预验证成立：manage↔state↔7 命令三方无编译检查；mock-app 基建非平凡，filed disposition 明示可 defer；现有缓解=release 启动冒烟实证装配成功 | defer |
| 22 | 验缺-其他 | pkill -x 与 productName 大小写未来漂移 | low | 推测性漂移（今日二进制名=egosync 正确匹配），无日常损害 | reject |

分组与路由：G1 迁移双源（#1-4，high→patch：删除旧目录+18 处路径重接——路径调整属冻结块显式允许的改动类）；G2 CI 断言与缓存（#5-7,13，patch）；G3 llm_config 接缝测试（#8，patch）；G4 destroy 斿令断言（#9-10，patch）；G5 时间戳格式（#15，patch）；G6 rust-version（#20，patch）；#11/12/14/16/17/19/21 → defer（各自独立根因）。无 intent_gap / bad_spec：G1 属实现未完成「平移」（规范已明确要求移动语义），补齐即合规，无需回环。

## Spec Change Log

## Review Triage Log

## Design Notes

- **依赖闭包实测**（grep 全量 + 逐文件核实）：迁移集合对外零反向依赖（→留守 services、→commands 均为空）；留守模块对迁移模块的 22 条引用经 `pub use` 回引全部消解。event_router 虽物理纯净但 15.2 AC 点名迁移，故不入本故事。
- **SecretStore 形态**：同步方法（keyring v3 本身同步，现调用方亦同步；commands/secret.rs 的 spawn_blocking 包装留在壳侧不动）；`&dyn` 参数注入（State 直取引用自动协变），不引入泛型与全局注册表。
- **engine 依赖清单**（按迁移集 grep 实测）：serde/serde_json/tokio/tokio-util/tracing/sqlx/thiserror/reqwest/futures/chrono/uuid/sha2/async-trait/sysinfo + dev-deps tempfile/filetime；不需要 dirs/indexmap/base64（均为留守侧使用）。
- **`llm` 可见性**：src-tauri 侧私有 `mod llm` → engine 侧须 `pub mod llm`（跨 crate 引用要求），可见性小幅放宽是 crate 拆分的机械后果，记录在案。

## Verification

**Commands:**
- `cd egosync-app && npm run build` -- tsc 零类型错误（前端无改动，守门）
- `cd egosync-app && npm run test:all` -- vitest + src-tauri cargo test + engine cargo test 全绿
- `cd egosync-app/tests/e2e && npm test` -- 环境级阻断（2026-09-17 用户裁决豁免：基线对照实验同败，证据链见 Implementation Notes；平台修复另立工作项）
- `cd crates/egosync-engine && cargo test` -- engine 内联测试全绿
- `grep -E '^(tauri|keyring)' crates/egosync-engine/Cargo.toml` -- 无输出（物理封禁）

**Manual checks (if no CLI):**
- 迁移前后各执行一次 `sqlite3 <app_data>/egosync.db "SELECT version,checksum FROM _sqlx_migrations ORDER BY version"` 比对一致，且启动日志无迁移重跑记录
