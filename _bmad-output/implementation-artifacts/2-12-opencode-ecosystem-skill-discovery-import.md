---
baseline_commit: 52becea46840e37c8d1c516cda2100f631e9f153
---

# Story 2.12: 用户能发现并导入 opencode 生态 Skill

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 从 opencode 可发现的 Skill 目录中扫描并导入第三方 Skill,
so that 我能复用 opencode 生态能力，同时仍由 EgoSync 管理每个角色启用什么。

## Acceptance Criteria

1. **发现入口受元 Skill 边界控制**
   - Given 用户未启用 `find-skills`
   - When 用户尝试发现 opencode Skill
   - Then 设置页提示“需要先启用 find-skills 才能发现可用 Skill”并提供启用入口
   - And 不自动扫描或导入任何 Skill

2. **扫描 opencode Skill 目录**
   - Given 用户已启用 `find-skills`
   - When 在 Skill 插件配置中点击“发现 Skill”
   - Then 系统扫描 opencode 项目级与全局 Skill 目录（如 `.opencode/skills/`、`~/.config/opencode/skills/`）
   - And 列表展示每个可导入 Skill 的 `name`、`description`、来源位置、sourceType=`opencode`、是否已导入

3. **跳过无效 Skill**
   - Given 扫描目录中存在无效条目（缺少 `SKILL.md`、frontmatter 缺少 name/description、文件不可读）
   - When 扫描完成
   - Then 无效项不进入可导入列表
   - And 设置页以友好中文说明跳过数量或原因，不暴露底层堆栈

4. **导入第三方 opencode Skill**
   - Given 用户从扫描结果中选择一个 Skill
   - When 确认导入
   - Then Skill 被复制或登记到 EgoSync Skill registry，sourceType=`opencode`
   - And 用户可立即在当前角色启用该 Skill
   - And 应用重启后导入状态和角色启用状态保留

5. **角色可用性同步**
   - Given 第三方 Skill 已导入并启用到角色
   - When 角色下一轮对话开始
   - Then 角色 opencode agent 能自动发现/加载该 Skill
   - And 禁用后角色不得再声明自己拥有该 Skill

6. **V1 明确不做远程市场**
   - Given 用户想从 URL 或远程市场导入 Skill
   - Then V1 不自动下载未知 URL、不实现账号登录、不实现付费/评分/远程市场
   - And 用户必须先把远程获得的 Skill 保存为本地 `SKILL.md`，再走 Story 2.11 导入路径

7. **现有功能不回归**
   - 自定义 Skill 导入（Story 2.11）、默认元 Skill 开关（Story 2.10）、角色设置保存、普通对话和 opencode custom tools 均保持可用

## Tasks / Subtasks

- [x] 复用 Skill registry 与配置 schema（AC: 2, 4, 5）
  - [x] 依赖 Story 2.11 的 registry 表/配置模型；不要创建第二套 registry
  - [x] sourceType 使用 `opencode`，与 `custom` 明确区分
  - [x] 重复检测复用 name + content hash 规则

- [x] 实现 opencode Skill 目录扫描服务（AC: 2, 3, 6）
  - [x] 扫描项目级 `.opencode/skills/*/SKILL.md`
  - [x] 扫描全局 `~/.config/opencode/skills/*/SKILL.md`；Windows 也按 opencode 实际使用的 `~/.config/opencode/` 路径处理
  - [x] 每个候选项解析 frontmatter；无效项进入 skipped summary，不进入导入列表
  - [x] 不递归扫描任意用户目录，避免性能和隐私风险

- [x] 新增发现/导入 Tauri command 与 service 封装（AC: 1, 2, 4）
  - [x] Rust command 示例：`skill_discover_opencode`、`skill_import_opencode`
  - [x] 如果当前角色未启用 `find-skills`，后端返回可区分的 validation error，前端显示启用提示
  - [x] 前端新增 `skillService.ts` 或扩展既有 service；组件不得直接调用 `invoke()`

- [x] SettingsTab 接入发现列表 UI（AC: 1, 2, 3, 4）
  - [x] 在 Skill 区域增加“发现 opencode Skill”按钮
  - [x] 展示扫描结果、已导入状态、跳过摘要、导入按钮
  - [x] 导入成功后刷新 registry 列表并允许立即启用到当前角色
  - [x] 使用 inline feedback，不新增全局 toast

- [x] 同步角色 agent 能力边界（AC: 5）
  - [x] 更新 `AgentConfigService::build_agent_entry` 或相关 helper，使启用的 opencode Skill 被反映到 agent 配置/prompt
  - [x] 禁用后重新同步，不留下旧 prompt 声明
  - [x] `full_sync` 必须保持导入 Skill 与角色绑定一致

- [x] 测试与验证（AC: 1-7）
  - [x] Rust 单测：目录扫描、无效项跳过、重复检测、find-skills 未启用阻断、导入 registry 写入
  - [x] 前端测试：未启用提示、扫描结果渲染、跳过摘要、导入后可启用
  - [x] 回归 2.10/2.11 相关测试
  - [x] 运行 `npm --prefix "GUI" run test:frontend`
  - [x] 运行 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`
  - [x] 运行 `npm --prefix "GUI" run build`

## Dev Notes

### Current State

- 目前没有 opencode ecosystem Skill 发现 UI 或扫描 command。
- Story 2.10 的 `find-skills` 只是元能力开关，当前没有实际扫描实现。
- Story 2.11 应提供可扩展 Skill registry 和角色绑定结构；本 story 必须复用该底座。
- `agent_config.rs::write_custom_tools` 已确认 Windows 上 opencode 使用 `~/.config/opencode/`，不是 `%APPDATA%`。

### Architecture Guardrails

- V1 的“opencode 生态”定义为本机 opencode 已可见的 Skill 目录，不是远程 marketplace。
- 不生成或猜测任何远程 URL，不自动下载未知内容。
- 扫描范围必须窄：只扫约定 Skill 目录，不扫用户 home 全量目录。
- 扫描结果不能把 Skill 正文发给 LLM 或日志；UI 只展示 name/description/source summary。
- 继续遵守 Tauri 三层：前端 service → command → Rust service/db。

### Previous Story Intelligence

- Story 2.0d 证明 opencode 受 project instance 影响，MCP 工具可见性曾出现偏差；Skill 扫描也应优先使用 opencode 实际全局路径和 EgoSync 受控路径，避免假设 cwd。
- Story 2.10 已把元 Skill 状态注入 prompt；本 story 不要新增另一套“find-skills 是否启用”的判断逻辑，应复用 role_config helper。
- Recent commits集中在 memory/source navigation，说明当前代码偏好小步接线、强测试回归；保持同样粒度。

### Regression Risks

- **误做远程市场**：本 story 不做下载、评分、登录、付费。
- **扫描过宽**：不要递归 home 或项目根所有目录。
- **导入重复**：不能让同一个 opencode Skill 多次导入成多个不可区分条目。
- **find-skills 失效**：未启用时必须阻断发现路径，否则 2.10 的能力边界变成假 UI。

### References

- PRD FR-4b：`_bmad-output/planning-artifacts/prd-egosync.md` → opencode 生态第三方 Skills、Agent 自动发现。
- Story 2.11：`_bmad-output/implementation-artifacts/2-11-custom-skill-md-import-role-binding.md`。
- Story 2.10：`_bmad-output/implementation-artifacts/2-10-role-skill-config-proactivity-ui.md`。
- Story 2.0d：`_bmad-output/implementation-artifacts/2-0d-opencode-custom-tools-replace-mcp.md` → opencode 路径和 instance 经验。
- Architecture Skill 体系：`_bmad-output/planning-artifacts/architecture.md` → `.opencode/skills/<name>/SKILL.md` 与 `~/.config/opencode/skills/`。
- Current files likely to extend: `GUI/src-tauri/src/services/role_config.rs`, `GUI/src-tauri/src/services/agent_config.rs`, `GUI/src/components/role/SettingsTab.tsx`, `GUI/src/services/roleService.ts` or new `skillService.ts`.

### Review Findings

_Code review 2026-06-09（Blind Hunter + Edge Case Hunter + Acceptance Auditor 三层对抗式评审）_

#### Resolved Decisions

- [x] [Review][Decision] (Decision #1 — 用户裁决 2026-06-09：选项 1「复制到受控目录」) opencode 导入的 `managed_path` 应指向 EgoSync 受控副本而非外部源文件 → 已转为下方 Patch「opencode 导入复制到受控目录」。`content_hash` 全局唯一模型保持现状（用户未选改 `(content_hash, source_type)`）→ 已转为下方 Deferred。

#### Patch

_全部 10 项已修复并验证（2026-06-09）：vitest 15 files/146 tests、cargo test 305+1、`npm run build` 均通过。_

- [x] [Review][Patch] opencode 导入复制 SKILL.md 到 EgoSync 受控目录（`managed_skill_path` + `create_dir_all` + `std::fs::write`），`managed_path` 指向副本（即 opencode 项目级 skills 根，agent 可自动发现），源文件删/改后仍可用，保证 AC4/AC5；测试 `import_opencode_skill_registers_source_type_and_applies_scope` 已改为断言 `managed_path != source_path`、副本存在且删源后仍存在 [GUI/src-tauri/src/services/skill_registry.rs:321-371]
- [x] [Review][Patch] full_sync 前置查询改用 `?` 而非 `unwrap_or_default()`，DB 瞬时错误直接失败，绝不以空集触发破坏性全量同步 [GUI/src-tauri/src/commands/skill.rs:112-114]
- [x] [Review][Patch] `ImportOpencodeSkillInput` 新增 `expected_content_hash`，import 时比对发现时的 hash，源文件被替换则拒绝并提示重新发现（TOCTOU）；新增测试 `import_opencode_skill_rejects_when_source_changed_after_discover` [GUI/src-tauri/src/services/skill_registry.rs:336-344]
- [x] [Review][Patch] `create_skill_with_source` 撞唯一索引时捕获 `DbError` 并重新查重，退化为优雅 duplicate（提取 `finalize_opencode_binding` 收口绑定逻辑）[GUI/src-tauri/src/services/skill_registry.rs:354-376]
- [x] [Review][Patch] `scan_opencode_root` 对任何解析错误（含非 ValidationError）均 `continue` 跳过当前条目，绝不中断整个扫描；新增 `skip_reason_for`/`parsed_skill_name` 为跳过摘要附带 skill name [GUI/src-tauri/src/services/skill_registry.rs:297-304]
- [x] [Review][Patch] `entries.flatten()` 改为显式 `match entry`，不可读目录项也 push reason，跳过统计准确 [GUI/src-tauri/src/services/skill_registry.rs:277-283]
- [x] [Review][Patch] UI 跳过摘要改为 `<ul>` 逐条列出（reason 已带 skill name），超过 5 条折叠计数，可定位具体被跳过 Skill [GUI/src/components/role/SettingsTab.tsx:550-561]
- [x] [Review][Patch] `handleImportOpencode` 入口预检 `skills.findSkills`，未启用时给出与发现一致的引导提示 [GUI/src/components/role/SettingsTab.tsx:handleImportOpencode]
- [x] [Review][Patch] discover 不再复用 `settingsSavedMessage` 承载跳过提示（改由独立 `opencodeSkipped` 区块），import 结果提示不会覆盖跳过信息 [GUI/src/components/role/SettingsTab.tsx:handleDiscoverOpencode]
- [x] [Review][Patch] `ImportOpencodeSkillResult` 新增 `synced` 字段，command 层 full_sync 失败时置 false，前端据此显示「已导入，但同步暂时失败，将在下次同步自动生效」而非谎称「已启用」[GUI/src-tauri/src/commands/skill.rs:117-121, GUI/src/components/role/SettingsTab.tsx]

#### Deferred

- [x] [Review][Defer] duplicate 分支 `replace_bindings` 单角色 scope 会 DELETE 该 Skill 全部绑定再只重插当前角色，静默解绑其它角色 [GUI/src-tauri/src/services/skill_registry.rs:492-501] — deferred, pre-existing（2.11 `import_custom_skill` 使用完全相同模式，非本次引入，应作为统一 binding 语义问题单独处理）
- [x] [Review][Defer] async 命令内使用阻塞 `std::fs::read_dir/read_to_string`，慢盘/大目录会阻塞 tokio 工作线程 [GUI/src-tauri/src/services/skill_registry.rs:427-449] — deferred, pre-existing（既有 skill_registry 同步 I/O 模式一致，建议统一迁移到 spawn_blocking）
- [x] [Review][Defer] `read_dir` 因权限失败时静默 `return Ok(())`，与「目录不存在」同等处理，用户无任何「目录不可扫描」反馈 [GUI/src-tauri/src/services/skill_registry.rs:427] — deferred, pre-existing（低概率边界，可与扫描可观测性增强一并处理）
- [x] [Review][Defer] `content_hash` 全局 UNIQUE 不分 source_type，内容相同的 opencode Skill 会被误判为某 custom Skill 的 duplicate（返回 `entry.source_type='custom'`）[GUI/src-tauri/src/services/skill_registry.rs:preview_from_parsed_with_source, migrations/010:24] — deferred, 用户裁决（Decision #1 选项 1，未选改 `(content_hash, source_type)`）：V1 同内容跨源场景极罕见，保持全局唯一

## Dev Agent Record

### Agent Model Used

Claude Opus 4.8 (Claude Code)

### Debug Log References

- 2026-06-08: `python3` 在当前 Windows 环境不可用，已使用 `python` 解析 BMad workflow/agent customization。
- 2026-06-08: 默认 Cargo registry 指向不可用 USTC 镜像；Rust 验证使用项目本地 `GUI/.cargo-home-local` 与一次性 `rsproxy` sparse registry override，未修改全局 Cargo 配置。
- 2026-06-08: TDD red tests 先失败于缺少 `create_skill_with_source` / `find_skill_by_name_and_source` / `discover_opencode_skills` 与前端“发现 opencode Skill”入口，随后按最小实现补齐。
- 2026-06-08: 迁移兼容 red test 暴露 `010` 重建 `skills` 表时会清空 `skill_role_bindings`；已通过临时备份/恢复绑定修复并验证。

### Completion Notes List

- 复用 Story 2.11 的 `skills` registry、`skill_role_bindings` 和 `enabledSkillIds` 配置链路，新增 `sourceType="opencode"`，通过追加 `010_skills_opencode_source_type.sql` 扩展 schema，保留 `custom` 兼容。
- 新增 opencode Skill discovery/import 后端能力：只扫描 EgoSync opencode workspace 的 `.opencode/skills/*/SKILL.md` 与用户 home 下 `~/.config/opencode/skills/*/SKILL.md`，不递归任意目录，不下载 URL，不读取远程市场。
- 无效 Skill（缺少 `SKILL.md`、frontmatter 不完整、不可读取）进入 skipped summary，不进入可导入列表；扫描结果只返回 name/description/source summary/sourceType/imported 状态。
- 新增 `skill_discover_opencode` / `skill_import_opencode` Tauri command 与 `skillService.discoverOpencode/importOpencode`；当前角色未启用 `find-skills` 时后端返回 validation error，前端显示启用入口且不触发扫描。
- SettingsTab 新增 “opencode 生态 Skill” 区块，展示扫描结果、已导入状态、跳过摘要与导入按钮；导入后刷新当前角色 registry 列表与角色 `enabledSkillIds`，inline feedback 无全局 toast。
- `AgentConfigService` 继续按 registry id 注入启用 Skill 的 name/description；opencode sourceType 走同一 registry，因此 role update/full sync 禁用后不会声明旧 Skill。
- 已验证：`npm --prefix "GUI" run test:frontend`（15 files / 146 tests 通过）；`cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`（304 lib tests + 1 integration test 通过）；`npm --prefix "GUI" run build` 通过。

### File List

- `GUI/src-tauri/migrations/010_skills_opencode_source_type.sql`
- `GUI/src-tauri/src/commands/skill.rs`
- `GUI/src-tauri/src/lib.rs`
- `GUI/src-tauri/src/db/pool.rs`
- `GUI/src-tauri/src/db/skills.rs`
- `GUI/src-tauri/src/models/skill.rs`
- `GUI/src-tauri/src/services/skill_registry.rs`
- `GUI/src/components/role/SettingsTab.test.tsx`
- `GUI/src/components/role/SettingsTab.tsx`
- `GUI/src/services/skillService.ts`
- `GUI/src/types/skill.ts`
- `_bmad-output/implementation-artifacts/2-12-opencode-ecosystem-skill-discovery-import.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-08: Implemented opencode ecosystem Skill discovery/import, registry sourceType extension, SettingsTab discovery UI, role binding sync, and validation coverage.