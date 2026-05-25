# Story 2.4: 每个角色用符合身份的个性化语调回应

Status: review

## Story

As a 用户,
I want 每个角色的回复风格与其身份匹配,
so that 不同角色之间有明显区分感，对话更自然。

## Acceptance Criteria

1. **AC-1 角色回复体现身份语调**
   - **Given** 用户向「产品经理」角色提问
   - **When** 角色回复
   - **Then** 语调简洁专业，偏结构化表达
   - **And** 不自称管家，不退回通用助理口吻

2. **AC-2 不同角色对相同问题有可感知差异**
   - **Given** 用户向「产品经理」与「家庭」角色提问相同问题
   - **When** 两个角色分别回复
   - **Then** 产品经理偏结构化、专业判断；家庭角色偏温暖、关怀、情感支持
   - **And** 差异来自角色 system prompt，不靠前端展示伪装

3. **AC-3 设置页可编辑角色个性描述**
   - **Given** 用户打开角色视图的 SettingsTab
   - **When** 编辑「角色个性描述」多行文本并保存
   - **Then** `role_update` 持久化 `personalityPrompt`
   - **And** 保存后当前 RoleView 立即使用更新后的角色对象
   - **And** 下次角色对话立即反映新语调

4. **AC-4 System Prompt 三层分离**
   - **Given** Rust 后端组装角色对话 messages
   - **Then** system prompt 按三层顺序拼接：`base_persona` + `role_definition` + `context_injection`
   - **And** 三层在代码中边界清晰，可单独测试
   - **And** `base_persona` 不复用 `BUTLER_SYSTEM_PROMPT`

5. **AC-5 预设语调模板可作为编辑参考**
   - **Given** 用户进入 SettingsTab
   - **Then** 显示预设语调模板参考：产品经理=简洁专业、家庭=温暖关怀、学习者=好奇探索
   - **And** 用户可自由覆盖，不强制套用模板

6. **AC-6 数据层兼容既有角色**
   - **Given** 既有角色的 `personality_prompt` 为空
   - **When** 角色列表、更新、对话构建执行
   - **Then** 不报错，不需要数据回填
   - **And** 新建角色默认 `personality_prompt=''`

7. **AC-7 测试通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd GUI/src-tauri && cargo test`
   - 至少覆盖：`UpdateRoleInput.personality_prompt` 持久化、`build_role_messages` 三层拼接与身份隔离、SettingsTab 保存 `personalityPrompt`。

## Tasks / Subtasks

### Phase 1: 数据契约补全（AC: #3, #6）

- [x] T1.1 `GUI/src-tauri/src/models/role.rs`：`UpdateRoleInput` 新增 `personality_prompt: Option<String>`
- [x] T1.2 `GUI/src-tauri/src/db/roles.rs`：`update_role` 的 SQL 增加 `personality_prompt = COALESCE(?5, personality_prompt)`，并顺延 bind 序号
- [x] T1.3 `GUI/src-tauri/src/commands/role.rs`：无需新增 command；沿用 `role_update`
- [x] T1.4 `GUI/src/types/role.ts`：`UpdateRoleInput` 新增 `personalityPrompt?: string`
- [x] T1.5 `GUI/src/services/roleService.ts`：无需新增 service；沿用 `roleService.update`

### Phase 2: Prompt 三层分离（AC: #1, #2, #4, #6）

- [x] T2.1 `GUI/src-tauri/src/services/agent_engine.rs`：保留角色身份与管家身份隔离，不把 `BUTLER_SYSTEM_PROMPT` 拼入角色 prompt
- [x] T2.2 将现有角色 prompt 拆成三段语义：
  - `base_persona`：EgoSync 内在维度身份、非管家、中文简洁自然
  - `role_definition`：角色名称、目标、个性描述；个性描述为空时跳过
  - `context_injection`：当前 story 暂不引入记忆，仅保留历史 messages 注入；不要提前实现 Story 2.6 记忆
- [x] T2.3 增加默认语调提示：当 `personality_prompt` 为空时，根据角色名称/目标给 LLM 一个轻量方向；不得写确定性路由或复杂分类逻辑
- [x] T2.4 单测验证 system prompt 包含角色名、目标、个性描述，且不包含「数字管家」身份

### Phase 3: SettingsTab 编辑入口（AC: #3, #5）

- [x] T3.1 `GUI/src/components/role/SettingsTab.tsx`：新增本地状态 `rolePersonalityPrompt`，随 `role.id`/`role.personalityPrompt` 同步
- [x] T3.2 在「角色信息」区域新增「角色个性描述」textarea
  - placeholder 解释用途：影响该角色下次回复风格
  - 显示三条模板参考：产品经理、家庭、学习者
  - 不自动覆盖用户输入
- [x] T3.3 `handleSave` 调用 `roleService.update` 时带上 `personalityPrompt: rolePersonalityPrompt.trim()`
- [x] T3.4 保留现有保存、错误、归档、删除状态机；不得改动危险区域行为

### Phase 4: 测试与验证（AC: #7）

- [x] T4.1 `GUI/src-tauri/src/db/roles.rs`：新增/更新单测，证明 `personality_prompt` 可通过 `update_role` 修改且读回
- [x] T4.2 `GUI/src-tauri/src/services/agent_engine.rs`：新增/更新单测，证明 `build_role_messages` 三层拼接、身份隔离、个性描述为空可用
- [x] T4.3 前端测试：补 SettingsTab 保存 payload 覆盖 `personalityPrompt`；如现有测试缺失，可在最小范围内新增 co-located 测试
- [x] T4.4 运行 AC-7 三条命令；若桌面端仍需人工验证，明确标记而不是宣称已完成

## Dev Notes

### 当前实现态

- `roles.personality_prompt` 字段已经存在于 Rust model、SQL SELECT、测试建表与 TypeScript `Role` 类型中。
- 当前缺口是：`UpdateRoleInput` 不接受该字段，`update_role` 不写该字段，SettingsTab 没有编辑入口。
- `build_role_messages` 已经会在 `role.personality_prompt.trim()` 非空时追加到角色 system prompt，因此实现重点是补齐编辑/保存链路，并整理 prompt 三层边界。

### 必须复用的既有资产

| 资产 | 用法 |
|---|---|
| `roleService.update(id, input)` | 继续作为唯一前端更新入口，不新增 command/service |
| `role_update` command | 继续作为唯一后端更新入口 |
| `db::roles::update_role` | 在现有 SQL 上增列，不重写角色 CRUD |
| `build_role_messages` | 只调整 system prompt 组装，不改 streaming 主流程 |
| `SettingsTab.handleSave` | 增加字段，不重做保存状态机 |
| `onUpdateRole(updated)` | 保存成功后继续把后端返回 Role 回灌当前视图 |

### Prompt 设计约束

- 角色 prompt 与管家 prompt 是互斥身份；角色路径不得复用 `BUTLER_SYSTEM_PROMPT`。
- 三层分离是实现边界，不是要引入复杂 prompt framework。
- `context_injection` 当前只代表 conversation history 注入；不要提前做 memories 表或记忆提炼，那是 Story 2.6/2.7。
- 预设语调模板只作为 SettingsTab UI 参考与空个性时的轻量后端默认方向，不要做硬编码“角色名 → 输出模板”的确定性回复。

### 前序 Story 2.3 经验

- Story 2.3 已建立 `build_cross_role_summary`、`delegate_to_role` 与 `build_role_messages` 复用路径；本 story 不要碰 `delegate_to_role` 执行链路。
- 上一轮出现过字符串引号导致 Rust 编译失败，新增 prompt 文案时避免半角双引号嵌套造成语法错误。
- 自动化测试不等于桌面验证；若没有跑 `tauri dev`，不要写“端到端已通过”。

### UX 与交互边界

- SettingsTab 当前已有角色名称、图标、颜色、目标、主动性、Skill 配置、危险区域。
- 本 story 只在角色信息中增加个性描述 textarea 与模板参考；不要重排整个设置页。
- 保存成功仍使用现有「已保存」反馈。
- 不新增 modal，不新增 sidebar 入口，不新增角色详情路由。

### Out of Scope

- 记忆提炼、记忆面板、记忆注入 prompt（Story 2.6/2.7）
- 角色主动性配置真实持久化（Story 2.10）
- 技能插件配置真实持久化（Story 2.10）
- 管家语气个性化
- 为每个角色新增独立 prompt 模板管理页面
- LLM 输出内容的确定性分类或后处理改写
- 新增数据库 migration 文件；当前字段已经存在，若兼容旧库只需沿用既有 migration/raw_sql 策略，不重复建列

## Project Structure Notes

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `GUI/src-tauri/src/models/role.rs` | UPDATE | `UpdateRoleInput` 加 `personality_prompt` |
| `GUI/src-tauri/src/db/roles.rs` | UPDATE | `update_role` 写入 personality_prompt；补测试 |
| `GUI/src-tauri/src/services/agent_engine.rs` | UPDATE | 整理角色 system prompt 三层；补测试 |
| `GUI/src/types/role.ts` | UPDATE | `UpdateRoleInput` 加 `personalityPrompt` |
| `GUI/src/components/role/SettingsTab.tsx` | UPDATE | 新增 textarea、模板参考、保存字段 |
| `GUI/src/components/role/SettingsTab.test.tsx` | NEW/UPDATE | 若无既有测试则新建，最小覆盖保存 payload |

### 不应改动

- `GUI/src-tauri/src/commands/chat.rs`
- `GUI/src-tauri/src/services/agent_engine.rs` 的 `delegate_to_role` 执行链路
- `GUI/src/components/chat/*` streaming 行为
- `GUI/src/components/onboarding/*`
- `GUI/src-tauri/migrations/*.sql`（除非实现时发现当前迁移实际缺失 `personality_prompt` 且运行时无法初始化）

### 结构冲突记录

- Epic 原文要求“`roles` 表增加 `personality_prompt` TEXT 字段（migration）”，但当前实现态已存在该字段。选择实现态优先：不重复新增 migration，只补齐使用链路。
- Project context 说新增列必须通过 migration；本 story 不是新增列，而是启用既有列。若实现时证明某环境缺列，再按最小兼容补丁处理。

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.4 AC]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — 后端管理 system prompt + 记忆 + 上下文裁剪；Tauri service/command 分层；命名规则]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — 管家策展 + 角色召唤、角色是用户不同面、角色深入视图与设置入口]
- [Source: `_bmad-output/project-context.md` — TS/Rust 命名、Tauri IPC、测试命令、禁止前端直接 DB/LLM]
- [Source: `_bmad-output/implementation-artifacts/2-3-butler-intent-routing.md` — build_role_messages 已就位；角色 stream 不污染管家；端到端需人工验证]
- [Source: `GUI/src-tauri/src/models/role.rs` — `Role.personality_prompt` 已存在，`UpdateRoleInput` 尚未包含]
- [Source: `GUI/src-tauri/src/db/roles.rs` — `ROLE_SELECT_COLUMNS` 已含 personality_prompt，`update_role` 尚未写入]
- [Source: `GUI/src-tauri/src/services/agent_engine.rs` — `build_role_messages` 已追加非空 personality_prompt]
- [Source: `GUI/src/components/role/SettingsTab.tsx` — 现有设置页保存状态机与角色基础字段]

## Dev Agent Record

### Agent Model Used

gpt-5.5

### Debug Log References

- `npx vitest run src/components/role/SettingsTab.test.tsx`：先红灯确认缺少 `personalityPrompt` payload，修复后 3 passed
- `cargo test`：80 unit tests + 1 integration test passed
- `npx tsc --noEmit`：passed
- `npm run test:frontend`：8 files / 28 tests passed

### Completion Notes List

- 补齐角色个性描述的前后端更新链路：`UpdateRoleInput` / `update_role` / `UpdateRoleInput` TS 类型 / `SettingsTab` 保存 payload。
- `SettingsTab` 新增角色个性描述 textarea 与三条模板参考，保留原保存、错误、归档、删除状态机。
- 角色 system prompt 拆为 `base_persona`、`role_definition`、`context_injection` 三层；角色 prompt 继续与管家身份隔离。
- `personality_prompt` 为空时提供轻量默认语调方向，不做确定性回复改写，不提前实现记忆注入。
- 未运行桌面端 `tauri dev` 人工验证；本轮完成自动化验证。
- **后续追加**：`ROLE_BASE_PERSONA_PROMPT` 从"你是角色"改为"你是用户的分身"——修正角色身份错位问题。
- **后续追加**：去掉管家/角色/onboarding 三处 prompt 的"不用 markdown 格式化"限制；前端 `ChatBubble` 引入 `react-markdown` + `@tailwindcss/typography` 渲染 assistant 消息。

### File List

- `GUI/src-tauri/src/models/role.rs`
- `GUI/src-tauri/src/db/roles.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src/types/role.ts`
- `GUI/src/components/role/SettingsTab.tsx`
- `GUI/src/components/role/SettingsTab.test.tsx`
- `GUI/src/components/chat/ChatBubble.tsx`
- `GUI/tailwind.config.js`
- `GUI/package.json`
- `_bmad-output/implementation-artifacts/2-4-role-personalized-tone.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

| 日期 | 变更 |
|---|---|
| 2026-05-24 | Story 2.4 上下文创建；状态 ready-for-dev |
| 2026-05-24 | 实现角色个性描述编辑与保存链路；角色 prompt 三层分离；自动化验证通过；状态推进到 review |
| 2026-05-25 | 修正 ROLE_BASE_PERSONA_PROMPT：角色是用户分身而非角色本体 |
| 2026-05-25 | 去掉 prompt 中 markdown 限制；ChatBubble 支持 markdown 渲染（react-markdown + typography） |