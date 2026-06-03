# Story 2.12: 用户能发现并导入 opencode 生态 Skill

Status: ready-for-dev

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

- [ ] 复用 Skill registry 与配置 schema（AC: 2, 4, 5）
  - [ ] 依赖 Story 2.11 的 registry 表/配置模型；不要创建第二套 registry
  - [ ] sourceType 使用 `opencode`，与 `custom` 明确区分
  - [ ] 重复检测复用 name + content hash 规则

- [ ] 实现 opencode Skill 目录扫描服务（AC: 2, 3, 6）
  - [ ] 扫描项目级 `.opencode/skills/*/SKILL.md`
  - [ ] 扫描全局 `~/.config/opencode/skills/*/SKILL.md`；Windows 也按 opencode 实际使用的 `~/.config/opencode/` 路径处理
  - [ ] 每个候选项解析 frontmatter；无效项进入 skipped summary，不进入导入列表
  - [ ] 不递归扫描任意用户目录，避免性能和隐私风险

- [ ] 新增发现/导入 Tauri command 与 service 封装（AC: 1, 2, 4）
  - [ ] Rust command 示例：`skill_discover_opencode`、`skill_import_opencode`
  - [ ] 如果当前角色未启用 `find-skills`，后端返回可区分的 validation error，前端显示启用提示
  - [ ] 前端新增 `skillService.ts` 或扩展既有 service；组件不得直接调用 `invoke()`

- [ ] SettingsTab 接入发现列表 UI（AC: 1, 2, 3, 4）
  - [ ] 在 Skill 区域增加“发现 opencode Skill”按钮
  - [ ] 展示扫描结果、已导入状态、跳过摘要、导入按钮
  - [ ] 导入成功后刷新 registry 列表并允许立即启用到当前角色
  - [ ] 使用 inline feedback，不新增全局 toast

- [ ] 同步角色 agent 能力边界（AC: 5）
  - [ ] 更新 `AgentConfigService::build_agent_entry` 或相关 helper，使启用的 opencode Skill 被反映到 agent 配置/prompt
  - [ ] 禁用后重新同步，不留下旧 prompt 声明
  - [ ] `full_sync` 必须保持导入 Skill 与角色绑定一致

- [ ] 测试与验证（AC: 1-7）
  - [ ] Rust 单测：目录扫描、无效项跳过、重复检测、find-skills 未启用阻断、导入 registry 写入
  - [ ] 前端测试：未启用提示、扫描结果渲染、跳过摘要、导入后可启用
  - [ ] 回归 2.10/2.11 相关测试
  - [ ] 运行 `npm --prefix "GUI" run test:frontend`
  - [ ] 运行 `cargo test --manifest-path "GUI/src-tauri/Cargo.toml" -- --test-threads=1`
  - [ ] 运行 `npm --prefix "GUI" run build`

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

## Dev Agent Record

### Agent Model Used

TBD by dev agent

### Debug Log References

### Completion Notes List

### File List
