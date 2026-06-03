# Story 2.13: 用户能配置 MCP server 列表并按角色接入外部工具

Status: ready-for-dev

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
   - And 只能保存 keyring 引用、环境变量引用或无密钥配置

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

- [ ] 设计 MCP server 本地配置模型（AC: 1, 2, 5）
  - [ ] 新增 SQLite 表或本地配置模型：id/name/type/command_or_url/env_refs/description/enabled/created_at/updated_at
  - [ ] 角色级启用关系独立建模，避免把完整 server 配置塞入 `roles.skills_config`
  - [ ] secret 字段只保存 keyring ref 或 env var ref；不要明文落库

- [ ] 实现 MCP server CRUD 与连接测试（AC: 1, 2, 6）
  - [ ] Rust service 负责配置校验、keyring/env ref 解析、测试连接
  - [ ] Tauri commands 示例：`mcp_server_create`、`mcp_server_update`、`mcp_server_delete`、`mcp_server_list`、`mcp_server_test`
  - [ ] Command 层返回友好 `AppError`，不使用 `.unwrap()`
  - [ ] 连接测试失败不阻塞保存草稿，除非用户明确要求“保存前必须通过”

- [ ] 更新 AgentConfigService 的 MCP 同步策略（AC: 3, 4, 5）
  - [ ] 当前 `full_sync()` 会 `root.remove("mcp")`；改为只清理旧 `egosync` MCP host 或 legacy tools，不删除用户外部 MCP 配置
  - [ ] 将启用的外部 MCP servers 写入 opencode.json 顶层 `mcp`
  - [ ] 保留 `model`、`provider`、`agent`、外部 `mcp` 等无关字段
  - [ ] 不恢复 `services/mcp_host.rs`，不新增 4097 固定端口内部 host

- [ ] 角色级 MCP 启用 UI 与同步（AC: 5）
  - [ ] 在角色 SettingsTab 或专门 MCP 设置区展示可用 MCP server 列表
  - [ ] 用户可按角色启用/禁用 server
  - [ ] 保存后触发角色 agent 配置同步或 prompt 能力约束更新
  - [ ] 未启用角色不得看到“我可以访问 X 工具”的 prompt 声明

- [ ] 前端全局 MCP 管理 UI（AC: 1, 2, 6）
  - [ ] 优先放在现有设置体系；若没有合适全局设置入口，story 内说明最小入口位置
  - [ ] 表单包含类型、URL/command、环境变量引用、说明、启用状态、测试按钮
  - [ ] 使用 inline feedback，不新增全局 toast/snackbar
  - [ ] 组件不直接调用 `invoke()`；新增 `mcpService.ts`

- [ ] 测试与验证（AC: 1-7）
  - [ ] Rust 单测：配置校验、secret ref 校验、opencode.json mcp 保留/同步、legacy egosync MCP 清理、角色启用关系
  - [ ] 前端测试：新增/编辑表单、secret 提示、测试失败展示、角色启用切换
  - [ ] 回归 `agent_config.rs` 现有 tests，尤其 full_sync、custom tools、provider 保留
  - [ ] 运行 `npm --prefix "GUI" run test:frontend`
  - [ ] 运行 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`
  - [ ] 运行 `npm --prefix "GUI" run build`
  - [ ] 可见 UI 改动需启动应用，验证新增 MCP server、测试失败、角色启用、重启持久化

## Dev Notes

### Current State

- 当前源码没有 MCP schema/service/command/UI。不要误判为“只差 UI”。
- Story 2.0d 删除了旧 `services/mcp_host.rs`，原因是 opencode project instance 下 MCP 工具可见性不可靠；EgoSync 内部工具已改为 opencode Custom Tools。
- `GUI/src-tauri/src/services/agent_config.rs::full_sync()` 当前会删除顶层 `mcp` 和 `tools`，这是本 story 必须调整的核心点。
- 顶层 `tools` 是 legacy；`agent.<role>.tools` 仍用于禁用 butler-only custom tools。不要混淆两者。
- `app_settings` 表存在（key/value），但 secret 不得明文塞入 value；复杂 MCP 配置更适合独立表或结构化本地配置。

### Architecture Guardrails

- 本 story 只接入**外部 MCP server**，不恢复 EgoSync 内部 MCP host。
- 外部 MCP 配置写入 opencode.json 的 `mcp` 时必须可被 `full_sync` 保留。
- 对外部工具的密钥采用 keyring 或环境变量引用。不得存储 API key/token 明文。
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
- Existing config code: `GUI/src-tauri/src/services/agent_config.rs` → `full_sync()`、custom tools、provider sync、agent sync。
- Initial schema: `GUI/src-tauri/migrations/001_initial_schema.sql` → `app_settings`、`llm_configs`。
- Current UI extension points: `GUI/src/components/role/SettingsTab.tsx`, `GUI/src/services/roleService.ts`, `GUI/src/types/role.ts`.

## Dev Agent Record

### Agent Model Used

TBD by dev agent

### Debug Log References

### Completion Notes List

### File List
