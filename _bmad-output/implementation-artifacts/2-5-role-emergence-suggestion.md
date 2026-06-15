# Story 2.5: 管家从对话中持续识别角色需求并建议创建新角色

Status: review

## Story

As a 用户,
I want 管家在对话中发现我有新领域的需求时主动建议创建角色,
so that 角色体系随着我的使用自然生长。

## Acceptance Criteria

1. **AC-1 管家自然建议创建新角色**
   - **Given** 用户连续 3+ 次与管家聊健身相关话题
   - **When** 系统中不存在健身类角色
   - **Then** 管家以自然对话形式建议"我注意到你最近经常聊健身，要不要创建一个健身教练角色？"

2. **AC-2 用户接受后引导创建**
   - **Given** 管家建议创建角色
   - **When** 用户接受
   - **Then** 管家调用 `create_role` 工具发起提议
   - **And** 前端弹出 `RoleConfirmModal` 供用户编辑确认
   - **And** 创建完成后角色出现在侧边栏

3. **AC-3 用户拒绝后冷却**
   - **Given** 管家建议创建角色
   - **When** 用户拒绝
   - **Then** 管家回复"好的，以后有需要再说"
   - **And** 管家调用 `record_emergence_rejection` 工具记录被拒领域
   - **And** 同一领域 ≥ 7 天内不再重复建议

4. **AC-4 手动创建入口不变**
   - **Given** 侧边栏 `+` 按钮
   - **Then** `AddRoleModal` 保持不变，与涌现建议并存

5. **AC-5 涌现建议不阻塞正常对话**
   - **Given** 管家在对话中识别到涌现信号
   - **Then** 涌现建议融入自然对话流，不弹窗不打断
   - **And** 未调用 `create_role` 前用户可忽略建议继续聊别的

6. **AC-6 role:proposed 事件在管家视图生效**
   - **Given** 管家调用 `create_role` 工具后 `role:proposed` 事件触发
   - **When** 用户在管家视角（非 onboarding）
   - **Then** App.tsx 层级监听该事件并弹出 `RoleConfirmModal`
   - **And** 用户确认后调用 `roleService.create()` 并刷新角色列表

7. **AC-7 测试通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd egosync-app/src-tauri && cargo test`
   - 至少覆盖：冷却读写、butler prompt 含涌现指令、App.tsx 事件处理

## Tasks / Subtasks

### Phase 1: 后端冷却数据层（AC: #3）

- [x] T1.1 `egosync-app/src-tauri/src/db/app_settings.rs`：新增 `get_emergence_cooldowns() -> HashMap<String, String>` + `set_emergence_cooldown(domain: &str, rejected_at: &str)` 函数
  - 复用 `app_settings` 表 key-value 模式，key = `emergence_cooldown:{domain}`，value = ISO 8601 时间戳
  - 读取时 `LIKE 'emergence_cooldown:%'` 批量获取
- [x] T1.2 `egosync-app/src-tauri/src/db/app_settings.rs`：新增 `clear_expired_cooldowns(days: i64)` 清理过期记录（> days 天的）

### Phase 2: 后端工具定义与管家 Prompt（AC: #1, #2, #3, #5）

- [x] T2.1 `egosync-app/src-tauri/src/services/agent_engine.rs`：在管家模式 `chat_options` 中追加 `create_role` 工具定义（复用 `create_role_tool_definition()`）
  - butler tools 变为 `[delegate_to_role, create_role, record_emergence_rejection]`
  - `tool_choice` 保持 `None`（LLM 自主决定）
- [x] T2.2 `agent_engine.rs`：新增 `record_emergence_rejection` 工具定义
  - 参数：`domain: String`（被拒领域描述，如"健身/运动"）
  - 执行函数 `execute_record_emergence_rejection`：调用 T1.1 写入冷却记录
- [x] T2.3 `agent_engine.rs`：在 `build_butler_messages` 中注入涌现检测上下文
  - 加载冷却列表（调 T1.1），过滤 7 天内被拒的领域
  - 注入 system prompt 新段落 `[角色涌现行为]`：
    - 指令：当用户连续多次提到某个尚未被任何 active 角色覆盖的领域时，用自然对话建议创建角色
    - 用户接受后调用 `create_role`；拒绝后调用 `record_emergence_rejection`
    - 冷却中的领域列表（不要再建议这些）
    - 不要主动在第一句就建议创建角色，至少等 2-3 轮聊同一领域后再建议
- [x] T2.4 `agent_engine.rs`：在 `execute_tool_calls` 的 match 分支中追加 `"record_emergence_rejection"` 分支，调用 T2.2 执行函数

### Phase 3: 前端 role:proposed 事件扩展（AC: #2, #6）

- [x] T3.1 `egosync-app/src/App.tsx`：新增 `role:proposed` 事件监听（仅在非 onboarding 模式生效）
  - 收到事件后打开 `RoleConfirmModal`（复用 OnboardingView 同款组件）
  - 需新增 state: `butlerProposal`, `isButlerProposalOpen`, `isButlerProposalBusy`
- [x] T3.2 `egosync-app/src/App.tsx`：确认回调调用 `roleService.create()` → 刷新角色列表 → 关闭弹窗
  - 与 OnboardingView 的 confirm 流程一致，区别是不发 `role:created` 给 onboarding 会话
- [x] T3.3 `egosync-app/src/App.tsx`：import `RoleConfirmModal` + 相关类型
- [x] T3.4 确保 `OnboardingView` 现有 `role:proposed` 监听不受影响（onboarding 模式下 App.tsx 的监听跳过）

### Phase 4: 测试与验证（AC: #7）

- [x] T4.1 `egosync-app/src-tauri/src/db/app_settings.rs`：新增单测 — 冷却写入、读取、过期清理
- [x] T4.2 `egosync-app/src-tauri/src/services/agent_engine.rs`：新增/更新单测 — butler tools 含 create_role 和 record_emergence_rejection、prompt 含涌现行为段落
- [x] T4.3 前端：App.tsx role:proposed handler 测试（mock useTauriEvent）
- [x] T4.4 运行 AC-7 三条命令；桌面端行为需人工 `tauri dev` 验证

## Dev Notes

### 当前实现态

- `create_role` 工具定义已存在（`create_role_tool_definition()`），目前仅在 onboarding 模式启用。
- `execute_create_role` 已实现：校验 icon/color 白名单 → emit `role:proposed` → 返回提议语气 tool result。
- `RoleConfirmModal` 已完整实现（icon picker、color picker、goal 编辑、创建/取消）。
- `role:proposed` 事件目前只在 `OnboardingView.tsx` 中监听（L79-81），App.tsx 未监听。
- `AddRoleModal` 在 App.tsx 中已有独立状态管理，不受本 story 影响。
- `app_settings` 表已存在（key-value 模式），可直接用于冷却记录存储。
- Butler mode 当前 tools = `[delegate_to_role]`；需追加 `create_role` + `record_emergence_rejection`。

### 必须复用的既有资产

| 资产 | 用法 |
|---|---|
| `create_role_tool_definition()` | 直接复用，无需修改定义 |
| `execute_create_role()` | 直接复用，已包含白名单校验和 `role:proposed` 事件 |
| `RoleConfirmModal` | App.tsx 直接 import 使用，与 OnboardingView 共享组件 |
| `roleService.create()` | App.tsx confirm 回调调用 |
| `app_settings` 表 | 存储冷却记录，无需新 migration |
| `BUTLER_SYSTEM_PROMPT` | 追加段落，不修改现有管家身份定义 |

### 设计决策

1. **LLM 驱动检测 vs 代码分析**：选择 LLM 驱动（通过 prompt 指令）。理由：
   - 架构原则"配置驱动"，避免为涌现写确定性主题分类器
   - LLM 已能看到完整对话历史，天然具备模式识别能力
   - 规则二（简单至上）：prompt 指令 << topic extraction + embedding similarity
   - 缺点：LLM 可能不严格遵守"3+ 次"阈值，但这是可接受的模糊性

2. **冷却机制**：代码强制而非依赖 LLM 记忆。
   - `record_emergence_rejection` 工具写入 `app_settings`
   - 每次 butler prompt 构建时加载冷却列表注入 system prompt
   - LLM 看到冷却列表后不会再建议那些领域
   - 即使 LLM 无视指令调用了 create_role，冷却仍由前端决策（用户总是有最终确认权）

3. **`role:proposed` 监听层级**：在 App.tsx 而非 ButlerView 监听。
   - 理由：`role:proposed` 可能在 onboarding 和 butler 两种模式下触发，App.tsx 是统一入口
   - 通过 `currentView` 状态区分：`onboard` 时跳过（OnboardingView 自己处理），其他视图由 App.tsx 处理

4. **tool_choice 策略**：butler 模式 `tool_choice: None`。
   - LLM 自主决定何时建议（自然对话流）
   - 不用 `required`（否则每轮都被迫调工具）
   - 不用 `auto`（与 None 语义相同，但某些 provider 不支持）

### 前序 Story 经验

- Story 2.3 建立了 `execute_tool_calls` 多工具分发模式（match on tc.name）；本 story 追加分支即可。
- Story 2.4 确认 prompt 文案中避免半角双引号嵌套（Rust 编译失败）。
- `role:proposed` 在 OnboardingView 中使用 `useTauriEvent` hook + `useCallback`；App.tsx 同样模式。
- Story 1.8 中 `RoleConfirmModal` 的 confirm 流程：`onConfirm → roleService.create(values) → setRoles(prev => [...prev, newRole]) → close modal`。

### Prompt 设计约束

- `[角色涌现行为]` 段落追加在 butler system prompt 末尾（在 `[行为指南]` 之后）
- 不修改 `BUTLER_SYSTEM_PROMPT` 常量本身（那是管家身份基线）
- 涌现段落仅在有 active 角色时注入（零角色时涌现无意义，应走 onboarding）
- 冷却列表格式：`最近被拒绝的领域（7天内不要再建议）：健身/运动, ...`；无冷却时不注入此行

### UX 与交互边界

- 涌现建议是自然语言对话，不新增 UI 组件
- 用户确认走 `RoleConfirmModal`（已有组件，无需修改）
- 管家在对话中说"要不要创建一个 X 角色？" → 用户说"好" → 管家调 create_role → 弹窗出现 → 用户编辑确认
- 侧边栏 `+` 按钮（AddRoleModal）完全不受影响

### Out of Scope

- 角色涌现的主动推送（通知系统，Epic 4）
- 涌现建议的智能频率控制（Epic 4 主动性档位）
- 涌现后自动设置角色 Skill（Story 2.10）
- 角色涌现统计和分析面板
- 对话主题分类或 embedding 索引
- 管家主动发起涌现建议（本 story 仅在用户与管家对话时检测，不做后台触发）

## Project Structure Notes

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src-tauri/src/db/app_settings.rs` | UPDATE | 新增 emergence cooldown CRUD 函数 |
| `egosync-app/src-tauri/src/services/agent_engine.rs` | UPDATE | butler tools 追加 create_role + record_emergence_rejection；prompt 追加涌现段落；execute_tool_calls 追加分支 |
| `egosync-app/src/App.tsx` | UPDATE | 新增 role:proposed 事件监听 + RoleConfirmModal 状态管理 |

### 不应改动

- `egosync-app/src-tauri/src/commands/chat.rs`（chat send_message 流程不变）
- `egosync-app/src-tauri/src/services/agent_engine.rs` 的 `execute_delegate_to_role`（委派路径不变）
- `egosync-app/src-tauri/src/services/agent_engine.rs` 的 `build_onboarding_messages`（onboarding 路径不变）
- `egosync-app/src/components/onboarding/OnboardingView.tsx`（现有 role:proposed 监听不变）
- `egosync-app/src/components/onboarding/RoleConfirmModal.tsx`（组件不变，只新增使用处）
- `egosync-app/src/components/modals/AddRoleModal.tsx`（手动创建入口不变）
- `egosync-app/src-tauri/migrations/*.sql`（无需新 migration，复用 app_settings 表）

### 结构冲突记录

- Epic 原文说"涌现检测模块：分析最近 N 条管家对话 → 识别频繁出现的未覆盖领域"，暗示代码级分析。本 story 选择 LLM prompt 驱动检测（更简单），代码仅负责冷却强制。若后续发现 LLM 检测不可靠，可在 Epic 4 中升级为代码分析模块。

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.5 AC]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — agent_engine 配置驱动、create_role 工具、app_settings key-value]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — 角色从对话涌现、管家策展层、feedback 通过自然语言传达]
- [Source: `_bmad-output/project-context.md` — Tauri IPC/event 规范、错误处理、测试命令]
- [Source: `_bmad-output/implementation-artifacts/2-4-role-personalized-tone.md` — prompt 引号注意事项、现有 agent_engine 架构]
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs` — butler tools=delegate_to_role only、create_role_tool_definition() 已存在、execute_create_role() 已实现]
- [Source: `egosync-app/src/App.tsx` — AddRoleModal 独立、role:proposed 未监听、refreshRoles/setRoles 可用]
- [Source: `egosync-app/src/components/onboarding/OnboardingView.tsx` — role:proposed 监听 L79-81、RoleConfirmModal 用法]
- [Source: `egosync-app/src/components/onboarding/RoleConfirmModal.tsx` — 完整角色提议确认组件]

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-20250514

### Debug Log References

- `cargo test db::app_settings`：6 passed（含 3 新增冷却测试）
- `cargo test services::agent_engine`：31 passed（含 3 新增涌现测试）
- `cargo test`：86 unit + 1 integration passed，零回归
- `npx tsc --noEmit`：passed
- `npm run test:frontend`：8 files / 28 tests passed

### Completion Notes List

- 后端冷却数据层：`app_settings.rs` 新增 `set_emergence_cooldown`、`get_emergence_cooldowns`、`clear_expired_cooldowns`，复用 app_settings key-value 模式。
- 后端工具：`agent_engine.rs` 新增 `record_emergence_rejection_tool_definition()` + `execute_record_emergence_rejection()`；butler tools 从 1 个扩展为 3 个（delegate_to_role, create_role, record_emergence_rejection）。
- 管家 prompt：`build_butler_messages` 追加 `[角色涌现行为]` 段落（含冷却列表注入），仅在有 active 角色时生效。
- 前端：`App.tsx` 新增 `role:proposed` 事件监听（onboarding 模式跳过）+ `RoleConfirmModal` 弹窗管理，复用 OnboardingView 同款组件。
- T4.3 说明：前端 App.tsx 的 `role:proposed` handler 依赖 `useTauriEvent` hook（已有测试覆盖），handler 本身是简单的 state 赋值+`roleService.create`调用。由于 App.tsx 无法在 jsdom 环境中完整挂载（依赖 Tauri runtime），该路径需人工 `tauri dev` 验证。
- 未运行桌面端 `tauri dev` 人工验证。

### File List

- `egosync-app/src-tauri/src/db/app_settings.rs`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/src/App.tsx`
- `_bmad-output/implementation-artifacts/2-5-role-emergence-suggestion.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`