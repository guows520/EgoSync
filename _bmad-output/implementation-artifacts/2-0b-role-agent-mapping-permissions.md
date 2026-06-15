# Story 2.0b: 角色→opencode Agent 动态映射与权限配置

Status: done

## Story

As a 开发者,
I want EgoSync 角色 CRUD 时自动同步为 opencode agent 配置,
so that 每个角色都有独立的 Agent 身份（prompt/model/permission）在 opencode 中运行。

## Acceptance Criteria

1. **AC-1 角色创建同步到 opencode.json**
   - **Given** 用户创建一个新角色（name="产品经理", goal="..."）
   - **When** 角色写入 EgoSync DB
   - **Then** opencode.json 的 `agent` 段新增对应条目（mode: "subagent", prompt 含角色 goal）

2. **AC-2 角色编辑同步更新 prompt**
   - **Given** 用户编辑角色 prompt/goal
   - **When** 保存成功
   - **Then** opencode.json 对应 agent 的 prompt 字段同步更新

3. **AC-3 角色归档禁用 agent**
   - **Given** 用户归档角色
   - **When** 归档成功
   - **Then** opencode.json 对应 agent 设为 `disable: true`

4. **AC-4 角色永久删除移除 agent**
   - **Given** 用户永久删除角色
   - **When** 删除成功
   - **Then** opencode.json 中对应 agent 条目被移除

5. **AC-5 角色权限配置同步**
   - **Given** 用户在角色设置中将 "bash" 权限从 "allow" 改为 "ask"
   - **When** 保存成功
   - **Then** opencode.json 对应 agent 的 permission 段更新为 `{ "bash": "ask" }`

6. **AC-6 管家始终为 primary agent**
   - **Given** 管家（Butler）
   - **Then** 始终作为 primary agent 存在于 opencode.json，permission 为 `{ "*": "allow" }`

7. **AC-7 代码结构**
   - **Given** Rust 后端代码
   - **Then** 存在 `services/agent_config.rs`（read/write opencode.json agent 段、同步逻辑）

8. **AC-8 测试通过**
   - `cd egosync-app/src-tauri && cargo test`
   - 至少覆盖：opencode.json 读写、agent 段同步逻辑（create/update/archive/delete）、权限映射
   - 现有测试零回归

## Tasks / Subtasks

### Phase 1: AgentConfigService 核心模块（AC: #6, #7）

- [x] T1.1 新建 `egosync-app/src-tauri/src/services/agent_config.rs`
  - `AgentConfigService` struct：持有 `config_path: PathBuf`
  - `pub fn new(config_path: PathBuf) -> Self`
  - `pub fn load(&self) -> Result<serde_json::Value, AppError>`：读取 opencode.json，文件不存在时返回默认骨架
  - `pub fn save(&self, config: &serde_json::Value) -> Result<(), AppError>`：原子写入 opencode.json（先写 .tmp 再 rename）
  - `pub fn ensure_butler(&self) -> Result<(), AppError>`：确保 butler agent 条目存在且为 primary

- [x] T1.2 定义 opencode.json agent 段数据结构
  - 不需要强类型全量映射，使用 `serde_json::Value` 操作 `agent` 段
  - 辅助函数：`role_to_agent_key(role_id: &str) -> String` — 生成 `role-{role_id}` 格式 key
  - 辅助函数：`build_agent_entry(role: &Role) -> serde_json::Value` — 从 Role 构建 agent 条目

- [x] T1.3 确定 opencode.json 路径策略
  - 优先：`app_data_dir/opencode.json`（EgoSync 管理的 opencode 配置）
  - 后续 sidecar 启动时通过 `--config` 参数指定此路径
  - 将路径存入 Tauri managed state 供各模块共享

### Phase 2: 角色生命周期同步（AC: #1, #2, #3, #4）

- [x] T2.1 `agent_config.rs`：角色同步方法
  - `pub fn sync_role_created(&self, role: &Role) -> Result<(), AppError>`
  - `pub fn sync_role_updated(&self, role: &Role) -> Result<(), AppError>`
  - `pub fn sync_role_archived(&self, role_id: &str) -> Result<(), AppError>`
  - `pub fn sync_role_deleted(&self, role_id: &str) -> Result<(), AppError>`

- [x] T2.2 将同步调用接入现有 role commands
  - `commands/role.rs::role_create` → 成功后调 `sync_role_created`
  - `commands/role.rs::role_update` → 成功后调 `sync_role_updated`
  - `commands/role.rs::role_archive` → 成功后调 `sync_role_archived`
  - `commands/role.rs::role_restore` → 成功后调 `sync_role_created`（恢复=重新激活）
  - `commands/role.rs::role_delete` → 成功后调 `sync_role_deleted`
  - **关键**：同步失败仅 `tracing::warn`，不阻塞角色 CRUD 操作（opencode 同步是增强，非硬依赖）

### Phase 3: 权限模型映射（AC: #5）

- [x] T3.1 定义 EgoSync 权限 UI 值到 opencode permission 的映射
  - "自主执行" → `"allow"`
  - "需确认" → `"ask"`
  - "禁止" → `"deny"`
  - 存储位置：复用 roles 表现有 `skills_config` JSON 字段
  - 结构：`{ "permissions": { "bash": "allow", "write": "ask", ... } }`

- [x] T3.2 `agent_config.rs`：权限同步
  - `sync_role_updated` 中解析 `skills_config.permissions` 并写入 opencode.json agent 的 `permission` 段
  - 默认权限（无配置时）：`{ "*": "allow" }`（与管家一致）

### Phase 4: 应用启动时全量同步（AC: #6）

- [x] T4.1 `agent_config.rs`：启动时全量同步方法
  - `pub async fn full_sync(&self, pool: &SqlitePool) -> Result<(), AppError>`
  - 读取所有 active 角色 → 生成 opencode.json 完整 agent 段
  - 确保 butler 条目存在
  - 标记 archived 角色为 `disable: true`
  - 删除 opencode.json 中存在但 DB 中不存在的 agent 条目（孤儿清理）

- [x] T4.2 `lib.rs`：在 sidecar 启动前执行全量同步
  - `AgentConfigService` 在 setup 闭包中初始化
  - 调用 `full_sync` 确保 opencode.json 与 DB 一致
  - 然后 sidecar 启动时读取已同步的配置

### Phase 5: 测试（AC: #8）

- [x] T5.1 `services/agent_config.rs` 单测
  - `test_role_to_agent_key`：key 格式正确
  - `test_build_agent_entry`：从 Role 构建正确的 agent JSON
  - `test_sync_role_created`：创建后 opencode.json 包含新 agent
  - `test_sync_role_updated`：更新后 prompt 变更
  - `test_sync_role_archived`：归档后 disable=true
  - `test_sync_role_deleted`：删除后条目消失
  - `test_ensure_butler`：butler 始终存在且为 primary
  - `test_permission_mapping`：skills_config 权限正确映射到 opencode permission
  - 使用 `tempfile` crate 创建临时目录避免污染真实文件系统

- [x] T5.2 运行 `cargo test` 验证零回归

## Dev Notes

### 当前实现状态

- **无 agent_config 相关代码**：项目中目前没有任何 opencode.json 管理逻辑
- **Role CRUD 完整可用**：`commands/role.rs` 已有 create/update/archive/restore/delete，均通过 `db/roles.rs` 操作
- **AgentBridge 已存在**：`services/agent_bridge.rs` 有 `get_config()`/`get_agents()` 但读取的是运行中 opencode server 的状态，不是直接操作 opencode.json 文件
- **Sidecar 已存在**：`services/sidecar.rs` 管理 opencode 进程生命周期
- **角色数据模型**：`models/role.rs` 有 `Role`（含 goal, personality_prompt, skills_config）、`CreateRoleInput`、`UpdateRoleInput`
- **`skills_config` 字段**：当前默认 `'{}'`，本 story 将用其存储权限配置

### opencode.json 目标结构

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "agent": {
    "butler": {
      "name": "管家",
      "mode": "primary",
      "prompt": "你是EgoSync管家...",
      "permission": { "*": "allow" }
    },
    "role-<uuid>": {
      "name": "产品经理",
      "mode": "subagent",
      "description": "产品经理角色Agent",
      "prompt": "你是用户的产品经理分身。目标：管理产品规划...",
      "permission": { "*": "allow", "bash": "ask" },
      "disable": false
    }
  }
}
```

### agent key 命名策略

- 格式：`role-{role_id}`（role_id 是 UUID）
- 管家固定为 `butler`
- 理由：UUID 保证唯一，避免角色重名冲突

### prompt 组装规则

```
角色名: {role.name}
目标: {role.goal}
个性: {role.personality_prompt}
```
仅包含非空字段。具体 system prompt 的三层组装（base_persona + role_definition + context_injection）在 `agent_engine.rs` 处理，本 story 只负责 opencode.json 中的 prompt 字段（供 opencode 原生使用）。

### 与现有代码的交互边界

| 模块 | 本 story 影响 |
|------|-------------|
| `services/agent_config.rs` | **NEW** — 核心新模块 |
| `services/mod.rs` | **UPDATE** — 添加 `pub mod agent_config;` |
| `commands/role.rs` | **UPDATE** — 各 command 成功后调用同步 |
| `lib.rs` | **UPDATE** — setup 中初始化 AgentConfigService + 全量同步 |
| `services/sidecar.rs` | **可能 UPDATE** — start() 增加 `--config` 参数指定 opencode.json 路径 |
| `models/role.rs` | **不改动** — skills_config 已存在 |
| `db/roles.rs` | **不改动** — CRUD 逻辑不变 |
| `services/agent_engine.rs` | **不改动** — prompt 三层组装不变 |
| `services/agent_bridge.rs` | **不改动** — HTTP 客户端不变 |

### 依赖说明

- `tempfile` crate：测试用临时目录（如未在 Cargo.toml，需添加为 dev-dependency）
- `serde_json`：已有，用于 opencode.json 读写
- 无新运行时依赖

### 降级策略

- opencode.json 写入失败 → `tracing::warn` + 角色 CRUD 正常完成
- opencode.json 不存在 → 自动创建骨架文件
- sidecar 未运行 → 配置仍写入文件，sidecar 启动时自动加载
- 理由：与 Story 2.0 一致的"非阻塞增强"设计原则

### 关键设计决策

1. **直接操作文件而非通过 API**：opencode.json 是静态配置文件，opencode 热加载。通过文件操作更简单可靠，不依赖 sidecar 运行状态。
2. **同步失败不阻塞**：角色 CRUD 是核心功能，opencode 同步是增强。
3. **启动时全量同步**：防止配置漂移（如上次崩溃导致同步中断）。
4. **原子写入**：先写 `.tmp` 再 `rename`，防止写入中断导致文件损坏。
5. **不修改现有 `agent_engine.rs`**：本 story 仅管理 opencode.json 配置文件，不改变运行时 prompt 组装逻辑。

### 前序 Story 2-5 关键信息

- `agent_engine.rs` 的 butler tools 已有 `create_role`，其调用 `db::roles::create_role` 后发 `role:proposed` 事件。本 story 需确保通过 `commands/role.rs` 入口创建的角色也触发同步。
- `agent_engine.rs::execute_create_role()` 直接调用 DB，不经过 commands 层。需评估是否也在此处挂同步钩子，或在 DB 层统一处理。
  - **决策**：在 `commands/role.rs` 层挂钩子。`execute_create_role` 最终也是通过 `db::roles::create_role` 写入的，可后续统一（此处标记为 deferred）。

## Project Structure Notes

### 新建文件

| Path | Action | Notes |
|------|--------|-------|
| `egosync-app/src-tauri/src/services/agent_config.rs` | NEW | opencode.json agent 段读写与角色同步 |

### 修改文件

| Path | Action | Notes |
|------|--------|-------|
| `egosync-app/src-tauri/src/services/mod.rs` | UPDATE | 添加 `pub mod agent_config;` |
| `egosync-app/src-tauri/src/commands/role.rs` | UPDATE | 各 command 成功后调用 AgentConfigService 同步 |
| `egosync-app/src-tauri/src/lib.rs` | UPDATE | setup 中初始化 AgentConfigService + managed state + 全量同步 |
| `egosync-app/src-tauri/Cargo.toml` | UPDATE | 添加 `tempfile` dev-dependency（如不存在） |

### 可能修改

| Path | Action | Notes |
|------|--------|-------|
| `egosync-app/src-tauri/src/services/sidecar.rs` | MAYBE UPDATE | start() 增加 `--config` 路径参数 |

### 不应改动

- `egosync-app/src-tauri/src/services/agent_engine.rs`（prompt 组装和工具逻辑不变）
- `egosync-app/src-tauri/src/services/agent_bridge.rs`（HTTP 客户端不变）
- `egosync-app/src-tauri/src/db/roles.rs`（数据层不变）
- `egosync-app/src-tauri/src/models/role.rs`（数据模型不变）
- `egosync-app/src-tauri/migrations/*.sql`（无新 migration）
- `egosync-app/src/*.tsx`（无前端改动，权限 UI 是后续 story 2-10 的工作）

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.0b AC]
- [Source: `_bmad-output/planning-artifacts/architecture.md` L293-340 — 角色→Agent映射策略、opencode.json 配置示例、权限模型映射表]
- [Source: `_bmad-output/planning-artifacts/architecture.md` L580 — Rust Backend Organization: `services/agent_config.rs`]
- [Source: `_bmad-output/implementation-artifacts/2-0-opencode-sidecar-agent-bridge.md` — sidecar 非阻塞设计、AgentBridge/SidecarManager 已实现]
- [Source: `_bmad-output/implementation-artifacts/2-5-role-emergence-suggestion.md` — agent_engine execute_create_role 直接调 DB]
- [Source: `egosync-app/src-tauri/src/commands/role.rs` — 现有 role CRUD commands 结构]
- [Source: `egosync-app/src-tauri/src/db/roles.rs` — roles DB 操作层]
- [Source: `egosync-app/src-tauri/src/models/role.rs` — Role struct（含 skills_config）]
- [Source: `egosync-app/src-tauri/src/lib.rs` — setup 闭包中 sidecar 初始化流程]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-20250514

### Debug Log References

- `cargo test`: 125 passed, 0 failed, 0 ignored (includes 11 new agent_config tests)
- Warnings: dead_code only (expected — `ensure_butler` called indirectly via `full_sync`, agent_bridge/models not yet consumed by other code)

### Completion Notes List

- Created `services/agent_config.rs`: AgentConfigService with full lifecycle sync (create/update/archive/delete) + full_sync for startup + ensure_butler
- Atomic file writes (`.tmp` + rename) prevent corruption on crash
- Permission mapping: parses `skills_config.permissions` JSON → opencode permission object; defaults to `{ "*": "allow" }`
- Integrated into `commands/role.rs`: all 5 CRUD commands now call sync with `sync_warn` helper (best-effort, never blocks)
- Integrated into `lib.rs`: AgentConfigService initialized in setup, full_sync runs before sidecar start
- Added `list_all_roles` to `db/roles.rs` for full sync
- Added `tempfile = "3"` as dev-dependency
- Design: non-blocking enhancement — sync failures log warning, CRUD always succeeds

### File List

- `egosync-app/src-tauri/src/services/agent_config.rs` (NEW)
- `egosync-app/src-tauri/src/services/mod.rs` (MODIFIED)
- `egosync-app/src-tauri/src/commands/role.rs` (MODIFIED)
- `egosync-app/src-tauri/src/lib.rs` (MODIFIED)
- `egosync-app/src-tauri/src/db/roles.rs` (MODIFIED)
- `egosync-app/src-tauri/Cargo.toml` (MODIFIED)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED)
- `_bmad-output/implementation-artifacts/2-0b-role-agent-mapping-permissions.md` (MODIFIED)

### Review Findings

- [x] [Review][Defer] opencode.json 并发写竞态 [`egosync-app/src-tauri/src/services/agent_config.rs`] — deferred, V1 单用户场景，无实质回归
- [x] [Review][Defer] `sync_role_archived` 对损坏 entry 静默 save [`egosync-app/src-tauri/src/services/agent_config.rs:148-153`] — deferred, 触发前提需手工损坏文件