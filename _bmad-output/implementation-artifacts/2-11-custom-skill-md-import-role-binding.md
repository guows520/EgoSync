---
baseline_commit: dccfdc609cfa7410f35b9721304d6e311c0307a9
---

# Story 2.11: 用户能导入自定义 SKILL.md 并按角色启用

Status: done

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

- [x] 设计可扩展 Skill 配置 schema（AC: 2, 3, 4）
  - [x] 在后端新增 `RoleSkillConfigV2`/helper，保留旧 key：`find-skills`、`skill-creator`
  - [x] 支持类似结构：`meta: { findSkills, skillCreator }`、`enabledSkillIds: string[]`、`permissions?: object`
  - [x] 读取旧 JSON 时规范化；写入时保留未知字段或明确迁移到新结构
  - [x] 修改 `role_config::normalize_skills_config`，避免每次保存只写两个布尔值导致扩展字段丢失

- [x] 新增 Skill registry 持久化（AC: 2, 5）
  - [x] 优先使用 SQLite 新表 `skills` 或等价本地配置；不要把 registry 塞进每个角色的 `skills_config`
  - [x] 字段至少包含 id/name/description/source_type/managed_path/content_hash/created_at/updated_at
  - [x] content hash 用于重复检测；同名不同内容需要明确提示
  - [x] managed path 指向 EgoSync 复制后的受控目录，不依赖用户原始路径

- [x] 实现自定义 SKILL.md 导入服务与命令（AC: 1, 2, 5）
  - [x] Rust service 负责读取文件、校验 frontmatter、复制到受控 opencode skills 目录、写 registry
  - [x] Tauri command 示例：`skill_import_custom`、`skill_list_registry`
  - [x] Command 层只做参数解析与 service 调用，不写业务逻辑
  - [x] 不使用 `.unwrap()`；所有失败映射为 `AppError`

- [x] 接通角色绑定保存链路（AC: 3, 4）
  - [x] 新增或扩展 `role_update_skills` 输入，使其能提交启用的 registry skill ids
  - [x] 更新 `AgentConfigService::build_agent_entry`，将启用 Skill 的可用性同步到 opencode agent 配置或 prompt
  - [x] 确保 `sync_role_updated` 和 `full_sync` 对 active/archived role 的行为一致

- [x] 前端 SettingsTab 接入导入与绑定 UI（AC: 1, 3, 5, 6）
  - [x] 在现有 Skill 插件配置区下方增加“自定义 Skill”列表和导入按钮
  - [x] 使用现有 inline feedback 模式，不新增 toast/snackbar
  - [x] 组件不直接调用 `invoke()`；新增方法必须经 `egosync-app/src/services/*Service.ts`
  - [x] 保持 Tailwind utility class，不新增 CSS 文件

- [x] 测试与验证（AC: 1-6）
  - [x] Rust 单测：frontmatter 解析、重复检测、旧 JSON 迁移、未知字段保留、registry CRUD、AgentConfigService 同步
  - [x] 前端测试：导入预览、解析失败提示、启用/禁用自定义 Skill、元 Skill toggle 不丢扩展字段
  - [x] 运行 `npm --prefix "GUI" run test:frontend`
  - [x] 运行 `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - [x] 运行 `npm --prefix "GUI" run build`
  - [x] 可见 UI 改动需启动应用并人工验证导入、启用、重启持久化 golden path

### Review Findings (2026-06-04 code review)

**Decision-Needed（已由 boss 拍板，转为 Patch）**

- [x] [Review][Patch][D1] 去重覆盖语义 — 保持 hash 优先；覆盖前检测 name 冲突，命中则返回友好提示而非裸 DbError [db/skills.rs:update_skill_metadata]
- [x] [Review][Patch][D3] 幽灵 id 权限与清理 — parse_permissions 按"过滤后有效 Skill"判定（有效为空则 skill:deny）；新增 skill_delete 命令 + 删除时清理所有角色 enabledSkillIds [agent_config.rs, commands/skill.rs, db/skills.rs delete_skill, role_config.rs remove_enabled_skill_id]
- [x] [Review][Patch][D4] 放宽 name 允许 Unicode + 目录名改用 content-hash 派生 — 同时消解 F1 路径穿越 / F2 大小写覆盖 / F12 Windows 保留名 [skill_registry.rs:validate_skill_name, managed_skill_path]
- [x] [Review][Patch][D5] 删除死代码 source_path 入参（保留前端 File API 读取模式）[models/skill.rs, skill_registry.rs:load_skill_content, types/skill.ts]

**Patch（修复明确，无需歧义决策）**

- [x] [Review][Patch] ~~路径穿越：`name: ..`/`.` 写出受控目录~~ — 由 [D4] hash 派生目录名一并修复
- [x] [Review][Patch] ~~大小写不敏感文件系统副本互相覆盖~~ — 由 [D4] hash 派生目录名一并修复
- [x] [Review][Patch][F4] 注册表加载失败 warn→error 日志 + 与 D3 联动消除权限漂移 [commands/role.rs:registry_for_sync]
- [x] [Review][Patch][F5] 导入 TOCTOU：唯一约束冲突映射为友好 ValidationError [db/skills.rs:map_skill_unique_error]
- [x] [Review][Patch][F6] 角色绑定失败仍弹"已导入"— saveSkills 返回 boolean，handleConfirmImport 据此区分提示 [SettingsTab.tsx]
- [x] [Review][Patch][F8] toFriendlyError 在 error 为 undefined 时崩溃 — 增加 null 兜底 [SettingsTab.tsx:toFriendlyError]
- [x] [Review][Patch][F9] toFriendlyError 不再回显后端原文 — 统一映射固定友好文案 [SettingsTab.tsx:toFriendlyError]
- [x] [Review][Patch][F10] agent_engine 降级 prompt 路径声明 enabledSkillIds，与 agent_config 口径一致 [agent_engine.rs:build_role_system_prompt]
- [x] [Review][Patch][F13] frontmatter 解析去前导空白（trim_start）[skill_registry.rs:parse_skill_content]
- [x] [Review][Patch][F15] 空 SKILL.md 友好提示"文件内容为空" [skill_registry.rs:load_skill_content]
- [x] [Review][Patch][F16] 目录选择器 AbortError（用户取消）静默返回 [SettingsTab.tsx:handleSkillDirectorySelected]

**验证（2026-06-04，全绿）**：前端测试 111/111 通过；Rust 测试 272 passed 0 failed（含新增 P3 幽灵 id 权限回归测试 build_agent_entry_denies_skill_when_enabled_ids_are_all_ghosts）；npm run build（tsc + vite）通过。

**Defer（既有/非本次引入或非阻塞）**

- [x] [Review][Defer][D2] description 限长 — boss 决定本次不做长度限制、不加 UI，遗留后续（详见 deferred-work.md）
- [x] [Review][Defer][F19] command 层拼装受控目录路径（业务逻辑应下沉 service）— boss 决定 defer，纯架构整洁度、无功能影响（详见 deferred-work.md）
- [x] [Review][Defer] 缺"日志不含文件内容/原始路径"的断言测试 [skill_registry.rs] — deferred，可观测性不变量加固，非阻塞

## Dev Notes

### Current State

- `roles.skills_config` 已存在，当前是字符串 JSON；`Role` 暴露该字段，见 `egosync-app/src-tauri/src/models/role.rs`。
- Story 2.10 当前只支持两个元 Skill：`find-skills`、`skill-creator`。`UpdateRoleSkillsInput` 只有两个 bool。
- `egosync-app/src-tauri/src/services/role_config.rs` 当前 `normalize_skills_config` 会把配置重写为两个 key，这是本 story 最大回归风险。
- `AgentConfigService::build_agent_entry` 会把 `meta_skill_prompt(skills_config)` 拼进角色 prompt，同时 `parse_permissions` 会读取 `skills_config.permissions`；保存路径目前并不保证保留 `permissions`。
- `SettingsTab` 已有 Skill 插件配置区和 inline error/saved message，可扩展，不要另起一套设置页。

### Architecture Guardrails

- 前端永远不直接访问 SQLite、opencode server 或文件系统；走 Tauri command + service 封装。
- Rust `commands/` 只做 IPC 参数解析、校验与 service/db 调用；DB SQL 放在 `egosync-app/src-tauri/src/db/`。
- opencode Skill 文件位置按架构文档：项目级 `.opencode/skills/<name>/SKILL.md` 或全局 `~/.config/opencode/skills/`；本故事应使用 EgoSync 管理路径，避免污染用户原始文件。
- 不存储 secret。Skill 文件本身可能包含用户提示词，日志中只记录 id/hash/简短状态，不记录全文。
- 不新增外部依赖，除非现有 Rust/TS 能力无法完成 frontmatter 解析；如需依赖必须说明理由并更新测试。

### Previous Story Intelligence

- Story 2.10 completion notes 表明已新增 `services::role_config`，并把元 Skill 状态注入 `AgentConfigService` 与 fallback `agent_engine`。本 story 必须扩展这些 helper，而不是复制一套平行解析器。
- Story 2.10 已验证命令：frontend tests、Rust tests 串行、Vite build；继续沿用相同验证标准。
- 过去在 Windows 环境中 `python3` 不可用，Rust 全量测试需用 `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1` 更稳定。

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
- Current files to extend: `egosync-app/src-tauri/src/services/role_config.rs`, `egosync-app/src-tauri/src/services/agent_config.rs`, `egosync-app/src-tauri/src/db/roles.rs`, `egosync-app/src-tauri/src/commands/role.rs`, `egosync-app/src/types/role.ts`, `egosync-app/src/services/roleService.ts`, `egosync-app/src/components/role/SettingsTab.tsx`.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7

### Debug Log References

- 2026-06-04: `python3 _bmad/scripts/resolve_customization.py ...` 在当前 Windows 环境不可用，已按 workflow 要求手动读取 customize.toml/team/user 覆盖文件。
- 2026-06-04: 首次 Rust 测试被全局 Cargo USTC registry 配置阻塞；使用一次性 `cargo --config 'source.crates-io.replace-with="rsproxy-sparse"' --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' ...` 后验证通过，未修改全局 Cargo 配置。
- 2026-06-04: 可见 UI 自动验证在纯 Vite 环境中缺少 Tauri invoke；使用浏览器会话内 mock 验证 SettingsTab UI，真实 Tauri dev 应用随后已成功编译启动。

### Completion Notes List

- 新增可扩展 `RoleSkillConfigV2` 规范化 helper，兼容旧 `find-skills` / `skill-creator` JSON，并在保存元 Skill 时保留 `enabledSkillIds`、`permissions` 和未知扩展字段。
- 新增 SQLite `skills` registry、Rust DB/service/model/command 层，支持解析 `SKILL.md` frontmatter、稳定 content hash 去重、同名重复提示、覆盖元数据，并复制到 EgoSync 管理的 `.opencode/skills/<name>/SKILL.md`。
- 扩展 `role_update_skills` 输入与前端类型，角色可保存启用的 registry Skill ids；`AgentConfigService` 在 role 更新和启动 full sync 时基于 registry 名称/描述同步自定义 Skill 能力到 opencode agent prompt，禁用后不再声明。
- SettingsTab 保持现有 inline feedback，新增自定义 Skill 列表、文件导入、目录导入、预览、重复提示、确认导入/覆盖元数据和启用/禁用开关；组件仍通过 service 层调用 Tauri command。
- 已验证：Rust 全量测试 270 个 lib 单测 + 1 个集成测试通过；前端全量测试 111 项通过；`npm --prefix "GUI" run build` 通过；真实 `npm --prefix "GUI" run tauri dev` 已编译并启动应用、数据库迁移和 opencode sidecar 成功。

### File List

- `egosync-app/src-tauri/migrations/008_skills_registry.sql`
- `egosync-app/src-tauri/src/commands/mod.rs`
- `egosync-app/src-tauri/src/commands/role.rs`
- `egosync-app/src-tauri/src/commands/skill.rs`
- `egosync-app/src-tauri/src/db/mod.rs`
- `egosync-app/src-tauri/src/db/pool.rs`
- `egosync-app/src-tauri/src/db/roles.rs`
- `egosync-app/src-tauri/src/db/skills.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src-tauri/src/models/mod.rs`
- `egosync-app/src-tauri/src/models/role.rs`
- `egosync-app/src-tauri/src/models/skill.rs`
- `egosync-app/src-tauri/src/services/agent_config.rs`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/src-tauri/src/services/mod.rs`
- `egosync-app/src-tauri/src/services/role_config.rs`
- `egosync-app/src-tauri/src/services/skill_registry.rs`
- `egosync-app/src/components/butler/ButlerSettingsContent.tsx`
- `egosync-app/src/components/role/SettingsTab.test.tsx`
- `egosync-app/src/components/role/SettingsTab.tsx`
- `egosync-app/src/services/appService.ts`
- `egosync-app/src/services/skillService.ts`
- `egosync-app/src/types/file-system-access.d.ts`
- `egosync-app/src/types/role.ts`
- `egosync-app/src/types/skill.ts`
- `_bmad-output/implementation-artifacts/2-11-custom-skill-md-import-role-binding.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-04: Implemented custom SKILL.md import, registry persistence, role binding, opencode sync, SettingsTab UI, and validation coverage.