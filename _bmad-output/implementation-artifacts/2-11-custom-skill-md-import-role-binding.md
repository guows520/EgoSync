# Story 2.11: 用户能导入自定义 SKILL.md 并按角色启用

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 把本地自定义 SKILL.md 加入 EgoSync 的 Skill 库并绑定到角色,
so that 我的角色能复用我自己沉淀的能力模块，而不需要每次手动复制提示词。

## Acceptance Criteria

1. **导入入口与解析预览**
   - Given 用户在 `RoleView` → `SettingsTab` 的 Skill 区域点击“导入自定义 Skill”
   - When 用户选择一个本地 `SKILL.md` 文件或包含 `SKILL.md` 的目录
   - Then 系统解析 frontmatter 中的 `name`、`description` 并展示预览
   - And 解析失败时显示友好中文错误，不暴露 Rust/JS 堆栈
   - And 不把文件内容、绝对路径或可能包含隐私的信息写入普通日志

2. **Skill registry 持久化**
   - Given 用户确认导入合法 Skill
   - When 保存成功
   - Then 系统记录全局 Skill registry 条目：`id`、`name`、`description`、`sourceType="custom"`、`managedPath`、`contentHash`、`createdAt`、`updatedAt`
   - And Skill 文件复制到 EgoSync 管理的 opencode skills 目录，后续不依赖原始用户路径继续存在
   - And 应用重启后导入 Skill 仍在列表中

3. **角色绑定与 opencode 同步**
   - Given 已导入一个自定义 Skill
   - When 用户在某个角色上启用该 Skill
   - Then 该角色的 `skills_config` 记录启用的 Skill id
   - And `AgentConfigService` 同步该角色的 opencode agent 配置或 prompt 能力约束
   - And 禁用后角色不得再声明自己拥有该 Skill

4. **向后兼容 Story 2.10 元 Skill**
   - Given 现有角色的 `skills_config` 只包含 `{ "find-skills": boolean, "skill-creator": boolean }`
   - When 本故事读取或保存 Skill 配置
   - Then 旧 JSON 自动规范化为兼容结构
   - And 切换 `find-skills` / `skill-creator` 不得丢弃自定义 Skill、`permissions` 或未来扩展字段

5. **重复导入处理**
   - Given 用户导入同名或相同 content hash 的 Skill
   - When 系统检测到重复
   - Then 设置页提示已存在，并允许取消或覆盖元数据
   - And 默认不创建不可区分的重复条目

6. **现有功能不回归**
   - Role 基础信息保存、默认元 Skill 开关、主动性切换、归档、删除、记忆溯源跳转和普通对话保持可用
   - 不新增后台主动循环、不实现远程 Skill 市场、不自动下载 URL 内容

## Tasks / Subtasks

- [ ] 设计可扩展 Skill 配置 schema（AC: 2, 3, 4）
  - [ ] 在后端新增 `RoleSkillConfigV2`/helper，保留旧 key：`find-skills`、`skill-creator`
  - [ ] 支持类似结构：`meta: { findSkills, skillCreator }`、`enabledSkillIds: string[]`、`permissions?: object`
  - [ ] 读取旧 JSON 时规范化；写入时保留未知字段或明确迁移到新结构
  - [ ] 修改 `role_config::normalize_skills_config`，避免每次保存只写两个布尔值导致扩展字段丢失

- [ ] 新增 Skill registry 持久化（AC: 2, 5）
  - [ ] 优先使用 SQLite 新表 `skills` 或等价本地配置；不要把 registry 塞进每个角色的 `skills_config`
  - [ ] 字段至少包含 id/name/description/source_type/managed_path/content_hash/created_at/updated_at
  - [ ] content hash 用于重复检测；同名不同内容需要明确提示
  - [ ] managed path 指向 EgoSync 复制后的受控目录，不依赖用户原始路径

- [ ] 实现自定义 SKILL.md 导入服务与命令（AC: 1, 2, 5）
  - [ ] Rust service 负责读取文件、校验 frontmatter、复制到受控 opencode skills 目录、写 registry
  - [ ] Tauri command 示例：`skill_import_custom`、`skill_list_registry`
  - [ ] Command 层只做参数解析与 service 调用，不写业务逻辑
  - [ ] 不使用 `.unwrap()`；所有失败映射为 `AppError`

- [ ] 接通角色绑定保存链路（AC: 3, 4）
  - [ ] 新增或扩展 `role_update_skills` 输入，使其能提交启用的 registry skill ids
  - [ ] 更新 `AgentConfigService::build_agent_entry`，将启用 Skill 的可用性同步到 opencode agent 配置或 prompt
  - [ ] 确保 `sync_role_updated` 和 `full_sync` 对 active/archived role 的行为一致

- [ ] 前端 SettingsTab 接入导入与绑定 UI（AC: 1, 3, 5, 6）
  - [ ] 在现有 Skill 插件配置区下方增加“自定义 Skill”列表和导入按钮
  - [ ] 使用现有 inline feedback 模式，不新增 toast/snackbar
  - [ ] 组件不直接调用 `invoke()`；新增方法必须经 `GUI/src/services/*Service.ts`
  - [ ] 保持 Tailwind utility class，不新增 CSS 文件

- [ ] 测试与验证（AC: 1-6）
  - [ ] Rust 单测：frontmatter 解析、重复检测、旧 JSON 迁移、未知字段保留、registry CRUD、AgentConfigService 同步
  - [ ] 前端测试：导入预览、解析失败提示、启用/禁用自定义 Skill、元 Skill toggle 不丢扩展字段
  - [ ] 运行 `npm --prefix "GUI" run test:frontend`
  - [ ] 运行 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`
  - [ ] 运行 `npm --prefix "GUI" run build`
  - [ ] 可见 UI 改动需启动应用并人工验证导入、启用、重启持久化 golden path

## Dev Notes

### Current State

- `roles.skills_config` 已存在，当前是字符串 JSON；`Role` 暴露该字段，见 `GUI/src-tauri/src/models/role.rs`。
- Story 2.10 当前只支持两个元 Skill：`find-skills`、`skill-creator`。`UpdateRoleSkillsInput` 只有两个 bool。
- `GUI/src-tauri/src/services/role_config.rs` 当前 `normalize_skills_config` 会把配置重写为两个 key，这是本 story 最大回归风险。
- `AgentConfigService::build_agent_entry` 会把 `meta_skill_prompt(skills_config)` 拼进角色 prompt，同时 `parse_permissions` 会读取 `skills_config.permissions`；保存路径目前并不保证保留 `permissions`。
- `SettingsTab` 已有 Skill 插件配置区和 inline error/saved message，可扩展，不要另起一套设置页。

### Architecture Guardrails

- 前端永远不直接访问 SQLite、opencode server 或文件系统；走 Tauri command + service 封装。
- Rust `commands/` 只做 IPC 参数解析、校验与 service/db 调用；DB SQL 放在 `GUI/src-tauri/src/db/`。
- opencode Skill 文件位置按架构文档：项目级 `.opencode/skills/<name>/SKILL.md` 或全局 `~/.config/opencode/skills/`；本故事应使用 EgoSync 管理路径，避免污染用户原始文件。
- 不存储 secret。Skill 文件本身可能包含用户提示词，日志中只记录 id/hash/简短状态，不记录全文。
- 不新增外部依赖，除非现有 Rust/TS 能力无法完成 frontmatter 解析；如需依赖必须说明理由并更新测试。

### Previous Story Intelligence

- Story 2.10 completion notes 表明已新增 `services::role_config`，并把元 Skill 状态注入 `AgentConfigService` 与 fallback `agent_engine`。本 story 必须扩展这些 helper，而不是复制一套平行解析器。
- Story 2.10 已验证命令：frontend tests、Rust tests 串行、Vite build；继续沿用相同验证标准。
- 过去在 Windows 环境中 `python3` 不可用，Rust 全量测试需用 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1` 更稳定。

### Regression Risks

- **扩展字段丢失**：任何保存元 Skill 的路径都不能覆盖掉 `enabledSkillIds`、`permissions`、未来 MCP 引用。
- **角色 prompt 虚假声明**：禁用的 Skill 不得在 prompt 中被描述为可用。
- **重复 Skill 混淆**：仅按 name 去重不够，至少结合 content hash。
- **路径泄露**：用户原始路径不应出现在普通 UI 之外的日志或 LLM prompt 中。

### References

- PRD FR-4b：`_bmad-output/planning-artifacts/prd-egosync.md` → 角色 Skill 配置、用户自定义 SKILL.md、自动发现、MCP 外部工具。
- Epic 2 FR 覆盖：`_bmad-output/planning-artifacts/epics.md` → FR-4b 归属 E2。
- Story 2.10：`_bmad-output/implementation-artifacts/2-10-role-skill-config-proactivity-ui.md`。
- Architecture Skill 位置与 opencode agent 映射：`_bmad-output/planning-artifacts/architecture.md` → Agent Engine Integration / Skill体系。
- UX Settings 面板：`_bmad-output/planning-artifacts/ux-design-specification.md` → RoleWorkspacePanel Settings。
- Current files to extend: `GUI/src-tauri/src/services/role_config.rs`, `GUI/src-tauri/src/services/agent_config.rs`, `GUI/src-tauri/src/db/roles.rs`, `GUI/src-tauri/src/commands/role.rs`, `GUI/src/types/role.ts`, `GUI/src/services/roleService.ts`, `GUI/src/components/role/SettingsTab.tsx`.

## Dev Agent Record

### Agent Model Used

TBD by dev agent

### Debug Log References

### Completion Notes List

### File List
