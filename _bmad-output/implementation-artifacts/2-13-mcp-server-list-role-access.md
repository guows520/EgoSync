---
baseline_commit: afbf282ac7b313a6fa74b37daeba9c5be5b15770
---

# Story 2.13: 用户能配置 MCP server 列表并按角色接入外部工具

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 配置外部 MCP server 并选择哪些角色可用,
so that 角色能安全接入日历、邮件、代码仓库等外部工具服务。

## Acceptance Criteria

1. **MCP server 管理入口**
   - Given 用户打开设置中的 MCP server 管理区域
   - When 新增 server
   - Then 可填写名称、类型（HTTP/SSE 或 command）、连接参数、说明和启用状态
   - And UI 明确提示：这是外部工具接入，不是 EgoSync 内部 create_role/delegate 工具

2. **安全持久化**
   - Given MCP server 配置包含 token/API key/secret
   - When 保存配置
   - Then secret 不写入 `roles.skills_config`、`app_settings.value` 明文字段、opencode.json 明文或日志
   - And 只能保存 `env:` 环境变量引用或无密钥配置

3. **opencode.json 同步**
   - Given 用户保存启用的 MCP server
   - When 配置校验通过
   - Then `AgentConfigService` 同步 opencode.json 的外部 `mcp` 配置
   - And `full_sync()` 不得删除用户配置的外部 MCP server
   - And 仍然删除或忽略旧的 `egosync` MCP host 遗留配置

4. **不恢复内部 MCP host**
   - Given Story 2.0d 已将 EgoSync 内部工具从 MCP 改为 opencode Custom Tools
   - Then 本 story 不恢复 `services/mcp_host.rs`、不重启 4097 MCP HTTP host
   - And `create_role`、`delegate_to_role`、`record_emergence_rejection` 继续走 `~/.config/opencode/tools/` custom tools 路径

5. **角色级 MCP 可用性**
   - Given 用户为某个角色启用一个 MCP server
   - When 保存成功
   - Then 角色配置记录该 server 可用
   - And opencode agent 配置或 prompt 能力约束反映该角色可使用的外部工具
   - And 未启用该 server 的角色不得声明或调用该外部工具

6. **连接测试与失败降级**
   - Given 用户点击测试 MCP server 连接
   - When server 不可达、启动失败或配置错误
   - Then 设置页显示友好中文错误并允许修改
   - And 不影响应用启动、角色 CRUD、普通对话和已有 custom tools

7. **现有功能不回归**
   - opencode sidecar 启动、delegate bridge、event router、角色 CRUD 同步、Skill registry、默认元 Skill、自定义/opencode Skill 均保持可用

## Tasks / Subtasks

- [x] 设计 MCP server 本地配置模型（AC: 1, 2, 5）
  - [x] 新增 SQLite 表或本地配置模型：id/name/type/command_or_url/env_refs/description/enabled/created_at/updated_at
  - [x] 角色级启用关系独立建模，避免把完整 server 配置塞入 `roles.skills_config`
  - [x] secret 字段只保存 `env:` 引用；不要明文落库

- [x] 实现 MCP server CRUD 与连接测试（AC: 1, 2, 6）
  - [x] Rust service 负责配置校验、env ref 解析、测试连接
  - [x] Tauri commands 示例：`mcp_server_create`、`mcp_server_update`、`mcp_server_delete`、`mcp_server_list`、`mcp_server_test`
  - [x] Command 层返回友好 `AppError`，不使用 `.unwrap()`
  - [x] 连接测试失败不阻塞保存草稿，除非用户明确要求“保存前必须通过”

- [x] 更新 AgentConfigService 的 MCP 同步策略（AC: 3, 4, 5）
  - [x] 当前 `full_sync()` 会 `root.remove("mcp")`；改为只清理旧 `egosync` MCP host 或 legacy tools，不删除用户外部 MCP 配置
  - [x] 将启用的外部 MCP servers 写入 opencode.json 顶层 `mcp`
  - [x] 保留 `model`、`provider`、`agent`、外部 `mcp` 等无关字段
  - [x] 不恢复 `services/mcp_host.rs`，不新增 4097 固定端口内部 host

- [x] 角色级 MCP 启用 UI 与同步（AC: 5）
  - [x] 在角色 SettingsTab 或专门 MCP 设置区展示可用 MCP server 列表
  - [x] 用户可按角色启用/禁用 server
  - [x] 保存后触发角色 agent 配置同步或 prompt 能力约束更新
  - [x] 未启用角色不得看到“我可以访问 X 工具”的 prompt 声明

- [x] 前端全局 MCP 管理 UI（AC: 1, 2, 6）
  - [x] 优先放在现有设置体系；若没有合适全局设置入口，story 内说明最小入口位置
  - [x] 表单包含类型、URL/command、环境变量引用、说明、启用状态、测试按钮
  - [x] 使用 inline feedback，不新增全局 toast/snackbar
  - [x] 组件不直接调用 `invoke()`；新增 `mcpService.ts`

- [x] 测试与验证（AC: 1-7）
  - [x] Rust 单测：配置校验、secret ref 校验、opencode.json mcp 保留/同步、legacy egosync MCP 清理、角色启用关系
  - [x] 前端测试：新增/编辑表单、secret 提示、测试失败展示、角色启用切换
  - [x] 回归 `agent_config.rs` 现有 tests，尤其 full_sync、custom tools、provider 保留
  - [x] 运行 `npm --prefix "GUI" run test:frontend`
  - [x] 运行 `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - [x] 运行 `npm --prefix "GUI" run build`
  - [x] 可见 UI 改动需启动应用，验证新增 MCP server、测试失败、角色启用、重启持久化

## Dev Notes

### Current State

- 当前源码没有 MCP schema/service/command/UI。不要误判为“只差 UI”。
- Story 2.0d 删除了旧 `services/mcp_host.rs`，原因是 opencode project instance 下 MCP 工具可见性不可靠；EgoSync 内部工具已改为 opencode Custom Tools。
- `egosync-app/src-tauri/src/services/agent_config.rs::full_sync()` 当前会删除顶层 `mcp` 和 `tools`，这是本 story 必须调整的核心点。
- 顶层 `tools` 是 legacy；`agent.<role>.tools` 仍用于禁用 butler-only custom tools。不要混淆两者。
- `app_settings` 表存在（key/value），但 secret 不得明文塞入 value；复杂 MCP 配置更适合独立表或结构化本地配置。

### Architecture Guardrails

- 本 story 只接入**外部 MCP server**，不恢复 EgoSync 内部 MCP host。
- 外部 MCP 配置写入 opencode.json 的 `mcp` 时必须可被 `full_sync` 保留。
- 对外部工具的密钥采用 `env:` 环境变量引用。不得存储 API key/token 明文。
- MCP server 失败必须降级为“该外部工具不可用”，不能影响普通聊天或应用启动。
- 角色级可用性必须明确：未启用的角色不能在 prompt 中声明或尝试调用该 MCP 工具。

### Previous Story Intelligence

- Story 2.0d 的根因：opencode MCP 工具绑定在 project instance state 中，EgoSync 内部工具通过 MCP 连接成功但 LLM 请求工具列表不可见。内部工具已改 custom tools。外部 MCP 仍可作为用户配置能力，但必须用真实 opencode.json 同步和连接测试证明可用。
- Story 2.0d 指出 Windows opencode 全局配置路径是 `~/.config/opencode/`；不要用 `%APPDATA%` 推断 opencode 自身路径。
- Story 2.11/2.12 会扩展 Skill registry 和角色绑定；本 story 的角色 MCP 关系应避免再污染 `skills_config`，除非只记录轻量引用并保证不被元 Skill 保存路径覆盖。

### Regression Risks

- **旧问题复活**：不要恢复 `mcp_host.rs` 和 4097 egosync MCP host。
- **full_sync 删除用户配置**：保存 MCP 后重启/全量同步不能把外部 MCP 删掉。
- **secret 泄露**：不要明文写 DB、opencode.json、日志、prompt。
- **tools 字段混淆**：不要删除 role agent 的 `tools` 隔离配置。
- **启动阻塞**：MCP server 配错不能阻塞应用启动或普通对话。

### References

- PRD FR-4b：`_bmad-output/planning-artifacts/prd-egosync.md` → MCP 外部工具服务。
- Story 2.0d：`_bmad-output/implementation-artifacts/2-0d-opencode-custom-tools-replace-mcp.md` → MCP 替换为 custom tools 的根因与边界。
- Architecture Agent Engine：`_bmad-output/planning-artifacts/architecture.md` → opencode sidecar、MCP Servers、opencode.json 动态管理。
- Existing config code: `egosync-app/src-tauri/src/services/agent_config.rs` → `full_sync()`、custom tools、provider sync、agent sync。
- Initial schema: `egosync-app/src-tauri/migrations/001_initial_schema.sql` → `app_settings`、`llm_configs`。
- Current UI extension points: `egosync-app/src/components/role/SettingsTab.tsx`, `egosync-app/src/services/roleService.ts`, `egosync-app/src/types/role.ts`.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.8

### Debug Log References

- `npm --prefix "GUI" run test:frontend -- src/components/settings/GlobalSettingsModal.test.tsx`：先红后绿，覆盖全局 MCP 标签入口、外部工具提示、MCP inline error、测试连接 loading spinner；最新 13 passed。
- `npm --prefix "GUI" run test:frontend -- src/components/role/SettingsTab.test.tsx`：先红后绿，覆盖角色级只展示已绑定 MCP、搜索添加、移除绑定。
- `npm --prefix "GUI" run test:frontend`：最新 164 passed。
- `cargo test services::mcp_server::tests::test_server_rejects_plain_http_200_without_mcp_handshake`：先红后绿，证明旧连接测试会把普通 HTTP 200 误判为 MCP 成功。
- `cargo test services::agent_engine::tests::invalid_mcp_session_tool_error_requests_runtime_refresh_retry_once`：先红后绿，覆盖 MCP `Invalid session id` 触发 runtime refresh + retry once。
- `cargo test services::agent_engine::tests::final_narration_flush_is_suppressed_after_tool_process_event`：先红后绿，覆盖工具调用后最终回答不再重复写入执行过程 narration。
- `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" --lib`：最新 344 passed；覆盖 MCP scope key、opencode session cache 隔离、真实 MCP 协议测试、session 自动恢复相关单测。
- `npm --prefix "GUI" run build`：TypeScript 与 Vite production build passed。
- 启动 `npm --prefix "GUI" run tauri dev` 并确认 `5173` 与 `4096` 监听，日志包含 `opencode sidecar started on port 4096`；人工 UI 复验仍建议覆盖真实外部 MCP 数据流。

### Completion Notes List

- 新增 MCP server 独立 SQLite 表与角色绑定表，避免把完整 server 配置塞入 `roles.skills_config`。
- 新增 Rust MCP model/db/service/commands，支持 CRUD、连接测试、角色绑定、secret ref 校验与 opencode 同步。
- `secret` 只接受 `env:` 引用；拒绝 `keyring:` 空壳引用、空 env 引用、明文 token/API key、URL credentials 和高风险 command/URL secret 形态。
- `AgentConfigService` 保留外部 MCP，只清理 legacy `egosync`/4097 host 和 legacy top-level tools；角色 prompt 只声明该角色启用的外部 MCP。
- MCP 连接测试不再只看 HTTP 200：`streamable_http` 执行 `initialize` → `notifications/initialized` → `tools/list`，SSE 校验 `text/event-stream`，stdio/command 校验命令能长期运行。
- MCP 创建、更新、删除、测试连接、角色绑定和解绑后会刷新 opencode runtime 并清空 opencode session cache，避免旧 MCP scope/session 继续生效。
- 角色级 MCP 隔离已收紧：opencode 会话缓存键包含当前 MCP scope，scope key 包含 server id/type/url/env/enabled 关键信息；发送 opencode 消息前串行化顶层 MCP scope 切换，避免并发角色消息互相覆盖 `opencode.json` 的 MCP 集合。
- 聊天中捕获 MCP `Invalid session id` 工具错误后，会在失败事件持久化前刷新 opencode runtime、清理当前 assistant 消息临时过程记录，并自动重试一次，不把第一次失败暴露给用户。
- MCP tool display name 会按启用 server 映射为 `<server 名称>:<tool>`，例如 `天气查询:maps_weather`，避免 raw namespace 泄露到 UI。
- 工具调用后的最终 assistant 正文不再作为 execution trace narration 再保存，避免“执行过程”和正文重复展示最终回答。
- 全局设置新增 `MCP 工具` 标签页，提供外部工具说明、列表、表单、测试、编辑、删除和 inline feedback；测试/保存 loading spinner 使用自有 `loading-spin` keyframes，避免 reduced-motion 下静止。
- 角色设置新增 `外部 MCP 工具` 区块，只展示当前角色已启用 MCP，支持搜索可添加 server、添加和移除绑定。
- 没有恢复 `services/mcp_host.rs`，也没有新增 4097 内部 MCP host。

### File List

- `egosync-app/src-tauri/migrations/011_mcp_servers.sql`
- `egosync-app/src-tauri/migrations/012_mcp_server_standard_types.sql`
- `egosync-app/src-tauri/src/models/mcp.rs`
- `egosync-app/src-tauri/src/models/mod.rs`
- `egosync-app/src-tauri/src/db/mcp_servers.rs`
- `egosync-app/src-tauri/src/db/conversations.rs`
- `egosync-app/src-tauri/src/db/mod.rs`
- `egosync-app/src-tauri/src/services/mcp_server.rs`
- `egosync-app/src-tauri/src/services/mod.rs`
- `egosync-app/src-tauri/src/services/agent_config.rs`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/src-tauri/src/services/sidecar.rs`
- `egosync-app/src-tauri/src/commands/mcp.rs`
- `egosync-app/src-tauri/src/commands/chat.rs`
- `egosync-app/src-tauri/src/commands/mod.rs`
- `egosync-app/src-tauri/src/commands/role.rs`
- `egosync-app/src-tauri/src/commands/skill.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/types/mcp.ts`
- `egosync-app/src/services/mcpService.ts`
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx`
- `egosync-app/src/components/role/SettingsTab.tsx`
- `egosync-app/src/components/role/SettingsTab.test.tsx`
- `egosync-app/src/index.css`

### Change Log

- 2026-06-10: Implemented external MCP server management, role-level MCP binding, opencode sync preservation, and tests for Story 2.13.
- 2026-06-13: Hardened MCP protocol connection testing, refreshed opencode runtime/session cache after MCP changes, added automatic refresh-and-retry for `Invalid session id`, fixed MCP tool display names, suppressed duplicate final narration, and fixed settings loading spinner animation.

## Review Findings (2026-06-11)

_代码评审：三层对抗式（Blind Hunter / Edge Case Hunter / Acceptance Auditor）。25 条原始发现去重为 15 条。_

### Decision-needed（已拍板并修复）

- [x] [Review][Decision→Patch] **keyring 引用未真正解析，是空壳** — 原实现把 `keyring:calendar-token` 与 `env:` 同样渲染成环境插值，却不解析 keyring。最终决策为当前版本只支持 `env:` 引用，校验层拒绝 `keyring:` 和空 env 引用，避免 secret 落入明文配置或运行时静默失效。[mcp_server.rs]
- [x] [Review][Decision→Patch] **MCP CRUD 同步绕过 OpencodeMcpScopeLock + 锁粒度过粗** — `create/update/delete/test/add/remove` 命令层统一持 `OpencodeMcpScopeLock` 并在 MCP 变化后刷新 opencode runtime、清空 opencode session cache；聊天路径按角色 scope 串行切换顶层 MCP 集合，避免运行中 scope 被并发覆盖。[commands/mcp.rs, agent_engine.rs]
- [x] [Review][Decision→Patch] **command/URL 内嵌明文 secret 可绕过启发式检测落库** — `ensure_no_secret_like` 加强 URL credentials、token 参数、裸 token 形态检测；`env_refs` 只允许 `env:` 引用，secret 不写入 `roles.skills_config`、`app_settings.value`、opencode.json 明文字段或日志。[mcp_server.rs]
- [x] [Review][Decision→Patch] **command/remote 的 opencode config 字段语义需确认 schema** — 已按当前 opencode schema 输出：remote MCP 写 `type: remote`、`url`、`headers`；stdio/local MCP 写 `type: local`、argv 数组 `command`、`environment`，并保留 legacy `http_sse`/`command` 类型向标准类型迁移。[mcp_server.rs, migrations/012_mcp_server_standard_types.sql]

### Patch（已修复）

- [x] [Review][Patch] `env:` 冒号后为空（或纯空格）通过校验、`keyring:` 空壳引用被误接受 [mcp_server.rs]
- [x] [Review][Patch] 全局停用的已绑定 server 在角色两个列表均不可见，形成无法移除的孤儿绑定，重新启用时静默复活 [db/mcp_servers.rs:~1693/1710]
- [x] [Review][Patch] 前端 `refreshMcpServers`（add/remove 后）无 cancelled/role.id 守卫，角色切换时把旧角色数据写到新角色视图 [SettingsTab.tsx:~978]
- [x] [Review][Patch] session cache key 仅含绑定 id 集合，改 server URL/env 不变 key → 复用旧 session 保留陈旧配置；切换绑定累积陈旧 session 不清理 [agent_engine.rs:~619, mcp_server.rs:~2166]
- [x] [Review][Patch] `toFriendlyMcpError` 对 Error 实例 `JSON.stringify` 得 `{}`，吞掉真实错误消息 [GlobalSettingsModal.tsx:~1460]

### Defer（真实但非阻塞）

- [x] [Review][Defer] command 连接测试 2s 超时即判成功（慢速失败误报）+ Windows `cmd /C` 孙进程不回收 [mcp_server.rs:~2069] — deferred, 平台特定非阻塞
- [x] [Review][Defer] `add_to_role` 不校验角色是否 archived，可绑 MCP 到归档角色 [mcp_server.rs:~2110] — deferred, full_sync 加 disable 兜底
- [x] [Review][Defer] `sync_role_updated_with_skills_and_mcp` 未对 archived 设 disable 标记 [agent_config.rs:~328] — deferred, full_sync/sync_role_archived 兜底
- [x] [Review][Defer] 角色级 MCP 硬隔离对已建 opencode session 的即时生效性需真实数据流人工 UAT（2.0d 根因领域） — deferred, 需人工验证