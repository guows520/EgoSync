---
baseline_commit: 4b69704ed6b57b781ec34d4a5499567366f3d6e8
---

# Story 8.7: 模型网络位置与代理绕过

Status: done

<!-- Ultimate context engine analysis completed - comprehensive developer guide created -->

## Story

As a 在公司内网部署 EgoSync 的用户,
I want 为每个大模型配置明确选择“内网”或“外网”,
so that 外网模型继续通过公司代理访问，而内网模型直接连接且不会被代理或公司网络策略拦截。

## Acceptance Criteria

1. 模型配置新增必填枚举 `networkLocation`，仅允许 `internal`、`external`；设置界面显示“模型网络位置”的“内网/外网”选项，新建配置默认“外网”。
2. 数据库新增非空 `network_location` 列，历史记录迁移为 `external`，升级后现有模型代理行为不变；Rust/TS 类型、DB CRUD、Tauri IPC、新建与编辑表单端到端一致使用 camelCase/snake_case 既有约定。
3. 所有 Rust 直接模型 HTTP 通道统一按该字段构建 client：`external` 保持当前代理行为，`internal` 显式使用 `reqwest::ClientBuilder::no_proxy()`；不得仅修复连接测试或主聊天路径。
4. “获取模型列表”无论针对已保存配置还是未保存表单，都携带并应用网络位置；内网配置获取模型列表不得经过代理。
5. opencode sidecar 启动前，根据当前所有 `internal` 模型配置的有效 Base URL 生成 host 级 bypass 集合，并生成动态 `NO_PROXY`；集合始终包含 `localhost`、`127.0.0.1`，并保留 EgoSync 进程启动时已有的 bypass 项。
6. 动态 `NO_PROXY` 每次从“进程原始 bypass + 固定 localhost + 当前 DB 内网 host”重新构建；删除或改为外网的 host 必须被移除，不得因合并 sidecar 当前环境而残留陈旧条目；输出去重且顺序稳定，便于比较与测试。
7. 模型配置创建、更新、删除后，仅当计算出的 sidecar bypass 集合发生变化时更新环境并受控重启 sidecar；仅修改名称、API Key、模型名等不影响集合的字段不得重启。应用启动时首次计算必须发生在 sidecar 启动之前。
8. 因 `NO_PROXY` 是 host/domain 级路由，同一规范化 host 不允许同时存在 `internal` 与 `external` 配置；保存时返回明确校验错误。不得按模型名、scheme 或端口把同一 host 的冲突静默拆开。
9. 若数据库已保存成功但 sidecar 环境刷新或重启失败，不回滚已提交配置；后端返回“配置已保存但运行时刷新失败”的显式错误，前端提示用户重试或重启应用，不得宣称保存失败或静默成功。
10. 自动化测试覆盖：旧库迁移、枚举校验、DB CRUD、host 规范化与冲突、稳定 bypass 生成、无关编辑不重启、集合变化触发重启、Rust internal/external client 策略、模型列表参数传播，以及设置表单新建/编辑回显。
11. 不改变 EgoSync → localhost opencode 的 `AgentBridge` 现有 `.no_proxy()` 行为，不升级 reqwest/opencode，不引入新的代理库或本地转发层，不影响无系统代理环境下的现有模型调用。

## Tasks / Subtasks

- [ ] Task 1：完成配置模型与数据库迁移（AC: 1, 2, 8）
  - [ ] 新增 migration，为 `llm_configs` 添加 `network_location TEXT NOT NULL DEFAULT 'external'` 及合法值约束；遵循现有 migration 顺序，不修改历史 migration。
  - [ ] 在 Rust 中定义受限枚举（serde camelCase/小写字符串），加入 `LlmConfig`、Create/Update input；非法值显式失败。
  - [ ] 更新 `db/settings.rs` 的全部 SELECT/INSERT/UPDATE 和测试，避免任何查询漏列。
  - [ ] 使用 `reqwest::Url` 解析 Base URL 并提取规范化 host；拒绝无法提取 host 的 internal 配置。
  - [ ] 保存前校验同一 host 的内外网冲突；规则以 host 为边界，不以 scheme/port 拆分。

- [ ] Task 2：贯通前端类型、服务和设置 UX（AC: 1, 2, 4, 9）
  - [ ] 更新 `src/types/settings.ts` 的查询、创建、更新类型。
  - [ ] 在 `GlobalSettingsModal` 新建/编辑状态中加入网络位置，编辑时正确回显，新建默认 external。
  - [ ] 在 API 地址附近增加“模型网络位置”选择控件，沿用现有 Tailwind、暗色模式和表单样式，不新增自定义 CSS。
  - [ ] `listModelsByParams` 增加 networkLocation 参数并从表单传入。
  - [ ] 对“已保存但运行时刷新失败”显示准确提示；不得清空用户已保存的配置状态。
  - [ ] 更新现有 `GlobalSettingsModal.test.tsx`，验证默认值、编辑回显、保存 payload、获取模型列表 payload 和部分失败提示。

- [ ] Task 3：统一 Rust 直接请求的代理策略（AC: 3, 4, 10, 11）
  - [ ] 在 `llm/` 内建立最小共享 HTTP client 构造函数或等价单一入口，保留现有 timeout；internal 调 `.no_proxy()`，external 不调用。
  - [ ] 让 `OpenAiProvider`、`AnthropicProvider` 接收网络位置并复用该入口；MiniMax reasoning split 行为保持不变。
  - [ ] 更新 `agent_engine.rs`、`mission_inferrer.rs`、`task_classifier.rs`、连接测试等所有 provider factory 调用点。
  - [ ] 更新 `fetch_models_by_params`、保存配置的 `list_models` 和未保存表单的 `list_models_by_params`。
  - [ ] 使用可测试的 client 策略/构造边界验证 internal 与 external 分支；不要通过真实公网调用完成单元测试。

- [ ] Task 4：生成并管理 sidecar 动态 bypass（AC: 5, 6, 7, 8, 11）
  - [ ] 在 service 层实现纯函数：读取配置、提取 internal host、合并进程原始 bypass 与 localhost、去重、稳定排序并输出字符串。
  - [ ] 不从 SidecarManager 当前 `NO_PROXY` 反向合并，防止已删除 host 永久残留。
  - [ ] 扩展 `SidecarManager` 的最小环境更新接口，避免外部直接操作私有 `extra_env`；保持现有 builder `with_env` 行为。
  - [ ] `lib.rs` 在 sidecar `start()` 前注入首次计算结果。
  - [ ] 配置变更后在既有 `Arc<Mutex<SidecarManager>>` 生命周期边界内比较旧/新值；仅变化时调用现有 `restart()`。
  - [ ] 不创建第二套 sidecar manager、watchdog 或进程生命周期机制。

- [ ] Task 5：定义配置持久化与运行态刷新顺序（AC: 7, 9）
  - [ ] Command 继续只做参数解析与 service orchestration，不直接 SQL 或拼接环境变量。
  - [ ] 顺序明确为：校验 → DB/Keyring 持久化 → `opencode.json` 投影 → 计算 bypass → 必要时更新环境并重启。
  - [ ] 复用项目现有“配置已保存但 Runtime 刷新失败”的错误语义；不得伪装成事务回滚。
  - [ ] 删除默认配置或无默认配置时维持现有业务规则，同时确保 bypass 仍由当前 DB 的 internal 配置集合正确重建。

- [ ] Task 6：测试与回归验证（AC: 1-11）
  - [ ] Rust 单元/集成测试覆盖 migration、枚举、DB、URL/host、冲突、bypass 纯函数、SidecarManager 环境更新及 restart 判定。
  - [ ] React/Vitest 覆盖 UI 与 IPC payload，不使用脆弱的纯文本快照替代行为断言。
  - [ ] 增加可观测 mock proxy/mock endpoint 测试或等价隔离测试，证明 external 经代理、internal 直连；若 CI 环境不适合进程级网络测试，记录并执行明确的手工验证步骤，不得宣称其自动通过。
  - [ ] 运行 `cargo test`、`cargo clippy -- -D warnings`、前端相关 Vitest、TypeScript build；任何跳过项必须在 Dev Agent Record 中显式记录。


### Review Findings

- [x] [Review][Patch] 流式模型请求必须复用 `networkLocation` 感知的 HTTP client，禁止 internal 流式请求经过系统代理 [`egosync-app/src-tauri/src/llm/openai.rs`:136]
- [x] [Review][Patch] 配置创建、更新、删除后重算 bypass，仅集合变化时更新环境并受控重启 sidecar [`egosync-app/src-tauri/src/commands/llm_config.rs`:15]
- [x] [Review][Patch] 动态 `NO_PROXY` 必须保留进程原始 bypass、去重并稳定排序 [`egosync-app/src-tauri/src/services/llm_config.rs`:64]
- [x] [Review][Patch] `NO_PROXY` 构建不得吞掉 DB 错误并以缺失 internal host 的环境启动 sidecar [`egosync-app/src-tauri/src/services/llm_config.rs`:69]
- [x] [Review][Patch] 实现“DB 已保存但运行态刷新失败”的部分成功错误与前端准确提示 [`egosync-app/src-tauri/src/commands/llm_config.rs`:20]
- [x] [Review][Patch] 仅 internal 配置强制要求可解析 host，保持 external 空 Base URL 的既有兼容性 [`egosync-app/src-tauri/src/services/llm_config.rs`:36]
- [x] [Review][Patch] `LlmConfig.networkLocation` 在 Rust/TS 查询模型中使用受限枚举而非普通字符串 [`egosync-app/src-tauri/src/models/settings.rs`:40]
- [x] [Review][Patch] 以事务、串行化或数据库约束消除同 host 内外网冲突检查竞态 [`egosync-app/src-tauri/src/services/llm_config.rs`:30]
- [x] [Review][Patch] 补齐 AC10 要求的迁移、代理策略、bypass、重启判定、部分失败与 UI payload 自动化测试 [`egosync-app/src/components/settings/GlobalSettingsModal.test.tsx`:1]
- [x] [Review][Defer] NSIS `installerHooks` 引用文件未纳入当前变更集 [`egosync-app/src-tauri/tauri.conf.json`:56] — deferred, pre-existing
- [x] [Review][Defer] 非 Windows 平台的陈旧 sidecar 端口清理仍调用 Windows 命令 [`egosync-app/src-tauri/src/services/sidecar.rs`:749] — deferred, pre-existing
- [x] [Review][Defer] Windows Job Object/stop 生命周期存在子进程逃逸、句柄丢失与关闭缺口 [`egosync-app/src-tauri/src/services/sidecar.rs`:531] — deferred, pre-existing
- [x] [Review][Defer] 聊天执行事件依赖正文关联历史消息，thinking-only 与多 bubble 归属不可靠 [`egosync-app/src/components/chat/ChatStream.tsx`:970] — deferred, pre-existing
- [x] [Review][Defer] Thinking 分段边界、替换及失败路径持久化不完整 [`egosync-app/src-tauri/src/services/agent_engine.rs`:2849] — deferred, pre-existing
- [x] [Review][Defer] Thinking 插入连续读取事件时会打乱执行轨迹顺序 [`egosync-app/src/components/chat/ChatStream.tsx`:479] — deferred, pre-existing
## Dev Notes

### 已确认的根因与范围

- 模型配置当前没有网络位置字段；问题不是单一 UI 缺口，而是数据库、IPC、Rust 直连 client 和 opencode sidecar 环境的端到端缺口。
- 应用存在两条网络通道：Rust 直接请求模型，以及 opencode sidecar/provider 请求模型。只修改其中一条不满足 Story。
- 用户已在线下确认当前 bundled opencode 支持动态 `NO_PROXY` 绕过代理；本 Story 不再研究替代转发层。
- `AgentBridge` 访问 localhost 已显式 `.no_proxy()`，该逻辑保持不变。

### 强制技术决策

- 网络位置枚举：`internal | external`；历史值和新建默认值均为 `external`。
- sidecar bypass 以 host/domain 为最小单位。之前调查中的 `scheme + host + port` 冲突粒度已被本 Story 明确废弃，因为跨 runtime 的 `NO_PROXY` 不能可靠表达同 host 不同端口的相反路由。
- Base URL 使用已存在依赖可访问的 `reqwest::Url` 解析，禁止手写字符串切割；不要仅为此新增 `url` 直接依赖。
- 项目锁定 reqwest `0.12.28`；使用现有 `ClientBuilder::no_proxy()`，本 Story 不升级依赖。
- 数据库是事实来源，`opencode.json` 是模型/provider 投影，sidecar 环境是运行态；运行态失败不能伪装成持久化失败。
- 不使用模型做路由、重试或确定性 host 转换；全部由普通代码完成。

### 相关源码与预期修改位置

- `egosync-app/src-tauri/migrations/`：新增 migration。
- `egosync-app/src-tauri/src/models/settings.rs`：配置和输入枚举/字段。
- `egosync-app/src-tauri/src/db/settings.rs`：CRUD、查询和 DB 测试。
- `egosync-app/src-tauri/src/services/llm_config.rs`：校验、模型列表、opencode 同步、运行态刷新 orchestration。
- `egosync-app/src-tauri/src/commands/llm_config.rs`：IPC 参数和 Sidecar state 注入；保持薄 Command。
- `egosync-app/src-tauri/src/llm/openai.rs`、`anthropic.rs`：统一 client 策略。
- `egosync-app/src-tauri/src/services/agent_engine.rs`、`mission_inferrer.rs`、`task_classifier.rs`：传播字段，不进行邻近重构。
- `egosync-app/src-tauri/src/services/sidecar.rs`：最小环境更新接口，复用现有 `restart()`。
- `egosync-app/src-tauri/src/lib.rs`：启动前首次注入。
- `egosync-app/src/types/settings.ts`、`src/services/llmConfigService.ts`、`src/components/settings/GlobalSettingsModal.tsx` 及其测试：前端贯通。

### 架构与编码约束

- React 组件不得直接 `invoke()`；继续通过 `llmConfigService`。
- Rust Command 不写 SQL、不实现网络路由；DB 层不操作 sidecar。
- serde 使用 `rename_all = "camelCase"`，数据库字段 snake_case，枚举序列化为小写字符串。
- Command/Service 返回 `Result<T, AppError>`，不得 `.unwrap()` 处理运行路径错误。
- 不修改 API Key 的 keyring 存储边界。
- 不顺手重构 `agent_engine.rs` 或 sidecar 生命周期；当前工作区这些区域已有其他未提交修改，实施前先检查 diff 并做外科手术式修改。

### Previous Story Intelligence

- Story 8.6 已建立并正在修改 opencode sidecar 升级/生命周期；8.7 必须复用现有 `SidecarManager::restart()`、`Arc<Mutex<SidecarManager>>` 和 watchdog 边界，不得另起进程管理范式。
- 现有 MCP 配置刷新已经体现“持久化成功但 runtime 刷新失败”的部分失败语义；实现时应复用该模式，而非吞错或回滚已提交 DB。
- `sync_default_to_opencode` 当前为 best-effort 并吞掉错误；开发者需要明确本 Story 的用户触发保存路径如何向前端暴露运行态失败，不能仅依赖启动日志。

### Git Intelligence Summary

- 最近提交集中于 sidecar 版本升级、聊天 UI 和活动统计；本 Story 与当前未提交的 `sidecar.rs`、`agent_engine.rs`、`lib.rs/App.tsx` 等改动存在潜在重叠。
- 禁止覆盖用户工作区改动；实施前必须逐文件查看 diff，只修改本 Story 必需行。

### Testing Requirements

- 测试必须说明 WHY：内网请求若进入代理会在企业网络失败；外网请求若绕过代理同样会失败。断言应验证路由选择，而不仅是字段存在。
- bypass builder 应优先作为纯函数测试，覆盖大小写 host、IPv4/IPv6、重复条目、空白、无效 URL、删除陈旧 host 和稳定顺序。
- sidecar restart 判定应通过可控边界验证，避免单元测试真实启动生产 sidecar。
- UI 测试验证发送给 IPC 的 `networkLocation`，否则仅渲染下拉框不能防止字段传播回归。

### Latest Technical Information

- 当前锁文件使用 reqwest `0.12.28`；其 `ClientBuilder::no_proxy()` 用于禁用系统代理自动发现，适合 internal Rust client。无需升级到 reqwest 0.13。
- Rust `std::process::Command::env` 只影响新启动子进程，因此运行中变更必须通过现有 sidecar restart 生效。
- `reqwest::Url` 提供标准 URL 解析与 host 提取；不要自行拆分 Base URL。

### Project Structure Notes

- Story 8.7 是 Epic 8 的增量 V1 企业网络加固 Story；`epics.md` 原始清单止于 8.5，但 sprint 中已有增量 Story 8.6，因此沿用 8.7，不新建额外 Epic。
- 未发现需要新增顶层目录或第三方依赖的理由。

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/model-network-location-investigation.md`]
- [Source: `_bmad-output/project-context.md` — Tauri/LLM/sidecar 数据边界与禁止事项]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — 配置事实、运行时投影和 Sidecar 生命周期边界]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md` — BYOK 与 base_url/api_key/model_name 模型配置]
- [Source: `_bmad-output/implementation-artifacts/8-6-opencode-sidecar-upgrade-lifecycle.md`]
- [Source: `egosync-app/src-tauri/src/models/settings.rs:5-35`]
- [Source: `egosync-app/src-tauri/src/services/llm_config.rs:97-295`]
- [Source: `egosync-app/src-tauri/src/services/sidecar.rs:220-278,450-466,560-672`]
- [Source: `egosync-app/src-tauri/src/llm/openai.rs:19-32`]
- [Source: `egosync-app/src-tauri/src/llm/anthropic.rs:18-30`]
- [Source: `egosync-app/src-tauri/src/services/agent_bridge.rs:18-24`]
- [Source: `egosync-app/src-tauri/src/lib.rs:172-209`]
- [Source: `egosync-app/src/components/settings/GlobalSettingsModal.tsx:221-303,629-656`]
- [Source: `egosync-app/src-tauri/Cargo.lock` — reqwest 0.12.28, url 2.5.8]
- [External: reqwest 0.12.28 `ClientBuilder::no_proxy` API documentation]
- [External: Rust standard library `std::process::Command::env` documentation]

## Dev Agent Record

### Agent Model Used

GPT-5.6 (Amelia / bmad-agent-dev)

### Debug Log References

- `cargo test services::llm_config::tests -- --nocapture` — 12 passed
- `cargo test test_update_env_reports_only_real_changes -- --nocapture` — 1 passed
- `cargo test error::tests -- --nocapture` — 2 passed
- `cargo test db::settings::tests -- --nocapture` — 10 passed
- `cargo test --no-run` — passed
- `cargo check` — passed（仅项目既有 warnings）
- `npx vitest run src/components/settings/GlobalSettingsModal.test.tsx` — 35 passed
- `npm run build` — passed（仅 chunk size warning）

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story 创建阶段未修改业务代码、数据库或依赖。
- Code Review 的 9 个 Patch 已全部完成并验证；6 个 Defer 保持原范围，未修改。
- 完成流式代理绕过、运行态刷新/条件重启、NO_PROXY 稳定合并与错误传播、部分成功提示、受限枚举、冲突串行化及 AC10 自动化测试。

### File List

- `_bmad-output/implementation-artifacts/8-7-model-network-location-proxy-bypass.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `egosync-app/src-tauri/migrations/029_llm_config_network_location.sql`
- `egosync-app/src-tauri/src/commands/llm_config.rs`
- `egosync-app/src-tauri/src/db/settings.rs`
- `egosync-app/src-tauri/src/error.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src-tauri/src/llm/anthropic.rs`
- `egosync-app/src-tauri/src/llm/openai.rs`
- `egosync-app/src-tauri/src/models/settings.rs`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/src-tauri/src/services/llm_config.rs`
- `egosync-app/src-tauri/src/services/mission_inferrer.rs`
- `egosync-app/src-tauri/src/services/sidecar.rs`
- `egosync-app/src-tauri/src/services/task_classifier.rs`
- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`
- `egosync-app/src/services/llmConfigService.ts`
- `egosync-app/src/types/settings.ts`
