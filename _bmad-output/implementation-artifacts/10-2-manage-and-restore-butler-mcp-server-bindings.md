---
baseline_commit: 4576156
---

# Story 10.2: 为管家独立管理并恢复 MCP Server 绑定

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 管家用户,
I want 在设置中独立管理管家的 MCP Server 绑定，并确保配置可恢复且准确同步到 Runtime,
so that 管家只使用我明确授权且当前启用的外部工具。

**FRs covered:** FR-39

## Acceptance Criteria

> 来源：`_bmad-output/planning-artifacts/epics.md` 第 2800-2884 行。AC 编号为本 Story 内部编号，用于 Tasks 引用。

**AC-1 数据库 Migration**
**Given** 应用升级到该版本
**When** 数据库 Migration 执行
**Then** 创建管家绑定所需的最小数据关系
**And** 不修改、复制或合并现有角色绑定
**And** 旧数据库升级后默认没有管家绑定。

**AC-2 管家 MCP 设置展示**
**Given** 用户打开管家 MCP 设置
**When** 数据加载成功
**Then** 展示现有 Server 的 enabled 状态和管家绑定状态
**And** 明确区分"Server 已启用"和"已绑定给管家"
**And** 不把角色绑定显示为管家绑定。

**AC-3 复用现有 Server 管理**
**Given** 用户执行现有 Server 查看、添加、编辑、测试、删除或启停操作
**When** 请求完成
**Then** 复用现有 MCP Server 管理链路而不重复实现
**And** Server 状态变化不修改管家或角色绑定。

**AC-4 添加管家绑定候选过滤**
**Given** 用户打开添加管家绑定入口
**When** 系统加载候选 Server
**Then** 仅展示已启用且尚未绑定给管家的 Server
**And** 支持按名称、描述或地址搜索
**And** 已关闭 Server 不可新增绑定。

**AC-5 添加/移除管家绑定隔离**
**Given** 用户添加或移除管家绑定
**When** 操作成功
**Then** 只修改管家绑定
**And** 不改变 Server enabled 状态或任何角色绑定。

**AC-6 有效集合交集**
**Given** 系统为管家准备后续会话
**When** 解析 MCP 作用域
**Then** 有效集合严格等于"管家绑定集合 ∩ enabled Server 集合"
**And** 已启用但未绑定、或已绑定但关闭的 Server 均不向管家暴露。

**AC-7 启停保留绑定**
**Given** Server 被关闭后重新启用
**When** Runtime 配置分别刷新成功
**Then** 关闭期间不暴露工具但保留所有绑定
**And** 重新启用后管家与角色原绑定分别恢复有效。

**AC-8 配置投影与 Runtime 刷新**
**Given** 管家绑定或 Server 配置发生变化
**When** 系统投影配置并刷新 Runtime
**Then** 复用 `AgentConfigService`、`OpencodeMcpScopeLock` 和现有 refresh 链路
**And** 后续会话使用最新配置
**And** 正在执行的任务保持启动时快照。

**AC-9 持久化失败提示**
**Given** 数据持久化失败
**When** 请求返回
**Then** 界面明确提示配置未保存
**And** 不显示 Runtime 已更新。

**AC-10 部分失败提示**
**Given** 数据已持久化但配置投影或 Runtime 刷新失败
**When** 结果返回前端
**Then** 明确提示"配置已保存，但 Agent Runtime 尚未刷新"
**And** 不显示为完全成功
**And** 用户可仅重试刷新，无需重复修改配置或绑定。

**AC-11 幂等操作**
**Given** 用户重复添加已有绑定或重复移除不存在的绑定
**When** 后端处理请求
**Then** 操作保持幂等
**And** 不产生重复绑定、绑定串扰或残留工具。

**AC-12 导入导出**
**Given** 用户导出或导入 EgoSync 数据
**When** 流程处理 MCP 配置
**Then** Server enabled 状态、管家绑定和角色绑定分别导出或恢复
**And** 管家绑定不覆盖或合并角色绑定
**And** 不导出 keyring 或环境变量中的明文秘密。

**AC-13 旧版本导入兼容**
**Given** 用户导入不包含管家绑定的旧版本数据
**When** 导入执行
**Then** 保持向后兼容
**And** 管家绑定初始化为空
**And** 现有角色绑定保持可用。

**AC-14 自动化测试覆盖**
**Given** Story 10.2 自动化测试运行
**When** 执行 Migration、Repository、Service、Command、组件及关键 E2E 测试
**Then** 覆盖绑定隔离、有效集合交集、启停保留绑定、部分失败、幂等操作、导入导出和旧数据兼容。

## Tasks / Subtasks

> 任务按架构路径自数据库到后端 Service/Command、再到前端、最后测试的顺序排列。每条标注涉及的 AC。所有"修改"为外科手术式扩展，不重构相邻代码。

### 阶段 1：数据库 Migration 与 DB 层（先稳定数据基础）

- [x] **Task 1: 新增 `butler_mcp_servers` 关联表 Migration** (AC: #1)
  - [x] 1.1 新建 `src-tauri/migrations/028_butler_mcp_servers.sql`，内容：
    ```sql
    CREATE TABLE IF NOT EXISTS butler_mcp_servers (
        server_id TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        PRIMARY KEY (server_id),
        FOREIGN KEY (server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
    );
    ```
    - **单字段主键 `server_id`**：管家只有一个，不需要 `role_id` 维度；与 `role_mcp_server_bindings` 的复合主键 `(server_id, role_id)` 不同，但语义对称
    - **ON DELETE CASCADE**：与 `role_mcp_server_bindings` 一致，Server 删除时自动清理绑定
    - **不新增 enabled 字段**：架构明确禁止（`architecture.md` 第 1633 行）
  - [x] 1.2 验证：旧数据库升级后 `butler_mcp_servers` 表存在且为空

- [x] **Task 2: DB 层新增管家绑定 CRUD** (AC: #1, #4, #5, #11)
  - [x] 2.1 在 `src-tauri/src/db/mcp_servers.rs` 新增四个函数，与角色对应函数严格对称：
    - `list_mcp_servers_for_butler(pool) -> Result<Vec<McpServer>, AppError>`：`SELECT {COLUMNS} FROM mcp_servers INNER JOIN butler_mcp_servers ON butler_mcp_servers.server_id = mcp_servers.id ORDER BY butler_mcp_servers.created_at ASC`（对称：`list_mcp_servers_for_role`，第 94-109 行）
    - `list_available_mcp_servers_for_butler(pool) -> Result<Vec<McpServer>, AppError>`：`SELECT {COLUMNS} FROM mcp_servers WHERE enabled = 1 AND id NOT IN (SELECT server_id FROM butler_mcp_servers) ORDER BY created_at ASC`（对称：`list_available_mcp_servers_for_role`，第 111-126 行；无需 `role_id` 参数）
    - `add_mcp_server_to_butler(pool, server_id) -> Result<(), AppError>`：先 `get_mcp_server(pool, server_id)` 验证存在，再 `INSERT OR IGNORE INTO butler_mcp_servers (server_id, created_at) VALUES (?1, ?2)`（对称：`add_mcp_server_to_role`，第 128-147 行；`INSERT OR IGNORE` 保证幂等，AC-11）
    - `remove_mcp_server_from_butler(pool, server_id) -> Result<(), AppError>`：`DELETE FROM butler_mcp_servers WHERE server_id = ?1`（对称：`remove_mcp_server_from_role`，第 149-161 行；删除不存在的记录不报错，保证幂等，AC-11）
  - [x] 2.2 新增 `butler_enabled_mcp_lines(pool) -> Result<Vec<String>, AppError>`：调用 `list_mcp_servers_for_butler`，过滤 `enabled`，生成 `format!("- {}（{}）：{}", server.name, mcp_server_type_label(&server.server_type), server.description)`（对称：`role_enabled_mcp_lines`，第 163-170 行）
  - [x] 2.3 单元测试覆盖：绑定列表、可用列表（排除已绑定和已关闭）、添加（含幂等）、移除（含幂等）、`butler_enabled_mcp_lines`（只返回绑定且 enabled 的 Server）

### 阶段 2：Service 层与 Agent 配置同步

- [x] **Task 3: Service 层新增管家绑定操作** (AC: #4, #5, #8, #10, #11)
  - [x] 3.1 在 `src-tauri/src/services/mcp_server.rs` 新增：
    - `list_servers_for_butler(pool) -> Result<Vec<McpServer>, AppError>`：委托 `db::list_mcp_servers_for_butler(pool)`（对称：`list_servers_for_role`，第 51-56 行）
    - `list_available_servers_for_butler(pool) -> Result<Vec<McpServer>, AppError>`：委托 `db::list_available_mcp_servers_for_butler(pool)`（对称：`list_available_servers_for_role`，第 58-63 行）
    - `add_to_butler(pool, agent_config, server_id) -> Result<(), AppError>`：
      1. `db::get_mcp_server(pool, server_id).await?` 验证 Server 存在
      2. `if !server.enabled` → `ValidationError("该 MCP server 已全局停用，不能添加到管家")`（对称：`add_to_role`，第 386-389 行）
      3. `db::add_mcp_server_to_butler(pool, server_id).await?`
      4. 调用 `sync_butler_agent(pool, agent_config).await` 同步管家 Agent 配置
    - `remove_from_butler(pool, agent_config, server_id) -> Result<(), AppError>`：
      1. `db::remove_mcp_server_from_butler(pool, server_id).await?`
      2. 调用 `sync_butler_agent(pool, agent_config).await` 同步管家 Agent 配置
  - [x] 3.2 新增 `sync_butler_agent(pool, agent_config)` 私有函数（对称：`sync_role_agent`，第 485-496 行）：
    ```rust
    async fn sync_butler_agent(pool: &SqlitePool, agent_config: &AgentConfigService) {
        let result: Result<(), AppError> = (|| async {
            let butler_skills = crate::services::butler_config::get_butler_skills(pool).await?;
            let registry = crate::db::skills::list_skills(pool).await.unwrap_or_default();
            let mcp_lines = db::butler_enabled_mcp_lines(pool).await?;
            agent_config.sync_butler_skills_with_registry_and_mcp(&butler_skills, &registry, &mcp_lines)
        })()
        .await;
        if let Err(e) = result {
            tracing::warn!("sync butler MCP prompt failed: {}", e);
        }
    }
    ```
  - [x] 3.3 扩展 `sync_all_role_agents`（第 498-510 行）使其同时同步管家 MCP：新增 `let butler_mcp_lines = db::butler_enabled_mcp_lines(pool).await.unwrap_or_default();`，修改 `full_sync_with_skills_and_mcp` 调用传入 `butler_mcp_lines`

- [x] **Task 4: 扩展 AgentConfigService 支持管家 MCP 能力说明** (AC: #6, #8)
  - [x] 4.1 新增 `build_butler_entry_with_skills_and_mcp(skills, registry, mcp_lines) -> Value`：复制现有 `build_butler_entry_with_skills`（第 504-569 行）全部逻辑，在 `push_custom_skill_prompt` 之后、`PLAIN_MESSAGE_CONFIRMATION_RULE` 之前插入 MCP 段落（对称：`build_agent_entry_with_skills_and_mcp` 第 426-431 行）：
    ```rust
    if !mcp_lines.is_empty() {
        prompt_parts.push(format!(
            "[外部 MCP 工具]\n{}\n只能使用以上为管家启用的外部 MCP server；不要声明或调用未启用的外部工具。",
            mcp_lines.join("\n")
        ));
    }
    ```
  - [x] 4.2 修改 `build_butler_entry_with_skills`（第 504 行）为委托：`Self::build_butler_entry_with_skills_and_mcp(skills, registry, &[])`
  - [x] 4.3 新增 `sync_butler_skills_with_registry_and_mcp(skills, registry, mcp_lines) -> Result<(), AppError>`：复制 `sync_butler_skills_with_registry`（第 575-593 行）逻辑，替换为 `build_butler_entry_with_skills_and_mcp`
  - [x] 4.4 修改 `full_sync_with_skills_and_mcp`（第 803-845 行）签名新增 `butler_mcp_lines: &[String]` 参数，第 817-820 行替换为 `build_butler_entry_with_skills_and_mcp(butler_skills, registry, butler_mcp_lines)`
  - [x] 4.5 修改所有调用点：
    - `mcp_server.rs` `sync_all_role_agents`：传入 `butler_mcp_lines`
    - `delegate_bridge.rs` 第 374-376 行：改为 `sync_butler_skills_with_registry_and_mcp`，传入 `butler_enabled_mcp_lines` 结果
    - `commands/app.rs` 第 51 行：改为 `sync_butler_skills_with_registry_and_mcp`，传入 `butler_enabled_mcp_lines` 结果
  - [x] 4.6 **不修改** `role_enabled_mcp_lines`、`add_to_role`、`remove_from_role`、`sync_mcp_scope_for_role` 的行为（回归边界）

### 阶段 3：Tauri Command 层

- [x] **Task 5: 新增管家 MCP 绑定 Tauri Commands** (AC: #2, #4, #5, #8, #10)
  - [x] 5.1 在 `src-tauri/src/commands/mcp.rs` 新增四个 Command，与角色 Command 严格对称：
    - `mcp_server_list_for_butler(pool) -> Result<Vec<McpServer>, AppError>`
    - `mcp_server_list_available_for_butler(pool) -> Result<Vec<McpServer>, AppError>`
    - `mcp_server_add_to_butler(server_id, pool, agent_config, mcp_scope_lock, sidecar, opencode_sessions) -> Result<(), AppError>`：加锁 → service 调用 → `refresh_opencode_runtime_after_mcp_change`
    - `mcp_server_remove_from_butler(server_id, pool, agent_config, mcp_scope_lock, sidecar, opencode_sessions) -> Result<(), AppError>`：加锁 → service 调用 → `refresh_opencode_runtime_after_mcp_change`
  - [x] 5.2 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 宏中注册四个新 Command（在第 353 行 `mcp_server_remove_from_role` 之后）
  - [x] 5.3 更新 `commands/mcp.rs` 测试 `mutating_mcp_commands_refresh_opencode_runtime`（第 150-187 行）：在 `command` 列表中添加 `"mcp_server_add_to_butler"` 和 `"mcp_server_remove_from_butler"`

### 阶段 4：前端 Service 与类型

- [x] **Task 6: 扩展 mcpService** (AC: #2, #4, #5)
  - [x] 6.1 在 `src/services/mcpService.ts` 新增四个方法（对称：第 6-13 行角色方法）：
    ```typescript
    listForButler: () => invoke<McpServer[]>('mcp_server_list_for_butler'),
    listAvailableForButler: () => invoke<McpServer[]>('mcp_server_list_available_for_butler'),
    addToButler: (serverId: string) => invoke<void>('mcp_server_add_to_butler', { serverId }),
    removeFromButler: (serverId: string) => invoke<void>('mcp_server_remove_from_butler', { serverId }),
    ```
  - [x] 6.2 `src/types/mcp.ts` 无需修改（复用现有 `McpServer` DTO）

### 阶段 5：前端 UI — 管家设置 MCP 绑定管理

- [x] **Task 7: ButlerSettingsContent 新增 MCP 绑定管理 UI** (AC: #2, #4, #5, #9, #10)
  - [x] 7.1 在 `src/components/butler/ButlerSettingsContent.tsx` 新增 MCP 绑定管理区块：
    - 新增 state：`butlerMcpServers`、`availableButlerMcpServers`、`isLoadingButlerMcpServers`、`isButlerMcpPickerOpen`、`butlerMcpSearch`、`pendingButlerMcpId`
    - 新增 `useEffect` 加载 `mcpService.listForButler()` 和 `mcpService.listAvailableForButler()`（对称：`SettingsTab.tsx` 第 193-215 行）
    - 新增 `refreshButlerMcpServers` 函数（对称：`SettingsTab.tsx` 第 217-225 行）
    - 新增 `handleAddMcpToButler(serverId)` 和 `handleRemoveMcpFromButler(serverId)`（对称：`SettingsTab.tsx` 第 512-548 行）
    - 新增搜索过滤 `filteredAvailableButlerMcpServers`（对称：`SettingsTab.tsx` 第 110-118 行）
  - [x] 7.2 在 `BUTLER_SECTION_IDS` 中新增 `'mcp'`，渲染 MCP 绑定管理 section：已绑定列表（含移除按钮）、可添加列表（Picker，含搜索和添加按钮）
  - [x] 7.3 错误处理：持久化失败显示明确错误（AC-9）；Runtime 刷新失败显示部分失败提示（AC-10）——复用现有 `toFriendlyError` 模式
  - [x] 7.4 **不重构**角色 `SettingsTab.tsx` 的 MCP UI（回归边界）

### 阶段 6：数据导出/导入/销毁闭环

- [x] **Task 8: 扩展数据导出/导入/销毁覆盖管家绑定** (AC: #12, #13)
  - [x] 8.1 在 `src-tauri/src/services/data_export.rs` 的 `ExportData` 结构体（第 36-59 行）新增 `#[serde(default)] pub butler_mcp_servers: Vec<serde_json::Value>`（放在 `role_mcp_server_bindings` 之后）
  - [x] 8.2 新增 `query_butler_mcp_servers(pool)` 函数（对称：`query_role_mcp_server_bindings`，第 254-262 行）：`SELECT server_id, created_at FROM butler_mcp_servers ORDER BY server_id ASC`
  - [x] 8.3 在 `export_all` 函数中调用并填入 `ExportData`
  - [x] 8.4 在 `IMPORT_TABLES` 常量（第 698-718 行）中添加 `"butler_mcp_servers"`（实际复用 `MAIN_DB_TABLES`）
  - [x] 8.5 在 `MAIN_DB_TABLES` 常量中添加 `"butler_mcp_servers"`
  - [x] 8.6 在 `import_all` 函数中新增 `butler_mcp_servers` 插入逻辑（对称：第 923-934 行 `role_mcp_server_bindings` 导入）
  - [x] 8.7 **旧版本兼容**（AC-13）：`#[serde(default)]` 确保缺失字段自动为空 Vec，不报错
  - [x] 8.8 更新所有测试中的 `ExportData` 构造，添加 `butler_mcp_servers: vec![]`（6 处）
  - [x] 8.9 Markdown 导出无 `role_mcp_server_bindings` 段落，对称不新增管家 MCP Markdown 段落

### 阶段 7：测试

- [x] **Task 9: Rust 测试** (AC: #1, #4, #5, #6, #7, #11, #12, #13)
  - [x] 9.1 `db/mcp_servers.rs` 测试：管家绑定 CRUD、可用列表排除已绑定和已关闭、幂等添加/移除、`butler_enabled_mcp_lines` 只返回绑定且 enabled（6 个测试已存在）
  - [x] 9.2 `services/mcp_server.rs` 测试：`add_to_butler` 拒绝已关闭 Server、不影响角色绑定、`remove_from_butler` 不影响角色绑定（新增 3 个测试）
  - [x] 9.3 `services/agent_config.rs` 测试：`build_butler_entry_with_skills_and_mcp` 含 MCP 段落、无 MCP 时不含（回归）、`full_sync_with_skills_and_mcp` 管家 entry 含 MCP（新增 3 个测试）
  - [x] 9.4 `services/data_export.rs` 测试：导出含 `butler_mcp_servers`、导入恢复绑定、旧数据兼容（AC-13）、`destroy_all_data` 清空 `butler_mcp_servers`（新增 3 个测试）
  - [x] 9.5 `commands/mcp.rs` 测试：新增 Command 含 sidecar 和 opencode_sessions 状态并调用 refresh（Task 5 已更新）
  - [x] 9.6 测试 DB 初始化（`setup_test_db`）已包含 `butler_mcp_servers` 表创建（db/mcp_servers.rs 和 services/mcp_server.rs）

- [x] **Task 10: 前端组件测试** (AC: #2, #4, #5, #9, #10)
  - [x] 10.1 `ButlerSettingsContent.test.tsx` 新增 6 个 MCP 测试：加载列表、添加/移除调用正确 service、搜索过滤、持久化失败提示、已关闭 Server 不在可添加列表。更新现有折叠测试的 section 名称和 localStorage 结构。

- [x] **Task 11: 回归验证** (AC: #3, #5, #7)
  - [x] 11.1 角色 `SettingsTab.test.tsx` 现有测试全部通过（34 passed）
  - [x] 11.2 `mcp_server.rs` 现有角色测试全部通过（7 passed）
  - [x] 11.3 `agent_config.rs` 现有角色 MCP 测试全部通过（4 passed, 1 pre-existing Windows 编码失败与本次无关）

**测试命令**：
```powershell
cd egosync-app/src-tauri && cargo check
cd egosync-app/src-tauri && cargo test
cd egosync-app && npx vitest run
cd egosync-app && npx tsc --noEmit
cd egosync-app && npm run build
```

**测试纪律**：任一测试被跳过或环境缺失时，必须明确记录，不能宣称全部通过（AGENTS 规则十二：显式失败）。

### Review Findings

- [x] [Review][Patch] 管家 Runtime MCP scope 仍使用全部全局启用 Server，未应用“管家绑定 ∩ enabled”授权集合 [egosync-app/src-tauri/src/services/mcp_server.rs:462]
- [x] [Review][Patch] 配置投影与 Runtime 刷新错误被吞掉，命令仍返回完全成功且无仅重试刷新入口 [egosync-app/src-tauri/src/services/mcp_server.rs:530]
- [x] [Review][Patch] 旧版 SQLite 存档缺少 butler_mcp_servers 表时被判定为结构不兼容 [egosync-app/src-tauri/src/services/data_export.rs:1153]
- [x] [Review][Patch] 多条同步路径将管家 MCP 查询失败降级为空集合并覆盖有效配置 [egosync-app/src-tauri/src/commands/app.rs:51]
- [x] [Review][Patch] 前端将持久化失败、Runtime 部分失败和成功后的列表刷新失败混为普通操作失败 [egosync-app/src/components/butler/ButlerSettingsContent.tsx:572]
- [x] [Review][Patch] 管家 Skill 更新未参与 MCP scope 锁，可能与绑定变更并发覆盖 opencode.json [egosync-app/src-tauri/src/commands/app.rs:42]
- [x] [Review][Patch] 自动化测试未覆盖真实投影/刷新失败、刷新重试及旧版 SQLite 导入兼容路径 [egosync-app/src-tauri/src/commands/mcp.rs:195]

## Dev Notes

### 核心架构决策（`architecture.md` 第 1451-1504 行）

**决策：采用最小增量方案，管家仿照角色现有 MCP 处理逻辑增加独立绑定能力，角色实现保持不变。**

```
管家有效 MCP = 管家已绑定 Server ∩ enabled Server
角色有效 MCP = 维持现有角色实现
```

`McpServer.enabled` 继续表示 Server 启停状态；管家绑定只表示管家选择使用哪些 Server。Server 关闭时不删除既有绑定，重新启用后原绑定自动恢复有效。管家绑定与所有角色绑定相互独立。

### 数据与接口（`architecture.md` 第 1464-1475 行）

新增独立 `butler_mcp_servers` 关联表，不迁移现有角色绑定表、不引入通用 Agent 绑定模型、不使用虚拟角色 ID。绑定规则沿用角色当前行为：关闭的 Server 不能新增绑定；已绑定 Server 被关闭后保留绑定但不进入管家有效集合。

### Runtime 与明确排除（`architecture.md` 第 1483-1495 行）

管家绑定变化沿用现有 Agent 配置同步方式；MCP Server 的新增、编辑、启用和关闭继续走现有 Runtime 刷新逻辑。本轮不新增 `McpRuntimeCoordinator`、Agent Permission 投影、Runtime revision、通用 Availability Resolver 或新的并发锁。

以下现有行为明确保持不变：
- 不修改角色 MCP 授权和绑定逻辑
- 不重构或删除 `sync_mcp_scope_for_role()`
- 不改变 `add_to_role()` 对关闭 Server 的校验
- 不建立管家与角色统一的 MCP 权限框架

### 管家 MCP 绑定模式（`architecture.md` 第 1570-1604 行）

**数据库命名**：新增表固定命名为 `butler_mcp_servers`，字段遵循现有 snake_case 规范。不新增通用 `agent_mcp_bindings`、scope 字段、虚拟 Butler role ID 或管家专属 enabled 字段。

**Repository 对称**：排序、Server 不存在、重复绑定、移除不存在绑定和级联删除语义均匹配角色对应函数；本轮不抽取通用 Agent MCP Repository。

**Command 对称**：Command 只解析参数并调用 Service。管家添加/移除绑定沿用角色当前的校验、Agent 配置同步、`OpencodeMcpScopeLock`、Sidecar 和 Session 刷新调用顺序，不新增第二套刷新机制。

**数据主权一致性**：新增 `butler_mcp_servers` 后必须同步检查数据导出、导入、全量销毁、Server 删除级联和测试数据库初始化。导出结构与角色 MCP 绑定保持同类风格，但使用独立集合。

### 禁止模式（`architecture.md` 第 1627-1637 行）

- ❌ 给 `butler_mcp_servers` 增加独立 enabled 字段
- ❌ 为管家 MCP 新建 Runtime Coordinator
- ❌ 顺手重构现有角色 MCP 绑定
- ❌ 使用 `roleId = "butler"` 表示管家
- ❌ 引入通用 Agent MCP 权限模型

### 现有代码当前状态（必须阅读后再修改）

**数据库层**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `src-tauri/migrations/011_mcp_servers.sql` 第 16-22 行 | `role_mcp_server_bindings` 表 | 复合主键 `(server_id, role_id)`，CASCADE | 新建 `028_butler_mcp_servers.sql`，单字段主键 `server_id`，CASCADE |
| `src-tauri/src/db/mcp_servers.rs` 第 94-170 行 | 角色绑定 CRUD + `role_enabled_mcp_lines` | 已有角色 DB 函数 | 新增管家对应函数（4 CRUD + 1 enabled_mcp_lines） |

**Service 层**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `services/mcp_server.rs` 第 51-63 行 | `list_servers_for_role`、`list_available_servers_for_role` | 角色 Service 查询 | 新增管家对应函数 |
| `services/mcp_server.rs` 第 380-404 行 | `add_to_role`、`remove_from_role` | 角色 Service 绑定，含 enabled 校验和 `sync_role_agent` | 新增 `add_to_butler`、`remove_from_butler`，含 `sync_butler_agent` |
| `services/mcp_server.rs` 第 485-510 行 | `sync_role_agent`、`sync_all_role_agents` | 角色 Agent 同步 | 新增 `sync_butler_agent`；扩展 `sync_all_role_agents` |
| `services/agent_config.rs` 第 504-569 行 | `build_butler_entry_with_skills` | 管家 Agent entry，无 MCP | 新增 `build_butler_entry_with_skills_and_mcp`；原函数委托 |
| `services/agent_config.rs` 第 575-593 行 | `sync_butler_skills_with_registry` | 管家 Agent 同步，无 MCP | 新增 `sync_butler_skills_with_registry_and_mcp` |
| `services/agent_config.rs` 第 803-845 行 | `full_sync_with_skills_and_mcp` | 全量同步，管家无 MCP | 新增 `butler_mcp_lines` 参数 |

**Command 层**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `commands/mcp.rs` 第 19-32 行 | `mcp_server_list_for_role`、`mcp_server_list_available_for_role` | 角色 Command 列表 | 新增管家 Command（无 `role_id`） |
| `commands/mcp.rs` 第 96-126 行 | `mcp_server_add_to_role`、`mcp_server_remove_from_role` | 角色 Command 绑定 | 新增管家 Command（无 `role_id`），含锁和 refresh |
| `commands/mcp.rs` 第 150-187 行 | 测试 `mutating_mcp_commands_refresh_opencode_runtime` | 验证角色 Command refresh | 添加管家 Command 到测试列表 |
| `lib.rs` 第 345-353 行 | MCP Command 注册 | 已有角色注册 | 新增 4 个管家 Command |

**前端**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `src/services/mcpService.ts` 第 6-13 行 | `listForRole`、`addToRole` 等 | 角色 MCP Service | 新增 `listForButler`、`addToButler` 等 |
| `src/components/butler/ButlerSettingsContent.tsx` | 1295 行，无 MCP UI | 管家设置页 | 新增 MCP 绑定管理 section |
| `src/components/role/SettingsTab.tsx` 第 89-548 行 | 角色 MCP UI | 角色 MCP 设置 | **不修改**（回归边界） |

**数据导出/导入**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `services/data_export.rs` 第 36-59 行 | `ExportData` 结构体 | 含 `role_mcp_server_bindings` | 新增 `butler_mcp_servers` 字段 |
| `services/data_export.rs` 第 254-262 行 | `query_role_mcp_server_bindings` | 角色绑定导出查询 | 新增 `query_butler_mcp_servers` |
| `services/data_export.rs` 第 698-718 行 | `MAIN_DB_TABLES`、`IMPORT_TABLES` | 含 `role_mcp_server_bindings` | 添加 `butler_mcp_servers` |
| `services/data_export.rs` 第 923-934 行 | `role_mcp_server_bindings` 导入 | 角色绑定导入 | 新增管家绑定导入 |
| `services/data_export.rs` 第 1278-1317 行 | `destroy_all_data` | 清空所有表 | 自动覆盖（通过 `MAIN_DB_TABLES`） |

### 现有可复用能力（禁止重复实现）

1. **MCP Server CRUD**：`db::mcp_servers` 的 `list_mcp_servers`、`get_mcp_server`、`insert_mcp_server`、`update_mcp_server`、`delete_mcp_server` 直接复用
2. **角色 MCP 绑定模式**：`db::mcp_servers` 第 94-170 行的角色 CRUD 是管家实现的对称模板
3. **Agent 配置同步**：`AgentConfigService` 的 `sync_butler_skills_with_registry` 是管家同步入口，扩展为含 MCP 即可
4. **Runtime 刷新**：`refresh_opencode_runtime_after_mcp_change`（`commands/mcp.rs` 第 138-145 行）直接复用
5. **`OpencodeMcpScopeLock`**：现有锁机制直接复用，不新建并发锁
6. **前端 MCP UI 模式**：`SettingsTab.tsx` 的 MCP section 是管家 UI 的对称模板
7. **数据导出/导入框架**：`data_export.rs` 现有 `ExportData`/`import_all`/`destroy_all_data` 框架直接扩展

### 必须保留的回归边界

**后端**：
- `role_mcp_server_bindings` 表结构不修改
- `role_enabled_mcp_lines`、`add_to_role`、`remove_from_role`、`sync_mcp_scope_for_role` 行为不变
- `build_agent_entry_with_skills_and_mcp` 不修改
- `sync_role_updated_with_skills_and_mcp` 不修改
- `sync_role_agent` 不修改
- 现有角色 MCP 测试全部通过

**前端**：
- `SettingsTab.tsx` MCP UI 不重构
- `mcpService.ts` 现有角色方法签名不变
- `types/mcp.ts` `McpServer` DTO 不修改

**业务逻辑**：
- 角色 MCP 绑定与管家 MCP 绑定完全独立
- Server enabled 状态变化不修改任何绑定
- 现有 Server CRUD 操作不重复实现

### 增量变更边界（`architecture.md` 第 1614-1626 行）

1. 优先扩展现有同类 Service、Repository 和 AgentConfig 路径
2. 不创建第二套 Runtime 刷新或错误包装机制
3. 不借管家 MCP 功能重构角色 MCP
4. 不引入通用 Agent MCP 权限模型
5. 新增 Tauri Command 时同步增加前端 Service、TypeScript DTO 和注册入口
6. 新增数据库表时同步检查 migration、导入导出、销毁和测试初始化
7. 任一步骤失败必须显式返回，禁止记录日志后继续宣称成功

### Project Structure Notes

- 所有修改遵循 `_bmad-output/project-context.md` 的目录组织、命名规范和分层架构
- 后端遵循三层架构：Command 只解析参数 → Service 含业务逻辑 → DB 只执行 SQL
- 新增 Tauri Command 必须在 Rust Command、`lib.rs generate_handler!`、前端 Service、TypeScript DTO 和测试中闭环（project-context.md 第 205-212 行检查清单）
- 新增数据库表时同步检查 migration、导入导出、销毁和测试 DB 初始化

### References

- `_bmad-output/planning-artifacts/epics.md` 第 2800-2884 行 — Story 10.2 完整 AC
- `_bmad-output/planning-artifacts/architecture.md` 第 1205-1211 行 — FR-39 需求概述
- `_bmad-output/planning-artifacts/architecture.md` 第 1451-1504 行 — FR-39 核心架构决策
- `_bmad-output/planning-artifacts/architecture.md` 第 1570-1604 行 — 管家 MCP 绑定实现模式
- `_bmad-output/planning-artifacts/architecture.md` 第 1627-1637 行 — 禁止模式
- `_bmad-output/planning-artifacts/architecture.md` 第 1639-1712 行 — 项目结构增量与文件标记
- `_bmad-output/planning-artifacts/architecture.md` 第 1737-1746 行 — FR-39 架构边界
- `_bmad-output/planning-artifacts/architecture.md` 第 1756-1767 行 — Tauri IPC 边界
- `_bmad-output/project-context.md` — 技术栈、命名规范、分层架构、禁止事项
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-39：管家 MCP 绑定能力]
- [Source: _bmad-output/planning-artifacts/architecture.md#管家 MCP 绑定模式]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 审查发现管家 Runtime scope 使用全局 enabled 集合，修正为“管家绑定 ∩ enabled”。
- 审查发现绑定已持久化后的配置投影/Runtime 刷新失败被吞掉，新增明确的部分成功错误与仅刷新重试命令。
- 回归测试首次暴露测试数据库缺少 `app_settings`/`skills`，补齐最小真实投影测试夹具后通过。
- `rustfmt --check` 会触及本 Story 开始前已存在的大量格式差异，因此未执行全仓格式化；使用 `git diff --check` 验证本次补丁无空白符错误。

### Completion Notes List

- ✅ 管家 Agent 配置只投影已绑定且当前启用的 MCP Server，角色绑定行为保持独立。
- ✅ 绑定持久化、Runtime 部分失败、列表刷新失败在 UI 中分别呈现；Runtime 部分失败支持“重试刷新”。
- ✅ MCP 查询、配置投影与启动同步失败均显式返回，不再用空集合覆盖有效配置。
- ✅ 管家 Skill 更新与 MCP 绑定变更共享 `OpencodeMcpScopeLock`。
- ✅ 新旧 SQLite 归档均可恢复；旧归档缺少 `butler_mcp_servers` 时按空绑定导入。
- ✅ 验证通过：`cargo check`、MCP command/service 定向测试、两条数据导入测试、前端 34 项组件测试、`tsc --noEmit`、`npm run build`、`git diff --check`。

### Change Log

- 2026-07-24: 完成 Story 10.2 实现及代码审查修复，七项 Review Finding 全部关闭。

### File List

**新增文件：**
- `egosync-app/src-tauri/migrations/028_butler_mcp_servers.sql`

**修改文件：**
- `egosync-app/src-tauri/src/db/mcp_servers.rs`
- `egosync-app/src-tauri/src/services/mcp_server.rs`
- `egosync-app/src-tauri/src/services/agent_config.rs`
- `egosync-app/src-tauri/src/services/data_export.rs`
- `egosync-app/src-tauri/src/services/delegate_bridge.rs`
- `egosync-app/src-tauri/src/commands/mcp.rs`
- `egosync-app/src-tauri/src/commands/app.rs`
- `egosync-app/src-tauri/src/commands/skill.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/services/mcpService.ts`
- `egosync-app/src/components/butler/ButlerSettingsContent.tsx`
- `egosync-app/src/components/butler/ButlerSettingsContent.test.tsx`
