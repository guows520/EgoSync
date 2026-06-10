---
baseline_commit: a4fb7c7bcec93dc708c190172eb9040215bedde1
---

# Story 2.10: 角色 Skill 配置与主动性 UI

Status: done

## Story

As a 用户,
I want 在角色设置中配置该角色可用的 V1 Skill 与主动性级别,
so that 每个角色的能力边界与主动协作程度都可被我明确控制，并在重启后保持一致。

## Acceptance Criteria

1. **Skill 配置入口**：在 `RoleView` → `SettingsTab` 中展示 “Skill 配置” 区块，包含两个 V1 默认元 Skill 开关：`find-skills`（Vercel 官方，用于发现/推荐可用 Skill）与 `skill-creator`（Anthropic 官方，用于创建/扩展 Skill），每个开关展示名称、来源与简短说明；管家设置页也展示同名 “Skill 配置” 区块。
2. **Skill 持久化**：切换角色 `find-skills` 或 `skill-creator` 后，写入当前角色的 `roles.skills_config` JSON；切换管家 Skill 后，写入 `app_settings` 的 `butler.skills_config`；应用重启后仍显示上次配置。
3. **Skill 生效边界**：角色与管家 Skill 配置必须同步到 opencode agent 配置或 system prompt 能力约束；禁用的元 Skill 不得因为 JSON 形状不匹配而被视为启用，也不得出现在 prompt 中被描述为可用。
4. **关闭 Skill 阻断**：当角色或管家在对话中收到发现/推荐/搜索 Skill 请求且 `find-skills=false`，或收到创建/扩展 Skill 请求且 `skill-creator=false`，后端必须在发送给 opencode 前返回温和边界提示，不调用 opencode `skill` 工具；当两个元 Skill 都关闭时，同步到 opencode agent permission 的 `skill: deny`。
5. **主动性配置 UI**：在 `SettingsTab` 中展示三档 `ProactivityToggle`：`静默执行`、`适度建议`、`积极主动`。
6. **主动性持久化**：切换主动性级别后写入当前角色的 `roles.proactivity_level`；应用重启后仍显示上次配置。
7. **主动性范围边界**：V1 仅存储与展示主动性级别；实际行为差异由 Epic 4 实现，本故事不得新增后台主动循环或建议生成逻辑。
8. **现有功能不回归**：角色基础设置保存、归档、删除、设置页打开/关闭、记忆引用跳转、source navigation、隐藏 thinking 内容等 Story 2.9 既有行为保持可用。

## Tasks / Subtasks

- [x] 后端：补齐角色 Skill 与主动性更新 API（AC: 2, 3, 6）
  - [x] 在 `GUI/src-tauri/src/models/role.rs` 增加专用输入 DTO：Skill 配置更新与主动性更新；保持 `#[serde(rename_all = "camelCase")]`。
  - [x] 在 `GUI/src-tauri/src/db/roles.rs` 增加只更新 `skills_config` 与只更新 `proactivity_level` 的 helper，返回完整 `Role`。
  - [x] 在 `GUI/src-tauri/src/commands/role.rs` 增加 Tauri command：`role_update_skills` 与 `role_update_proactivity`。
  - [x] 在 `GUI/src-tauri/src/lib.rs` 的 `tauri::generate_handler!` 注册新增 command。
  - [x] 更新后调用 `AgentConfigService::sync_role_updated(&role)`，沿用现有 `sync_warn(...)` 降级策略。

- [x] 后端：定义并验证 Skill JSON、元 Skill 生效映射与关闭阻断（AC: 2, 3, 4）
  - [x] 使用当前数据库字段 `roles.skills_config`，不要新增重复 migration 或重复列。
  - [x] UI 语义至少包含 `find-skills` 与 `skill-creator` 两个布尔值；解析失败时使用安全默认值并返回规范化 JSON。
  - [x] 明确 `skills_config` JSON 结构，例如 `{ "find-skills": true, "skill-creator": true }`；若 Rust/TypeScript 标识符需要下划线，可在边界层统一映射，不改变展示名。
  - [x] 确保 `GUI/src-tauri/src/services/agent_config.rs` 能把角色启用的元 Skill 写入 opencode agent 提示约束；禁用的元 Skill 不应在角色 prompt 中被描述为可用。
  - [x] 当两个元 Skill 都关闭时，角色与管家 opencode agent permission 写入 `skill: deny`；当任一元 Skill 仍启用时，不 blanket deny opencode 的通用 `skill` 工具。
  - [x] 文件读写、网页搜索属于底层 opencode tool permission，不作为本故事默认 Skill 主入口；除非实现元 Skill 需要，避免在 2.10 UI 暴露低层 tool 开关。
  - [x] 不在 `skills_config` 存 API key、token、路径密钥或其他 secret。

- [x] 后端：实现主动性级别校验（AC: 5, 6, 7）
  - [x] 使用现有 schema 枚举：`passive`、`moderate`、`proactive`。
  - [x] 不使用 epic 中过时的 `low / medium / high` 表述。
  - [x] 对非法值返回 `AppError`，不要让 SQLite CHECK 约束成为第一层用户可见错误。
  - [x] 默认值保持 `moderate`，对应 UI 文案 `适度建议`。

- [x] 前端：补齐类型与 service（AC: 1, 2, 5, 6）
  - [x] 在 `GUI/src/types/role.ts` 增加 Skill 配置与主动性更新输入类型。
  - [x] 在 `GUI/src/services/roleService.ts` 增加 `updateSkills(id, input)` 与 `updateProactivity(id, input)`，通过 Tauri `invoke()` 调用新增 command。
  - [x] 在 `GUI/src/services/appService.ts` 增加 `getButlerSkills()` 与 `updateButlerSkills(input)`，通过 Tauri `invoke()` 调用管家 Skill command。
  - [x] 保持组件不直接调用 `invoke()`；所有 IPC 仍走 service 层。

- [x] 前端：替换 SettingsTab 的 mock Skill 区块并补齐管家 Skill 配置（AC: 1, 2, 3, 8)
  - [x] 在 `GUI/src/components/role/SettingsTab.tsx` 解析 `role.skillsConfig`，展示两个真实元 Skill 开关：`find-skills`、`skill-creator`。
  - [x] 删除现有硬编码 API key 风格输入 `sk-xxxx-xxxx-xxxx-xxxx` 与 mock Skill 凭据 UI。
  - [x] 在 `GUI/src/components/butler/ButlerSettingsContent.tsx` 增加同名 “Skill 配置” 区块，读写管家 Skill 配置。
  - [x] Skill 开关切换时不显示额外 “保存中...” 行，只保留开关 pending 视觉和保存成功/失败反馈。
  - [x] 切换成功后调用 `onUpdateRole(updated)`，确保侧栏/头部/设置页持有最新 `Role`。
  - [x] 失败时使用设置页内联反馈；不要新增 toast/snackbar。
  - [x] 不改动归档/删除保护逻辑，保留仅剩一个 active role 时禁用危险操作的行为。

- [x] 前端：改造 ProactivityToggle 为真实受控组件（AC: 5, 6, 7, 8）
  - [x] 将 `GUI/src/components/role/ProactivityToggle.tsx` 从本地 mock state 改为受控 props：当前 level、变更回调、可选 pending/disabled 状态。
  - [x] `SettingsTab` 传入 `role.proactivityLevel`，切换后调用 `roleService.updateProactivity(...)`。
  - [x] 保持三档文案：`静默执行`、`适度建议`、`积极主动`。
  - [x] UI 只表达设置含义，不新增 Epic 4 的后台调度、建议卡片或通知逻辑。

- [x] 对话关闭 Skill 阻断路径（AC: 4）
  - [x] 先检查 `GUI/src-tauri/src/services/agent_engine.rs` 现有 opencode event/tool 处理，不发明前端无法接收的新事件协议。
  - [x] 关闭的元 Skill 不出现在角色或管家 system prompt 中，避免模型被提示为可用能力。
  - [x] 在发送给 opencode 前按用户意图硬阻断：发现/推荐/搜索 Skill 需要 `find-skills`；创建/扩展 Skill 需要 `skill-creator`。
  - [x] 阻断时直接完成当前 assistant message 并通过现有 `llm:stream` 事件返回边界提示，不调用 opencode 工具。
  - [x] 不破坏 Story 2.9 的 ChatStream source navigation、memory reference click、thinking 隐藏逻辑。

- [x] 测试与验证（AC: 1-8）
  - [x] 更新/新增 Rust 单元测试：`db::roles` Skill/proactivity 更新、非法主动性值、`AgentConfigService` 元 Skill 同步。
  - [x] 更新 `GUI/src/components/role/SettingsTab.test.tsx`：mock `roleService.updateSkills` / `updateProactivity`，覆盖两个元 Skill 开关与主动性切换持久化。
  - [x] 更新/新增 `ProactivityToggle` 测试，验证受控渲染与 onChange。
  - [x] 回归现有 SettingsTab 测试：保存角色基础信息、删除确认、仅剩一个 active role 时危险操作禁用。
  - [x] 至少运行：`cd GUI && npm run test:frontend`、`cd GUI/src-tauri && cargo test`、`cd GUI && npm run build`。
  - [x] 若修改可见 UI，启动应用并人工验证 SettingsTab golden path 与重启持久化；若无法启动，必须在 Dev Agent Record 写明原因。

## Dev Notes

### Current State

- `roles.skills_config` 与 `roles.proactivity_level` 已存在于 schema 中；本故事是接线、持久化、元 Skill 同步与 UI 替换，不是 schema 新增故事。
- `Role` Rust model 与 TypeScript `Role` 已暴露：`skillsConfig`、`proactivityLevel`。
- `UpdateRoleInput` 当前只覆盖基础角色信息，不覆盖 Skill/主动性。
- `SettingsTab` 当前 Skill 区块是 mock UI，包含 API key 风格硬编码字符串，必须替换为真实开关且不得存储 secret。
- `ProactivityToggle` 当前只维护本地 state，刷新/重启不会持久化。
- `AgentConfigService` 当前主要处理 opencode permission；本故事需要补齐角色启用元 Skill 与 opencode agent/prompt 的同步关系，避免 `find-skills` / `skill-creator` 只存在于 UI JSON 中。

### Architecture Guardrails

- 前端不得直接访问 SQLite、opencode server 或 LLM API；Tauri IPC 必须经 `GUI/src/services/*Service.ts`。
- Rust `commands/` 只做 IPC、参数校验与 service/db 调用；业务规则不要塞进 command 大函数。
- Rust DB SQL 保持在 `GUI/src-tauri/src/db/`。
- Rust command 返回 `Result<T, AppError>`，不要在 command path 使用 `.unwrap()`。
- opencode agent 配置同步由 `GUI/src-tauri/src/services/agent_config.rs` 负责；角色更新后使用既有 best-effort sync，不因 opencode 配置写入失败阻塞角色 CRUD。
- TypeScript strict mode、`noUnusedLocals`、`noUnusedParameters` 均开启；不要留下未用类型、变量或 mock。
- Tailwind utility classes only；不要新增 CSS 文件。
- 不新增依赖；React、Tauri、SQLx、serde_json 已足够。

### Existing Files to Touch

- `GUI/src-tauri/src/models/role.rs`
  - Current `Role` includes `skills_config: String` and `proactivity_level: String`.
  - Add focused input DTOs rather than overloading unrelated UI fields ambiguously.

- `GUI/src-tauri/src/db/roles.rs`
  - `ROLE_SELECT_COLUMNS` already selects both fields.
  - Existing `update_role()` only updates name/icon/color/goal/personality_prompt; add focused update helpers.

- `GUI/src-tauri/src/commands/role.rs`
  - Existing `role_update` validates name and syncs agent config after DB update.
  - New Skill/proactivity commands should follow this pattern and call `sync_warn(agent_config.sync_role_updated(&role), ...)`.

- `GUI/src-tauri/src/services/agent_config.rs`
  - `build_agent_entry(role)` currently builds prompt and permission from `role.skills_config`.
  - Update tests around invalid JSON, explicit permissions if still supported, and enabled/disabled meta Skill propagation for `find-skills` / `skill-creator`.

- `GUI/src-tauri/src/lib.rs`
  - Register `role_update_skills` and `role_update_proactivity` in `tauri::generate_handler!`.

- `GUI/src/types/role.ts`
  - `Role.proactivityLevel` already uses `passive | moderate | proactive`.
  - Add Skill config/update types matching camelCase IPC payloads.

- `GUI/src/services/roleService.ts`
  - Add service methods; components must not import Tauri `invoke()` directly.

- `GUI/src/components/role/SettingsTab.tsx`
  - Replace mock Skill UI with real switches.
  - Keep existing save/archive/delete behavior intact.
  - Use inline pending/error feedback inside the settings panel.

- `GUI/src/components/role/ProactivityToggle.tsx`
  - Convert to controlled component.
  - Preserve labels and visual style unless implementation requires small accessibility improvements.

- `GUI/src/components/role/SettingsTab.test.tsx`
  - Extend existing tests; do not move SettingsTab behavior into `RoleWorkspacePanel.test.tsx`, because that test currently mocks SettingsTab.

### Regression Risks

- **Duplicate schema work**: Do not add duplicate `skills_config` / `proactivity_level` columns.
- **Enum mismatch**: Do not implement `low / medium / high`; schema accepts only `passive / moderate / proactive`.
- **Meta Skill false positive**: A UI toggle that persists JSON but does not change role agent config/prompt availability for `find-skills` / `skill-creator` does not satisfy AC 3.
- **Secret leakage**: Do not store Skill API keys or provider credentials in role Skill config. Secrets remain in keyring/global provider settings.
- **Story 2.9 regression**: Avoid broad rewrites of `ChatStream`, `RoleView`, `MemoryTab`, or source navigation state unless directly required for AC 4.
- **Scope creep**: Do not implement Epic 4 background scheduler, proactive suggestions, notification tiers, or dashboard behavior.

### Testing Standards

- Frontend: Vitest + React Testing Library under `GUI/src/**/*.test.tsx`.
- Rust: `cargo test` under `GUI/src-tauri`.
- Build validation: `cd GUI && npm run build`.
- UI validation is required for visible UI changes when feasible: open Settings tab, toggle `find-skills` / `skill-creator`, change proactivity, restart app, confirm values persist.

### Project Structure Notes

- Existing role UI is under `GUI/src/components/role/`; keep new UI there.
- Existing role IPC service is `GUI/src/services/roleService.ts`; extend it rather than creating a parallel service.
- Existing role persistence is under `GUI/src-tauri/src/db/roles.rs`; extend it rather than embedding SQL in commands.
- No `GUI/src/hooks/useRoles.ts` exists; do not reference or create it unless a separate refactor is explicitly requested.

### References

- Story requirements: `_bmad-output/planning-artifacts/epics.md` → Story 2.10 Role Skill Config + Proactivity UI.
- PRD requirements: `_bmad-output/planning-artifacts/prd-egosync.md` → FR-4b, FR-12, FR-31..FR-36.
- Architecture rules: `_bmad-output/planning-artifacts/architecture.md` → frontend/Rust/opencode layering, role-agent mapping, permissions, testing.
- UX rules: `_bmad-output/planning-artifacts/ux-design-specification.md` → Settings tab Skill config, proactivity, in-world feedback, Tailwind-only UI.
- Existing schema: `GUI/src-tauri/migrations/003_roles.sql` → `skills_config`, `proactivity_level`.
- Existing role model: `GUI/src-tauri/src/models/role.rs`.
- Existing role DB updates: `GUI/src-tauri/src/db/roles.rs`.
- Existing role commands: `GUI/src-tauri/src/commands/role.rs`.
- Existing opencode config mapping: `GUI/src-tauri/src/services/agent_config.rs`.
- Existing settings UI: `GUI/src/components/role/SettingsTab.tsx`.
- Existing proactivity mock: `GUI/src/components/role/ProactivityToggle.tsx`.
- Previous story regression context: `_bmad-output/implementation-artifacts/2-9-reasoning-transparency-uncertainty.md`.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.8 (Claude Code)

### Debug Log References

- `python3` 在当前 Windows 环境不可用，已按 BMAD workflow 说明手工读取 `customize.toml` 与项目配置完成初始化。
- 初次在项目根目录执行 Rust 测试时未找到 `Cargo.toml`，已改用 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml"`。
- 全局 Cargo registry 指向失效 USTC 镜像，已使用项目本地 `GUI/.cargo-home-local` 执行 Rust 验证。
- Windows 并行 Rust 全量测试中 `services::sidecar::tests::test_start_rejects_preexisting_healthy_listener` 曾出现环境不稳定；该测试单独通过，串行全量 Rust 测试通过。

### Completion Notes List

- 增加角色 Skill 与主动性专用更新 DTO、DB helper、Tauri command 与 command 注册，保持组件 IPC 只经 `roleService`。
- 新增 `services::role_config` 统一规范化 `find-skills` / `skill-creator` JSON、主动性枚举校验、元 Skill prompt 注入文本与确认意图识别。
- `AgentConfigService` 与 `agent_engine` 均注入元 Skill 状态约束，避免 UI JSON 与实际角色/管家能力边界脱节。
- 聊天发送路径增加元 Skill 关闭硬阻断：角色与管家在 `find-skills=false` 时不会执行发现/推荐/搜索 Skill 请求，在 `skill-creator=false` 时不会执行创建/扩展 Skill 请求。
- `SettingsTab` 替换 mock Skill/API key UI，展示两个默认元 Skill 开关和来源，区块标题为 “Skill 配置”；`ButlerSettingsContent` 也新增同名管家 Skill 配置；`ProactivityToggle` 改为受控组件并持久化三档主动性。
- 已运行验证：`npm --prefix "GUI" run test:frontend`（14 个测试文件、103 个测试通过）、`cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`（241 个 lib 测试 + 1 个 integration 测试通过）、`npm --prefix "GUI" run build`（TypeScript + Vite 构建通过）。后续回归补充：`cargo test --manifest-path "GUI/src-tauri/Cargo.toml" agent_config`（21 passed）、`cargo test --manifest-path "GUI/src-tauri/Cargo.toml" meta_skill`（17 passed）、完整 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml"`（254 个 lib 测试 + 1 个 integration 测试通过）、`cargo check --manifest-path "GUI/src-tauri/Cargo.toml"` 通过。
- 可见 UI 验证：已启动 `npm dev` 并通过浏览器确认本地页面可访问；随后启动 Tauri 桌面壳。编译完成后应用正常运行，数据库迁移、opencode sidecar 与 delegate bridge 均启动成功，系统检测到窗口标题 `EgoSync`。后续回归中已重启 Tauri，确认 Vite `5173` 与 opencode `4096` 就绪，并检查运行态 opencode agent：关闭的 `find-skills` 不出现在“父亲”角色 prompt 中；两个元 Skill 都关闭的角色写入 `permission.skill = deny`。

### File List

- `_bmad-output/implementation-artifacts/2-10-role-skill-config-proactivity-ui.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `GUI/src-tauri/src/commands/app.rs`
- `GUI/src-tauri/src/commands/role.rs`
- `GUI/src-tauri/src/commands/chat.rs`
- `GUI/src-tauri/src/db/roles.rs`
- `GUI/src-tauri/src/lib.rs`
- `GUI/src-tauri/src/models/role.rs`
- `GUI/src-tauri/src/services/mod.rs`
- `GUI/src-tauri/src/services/role_config.rs`
- `GUI/src-tauri/src/services/butler_config.rs`
- `GUI/src-tauri/src/services/agent_config.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src/services/appService.ts`
- `GUI/src/services/roleService.ts`
- `GUI/src/types/role.ts`
- `GUI/src/components/butler/ButlerSettingsContent.tsx`
- `GUI/src/components/role/ProactivityToggle.tsx`
- `GUI/src/components/role/SettingsTab.tsx`
- `GUI/src/components/role/SettingsTab.test.tsx`
- `GUI/src/components/role/ProactivityToggle.test.tsx`

### Change Log

- 2026-06-02: Implemented Story 2.10 role meta Skill configuration, proactivity persistence, prompt synchronization, confirmation-based Skill enablement, and validation coverage.
- 2026-06-03: Refreshed implementation to match actual runtime behavior: renamed UI to “Skill 配置”, added butler Skill configuration, hid disabled meta Skills from prompts, blocked disabled role/butler meta Skill intents before opencode, protected all-disabled agents with `permission.skill = deny`, and verified Tauri runtime readiness.
