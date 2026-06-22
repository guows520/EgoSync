---
baseline_commit: 1d8249ee427fd185c4828fc56d6f11dd23ab096a
---

# Story 4.4: 用户在管家对话中看到主动建议卡片并确认/拒绝

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 在管家视角看到角色生成的待确认建议并快速处理,
so that 我能决定哪些建议值得执行。

## 背景与现状（务必先读）

**本 story 是全栈 story — 在 Story 4.2（建议生成+持久化）和 Story 4.3（主动性过滤）的基础上，前端展示 pending 建议为 ActionCard 并支持确认/拒绝操作，后端新增 3 个 Tauri 命令 + DB 操作 + 拒绝反馈注入。涉及前端组件改造、新增 hooks/services/types，以及后端 commands/db/services 三层改动。**

### 已建成的基础（本 story 的接入点）

**Story 4.2（建议生成与持久化）：**
- `migrations/016_suggestions.sql` 已创建 `suggestions` 表：`id`, `role_id`, `title`, `content`, `priority`(high/medium/low), `status`(pending/confirmed/rejected), `rejection_reason`, `converted_task_id`, `created_at`
- `models/suggestion.rs` 已定义 `Suggestion` 结构体（serde camelCase）和 `CreateSuggestionInput`
- `db/suggestions.rs` 已实现 `create_suggestion`、`get_suggestion`、`list_recent_suggestions`
- `services/suggestion_generator.rs` 已实现 `generate_suggestions`、`build_suggestion_prompt`、`parse_suggestions_response`
- **当前 DB 层没有 `list_pending`、`confirm`、`reject` 函数 — 本 story 需新增**

**Story 4.3（主动性过滤）：**
- `services/suggestion_generator.rs:357-369` `filter_suggestions_by_proactivity` 已实现：`proactive` 保留全部，`moderate` 过滤 low，`passive` 返回空
- `services/suggestion_generator.rs:324-348` `NotificationLevel` 枚举 + `max_notification_level_for_proactivity` 已定义（供 Story 4.5 消费）
- **本 story 不动过滤逻辑 — 它已经完成**

**Story 4.1（调度器）：**
- `services/scheduler.rs:144-216` `run_work_loop_for_role` 已实现完整工作循环：generate → filter → write to DB
- `lib.rs:274` `spawn_scheduler(pool.clone())` 在应用启动时已注册
- **本 story 不动调度器 — 建议已能自动写入 DB**

**前端现有组件：**
- `components/butler/ActionCard.tsx`（22 行）— 当前是极简 mock 组件，仅接受 `icon/title/meta/primaryBtn/onPrimary` props，有"稍后"和主操作按钮。**本 story 需完全重写为真实建议数据驱动的组件**
- `components/butler/ButlerView.tsx`（111 行）— 管家视角容器，包含 ChatStream + ButlerWorkspacePanel。**本 story 需在此集成 ActionCard 渲染区域**
- `components/chat/ChatStream.tsx`（1126 行）— 复杂的对话流组件，处理消息渲染、流式输出等。**本 story 已修改 ChatStream — ActionCard 嵌入 ChatStream 对话流末尾渲染，作为管家发出的对话气泡**
- `hooks/useTasks.ts` — 任务 hook 模式参考（`useTauriEvent` 监听 + `invoke` 调用 + scopeKey 稳定依赖）
- `services/chatService.ts` — service 层模式参考（`invoke<T>('command_name', { params })`）

**任务创建基础设施（AC2 确认时创建任务）：**
- `db/tasks.rs:11-49` `create_task` 已实现，接受 `CreateTaskInput { owner_type, role_id, title, deadline, quadrant, is_big_rock }`
- `commands/task.rs:14-51` `task_create` 命令已注册，创建后异步触发自动分类
- `models/task.rs:39-49` `CreateTaskInput` 结构体已定义
- **确认建议时应在后端直接调用 `db::tasks::create_task`（非通过 IPC 调 `task_create` 命令），避免自动分类的异步事件推送无前端监听者**

### 通知系统的现状（影响 AC 中的通知部分）

**Story 4.5（三级通知系统）尚未实现** — 当前代码库中不存在 `notifications` 表、通知服务或通知级别枚举（`NotificationLevel` 枚举已在 4.3 中定义但无消费者）。因此：
- 本 story **不涉及通知创建/推送** — ActionCard 的展示是通过前端主动查询 pending 建议列表，而非通知事件驱动
- 本 story 的 ActionCard 展示逻辑与 Story 4.5 的"敲门"级通知展示将共享同一个 ActionCard 组件，但触发路径不同（本 story = 主动查询；4.5 = 事件推送）

## Acceptance Criteria

1. **AC1**: Given 有 pending 状态的建议，When 用户打开管家视角，Then 管家对话区嵌入 ActionCard 组件展示建议（图标 + 标题 + 来源角色 + 时间）

2. **AC2**: Given 用户点击 ActionCard 的"确认"按钮，When 确认完成，Then `suggestions.status` 更新为 `confirmed`，And 自动在对应角色的 `tasks` 表创建新任务（`converted_task_id` 关联），And ActionCard 显示 ✓ 动画后消失

3. **AC3**: Given 用户点击"拒绝"按钮，When 弹出拒绝原因选择（不相关/时机不对/已完成/其他），Then `suggestions.status` 更新为 `rejected` + `rejection_reason` 写入，And ActionCard 消失

4. **AC4**: Given 拒绝记录，Then 角色下次生成建议时 System Prompt 注入"用户曾拒绝以下类型建议：..."，And 减少类似建议的生成频率

5. **AC5**: Given 已处理（confirmed/rejected）的建议，Then 不再重复显示在管家对话区

6. **AC6**: Given 前端，Then `components/butler/ActionCard.tsx` 接通真实建议数据（替换原型 mock 卡片），And `useSuggestions()` hook 封装查询和操作

7. **AC7**: Given Rust 后端，Then Tauri commands: `suggestion::list_pending` / `suggestion::confirm { id }` / `suggestion::reject { id, reason }`

## Tasks / Subtasks

- [ ] **Task 1: Rust DB 层 — 新增建议查询和状态更新函数** (AC: #1, #2, #3, #5)
  - [ ] 1.1: 在 `db/suggestions.rs` 新增 `list_pending_suggestions(pool) -> Result<Vec<SuggestionWithRole>, AppError>` — 查询所有 `status='pending'` 的建议，JOIN `roles` 表获取角色名和图标
  - [ ] 1.2: 在 `db/suggestions.rs` 新增 `confirm_suggestion(pool, id) -> Result<Suggestion, AppError>` — 更新 `status='confirmed'`，返回更新后的 Suggestion
  - [ ] 1.3: 在 `db/suggestions.rs` 新增 `reject_suggestion(pool, id, reason) -> Result<Suggestion, AppError>` — 更新 `status='rejected'` + `rejection_reason`，返回更新后的 Suggestion
  - [ ] 1.4: 在 `db/suggestions.rs` 新增 `list_rejected_suggestions(pool, role_id, since_iso) -> Result<Vec<Suggestion>, AppError>` — 查询指定角色近 N 天已拒绝的建议（供 AC4 拒绝反馈注入使用）
  - [ ] 1.5: 在 `models/suggestion.rs` 新增 `SuggestionWithRole` 结构体 — 包含 Suggestion 所有字段 + `role_name: String` + `role_icon: String` + `role_color: String`（JOIN 查询结果）

- [ ] **Task 2: Rust command 层 — 新增 3 个 Tauri 命令** (AC: #2, #3, #7)
  - [ ] 2.1: 新建 `commands/suggestion.rs` 文件
  - [ ] 2.2: 实现 `suggestion_list_pending` 命令 — 调用 `db::suggestions::list_pending_suggestions`，返回 `Vec<SuggestionWithRole>`
  - [ ] 2.3: 实现 `suggestion_confirm` 命令 — 接受 `id: String`，调用 `db::suggestions::confirm_suggestion`，然后调用 `db::tasks::create_task` 在对应角色下创建任务（`owner_type=Role`, `role_id=suggestion.role_id`, `title=suggestion.title`），最后更新 `suggestions.converted_task_id` 为新任务 ID，返回确认后的 Suggestion
  - [ ] 2.4: 实现 `suggestion_reject` 命令 — 接受 `id: String, reason: String`，调用 `db::suggestions::reject_suggestion`，返回 rejected 后的 Suggestion
  - [ ] 2.5: 在 `commands/mod.rs` 添加 `pub mod suggestion;`
  - [ ] 2.6: 在 `lib.rs` 的 `invoke_handler` 注册 3 个命令

- [ ] **Task 3: Rust service 层 — 拒绝反馈注入建议生成 prompt** (AC: #4)
  - [ ] 3.1: 在 `services/suggestion_generator.rs` 的 `generate_suggestions` 函数中，在构建 prompt 前查询该角色近 7 天的已拒绝建议（调用 `db::suggestions::list_rejected_suggestions`）
  - [ ] 3.2: 修改 `build_suggestion_prompt` 函数签名，新增 `rejected_suggestions: &[Suggestion]` 参数
  - [ ] 3.3: 在 `build_suggestion_prompt` 的 user prompt 中，当 `rejected_suggestions` 非空时追加段落：`[用户曾拒绝的建议]\n- 标题：... | 拒绝原因：...\n请避免生成与以上被拒绝建议类似的内容。`
  - [ ] 3.4: 更新 `build_suggestion_prompt` 的所有调用处（`generate_suggestions` 内部）和所有单元测试

- [ ] **Task 4: 前端 types + service 层** (AC: #6)
  - [ ] 4.1: 新建 `types/suggestion.ts` — 定义 `Suggestion` 和 `SuggestionWithRole` 接口（camelCase，与 Rust serde 对齐）
  - [ ] 4.2: 新建 `services/suggestionService.ts` — 封装 3 个 Tauri invoke 调用：`listPending()`, `confirm(id)`, `reject(id, reason)`

- [ ] **Task 5: 前端 hooks — useSuggestions** (AC: #1, #5, #6)
  - [ ] 5.1: 新建 `hooks/useSuggestions.ts` — 封装 pending 建议查询 + 确认 + 拒绝操作
  - [ ] 5.2: hook 内部管理 `suggestions: SuggestionWithRole[]` 状态，`isLoading` 状态，`error` 状态
  - [ ] 5.3: 实现 `confirmSuggestion(id)` — 调用 `suggestionService.confirm(id)`，成功后从本地 state 移除该建议
  - [ ] 5.4: 实现 `rejectSuggestion(id, reason)` — 调用 `suggestionService.reject(id, reason)`，成功后从本地 state 移除该建议
  - [ ] 5.5: 提供 `refetch()` 方法供手动刷新

- [ ] **Task 6: 前端 ActionCard 组件重写** (AC: #1, #2, #3)
  - [ ] 6.1: 重写 `components/butler/ActionCard.tsx` — 接受 `SuggestionWithRole` 数据 + `onConfirm` / `onReject` 回调
  - [ ] 6.2: 展示内容：角色图标 + 建议标题 + 来源角色名 + 创建时间 + priority 标签
  - [ ] 6.3: 操作按钮：Primary "确认"（indigo 实心）+ Ghost "拒绝"（灰边框）— 遵循 UX 按钮层级规则
  - [ ] 6.4: 确认动画：点击确认后显示 ✓ 图标 + opacity 降至 0.5 + 300ms 后消失（遵循 UX 反馈模式）
  - [ ] 6.5: 拒绝交互：点击拒绝后展开内联拒绝原因选择器（4 个选项：不相关/时机不对/已完成/其他），选择"其他"时展开文本输入框供用户录入自定义原因（可选，不录入则传 "other"），选择后触发拒绝动画 + 消失
  - [ ] 6.6: 无障碍：`role="article"`, `aria-label="[标题] - [来源角色]"`，按钮可 Tab 聚焦，Enter/Space 触发

- [ ] **Task 7: 前端 ButlerView + ChatStream 集成 ActionCard** (AC: #1, #5)
  - [ ] 7.1: 在 `ButlerView.tsx` 中调用 `useSuggestions()` hook
  - [ ] 7.2: 将 `suggestions` 及操作函数通过 props 传递给 `ChatStream`
  - [ ] 7.3: 在 `ChatStream.tsx` 对话消息列表末尾、流式消息之前渲染 ActionCard 列表（管家气泡风格：左对齐 + 管家图标/名称头部 + max-w-[85%] 宽度）
  - [ ] 7.4: 确认/拒绝操作后，hook 自动从列表移除，ActionCard 区域动态更新

- [ ] **Task 8: Rust 单元测试** (AC: #1-#5)
  - [ ] 8.1: `db/suggestions.rs` 测试 — `list_pending_suggestions` 返回正确结果、`confirm_suggestion` 状态更新 + `converted_task_id` 写入、`reject_suggestion` 状态更新 + `rejection_reason` 写入、`list_rejected_suggestions` 按角色和时间范围过滤
  - [ ] 8.2: `services/suggestion_generator.rs` 测试 — `build_suggestion_prompt` 含拒绝建议段落、不含拒绝建议时不输出拒绝段、`generate_suggestions` 集成测试（mock DB + 验证拒绝反馈注入）
  - [ ] 8.3: `commands/suggestion.rs` 测试 — 3 个命令的输入校验和错误路径

- [ ] **Task 9: 前端测试** (AC: #1, #2, #3, #5)
  - [ ] 9.1: `ActionCard.test.tsx` — 渲染正确内容、确认回调触发、拒绝原因选择 + 回调触发、确认动画、拒绝动画
  - [ ] 9.2: `useSuggestions.test.ts` — 初始加载、确认后移除、拒绝后移除、错误处理

## Dev Notes

### 架构约束（开发者必须遵守）

**Rust 三层架构：**
- `commands/` — Tauri IPC 命令处理，薄层，参数校验后调 db/services
- `db/` — SQLx 数据访问，`Result<T, AppError>` 返回，无业务逻辑
- `services/` — 业务逻辑，可调 db 层
- **本 story 严格遵循三层：commands/suggestion.rs → db/suggestions.rs + services/suggestion_generator.rs**

**serde camelCase 规则：**
- 所有 Rust 结构体必须标注 `#[serde(rename_all = "camelCase")]`
- 前端 TypeScript 接口使用 camelCase 属性名
- `SuggestionWithRole` 的字段：`roleId`, `roleName`, `roleIcon`, `roleColor`, `createdAt` 等

**错误处理：**
- 所有 DB 操作返回 `Result<T, AppError>`，禁止 `.unwrap()`
- `AppError` 枚举：`NotFound`、`DbError`、`ValidationError`
- 命令层错误通过 `Result<T, AppError>` 直接返回给前端

**Tauri 命令注册模式：**
- `commands/mod.rs` 添加 `pub mod suggestion;`
- `lib.rs` 的 `invoke_handler` 宏中添加 3 个命令引用
- 命令函数签名：`#[tauri::command] pub async fn suggestion_xxx(params..., pool: State<'_, DbPool>) -> Result<ReturnType, AppError>`

**前端 service 层模式（参考 `chatService.ts`）：**
```typescript
import { invoke } from '@tauri-apps/api/core';
export const suggestionService = {
  listPending: () => invoke<SuggestionWithRole[]>('suggestion_list_pending'),
  confirm: (id: string) => invoke<Suggestion>('suggestion_confirm', { id }),
  reject: (id: string, reason: string) => invoke<Suggestion>('suggestion_reject', { id, reason }),
};
```

**前端 hook 模式（参考 `useTasks.ts`）：**
- `useState` 管理 `suggestions`、`isLoading`、`error`
- `useEffect` 初始加载
- `useCallback` 封装 `confirmSuggestion`、`rejectSuggestion`、`refetch`
- 操作成功后从本地 state 移除已处理建议（无需重新查询全量）

**UX 设计约束（必须遵守）：**
- **不使用 toast/snackbar** — 所有反馈通过管家自然语言或卡片状态变更传达
- **ActionCard 按钮层级**：Primary "确认"（实心 indigo 色）+ Ghost "拒绝"（透明底灰边框），Primary 在最右
- **卡片圆角 10px**，hover 上浮 `translateY(-1px)` + 阴影加深，200ms ease
- **操作成功反馈**：卡片 opacity 降至 0.5 + ✓ 图标，持续显示后淡出消失
- **拒绝原因选择器**：内联展开，不使用 Modal/Dialog — 4 个选项以小按钮形式横向排列
- **无障碍**：`role="article"`, `aria-label="[标题] - [来源角色]"`，按钮可 Tab 聚焦
- **动效减弱**：`prefers-reduced-motion` 时动画降为 0ms

### AC2 确认创建任务的关键细节

**确认建议时创建任务的流程：**
1. `suggestion_confirm` 命令接收 `id`
2. 调用 `db::suggestions::confirm_suggestion(pool, &id)` — 更新 status 为 confirmed
3. 读取 suggestion 的 `role_id` 和 `title`
4. 构造 `CreateTaskInput { owner_type: Some(TaskOwnerType::Role), role_id: Some(role_id), title: suggestion.title, deadline: None, quadrant: None, is_big_rock: None }`
5. 调用 `db::tasks::create_task(pool, &input)` — 创建任务（默认 Q2，后续自动分类）
6. 更新 `suggestions.converted_task_id` 为新任务 ID — 需在 `db/suggestions.rs` 新增 `set_converted_task_id(pool, suggestion_id, task_id)` 函数
7. 返回确认后的 Suggestion

**注意：不通过 IPC 调用 `task_create` 命令** — 直接调用 `db::tasks::create_task` 避免自动分类的 `task:classified` 事件推送到前端时无监听者。自动分类仍会在后台运行（因为 `quadrant=None`），但前端通过任务列表刷新自然获取分类结果。

### AC4 拒绝反馈注入的关键细节

**拒绝反馈注入流程：**
1. `generate_suggestions` 在构建 prompt 前，调用 `db::suggestions::list_rejected_suggestions(pool, &role.id, &seven_days_ago_iso())`
2. 将拒绝建议列表传入 `build_suggestion_prompt(role, task_summary, memory_summary, recent_suggestions, rejected_suggestions)`
3. `build_suggestion_prompt` 在 user prompt 中追加：
   ```
   [用户曾拒绝的建议]
   - 标题：xxx | 拒绝原因：不相关
   - 标题：yyy | 拒绝原因：时机不对
   请避免生成与以上被拒绝建议类似的内容。
   ```
4. 拒绝查询失败时降级为空列表（不阻塞建议生成），与 `recent_suggestions` 查询失败的处理方式一致

### ActionCard 渲染位置决策

**ActionCard 嵌入 ChatStream 对话流渲染：**
- `ButlerView.tsx` 调用 `useSuggestions()` hook，将 `suggestions` 及 `onConfirmSuggestion` / `onRejectSuggestion` / `onDismissSuggestion` 通过 props 传给 `ChatStream`
- `ChatStream.tsx` 在对话消息列表末尾、流式消息之前渲染 ActionCard 列表
- 渲染风格模仿管家对话气泡：左对齐 + 管家图标/名称头部 + `max-w-[85%]` 宽度
- 流式回复期间（`isStreaming`）暂时隐藏建议卡片，回复结束后重新显示
- 仅当 `suggestions.length > 0` 且非流式状态时渲染该区域

### 前端 ActionCard 组件设计

**ActionCard 重写后的 props 接口：**
```typescript
interface ActionCardProps {
  suggestion: SuggestionWithRole;
  onConfirm: (id: string) => void;
  onReject: (id: string, reason: string) => void;
}
```

**拒绝原因选项（固定 4 项）：**
```typescript
const REJECTION_REASONS = [
  { value: 'irrelevant', label: '不相关' },
  { value: 'bad_timing', label: '时机不对' },
  { value: 'already_done', label: '已完成' },
  { value: 'other', label: '其他' },
];
```

**组件内部状态：**
- `status: 'pending' | 'confirming' | 'confirmed' | 'rejecting' | 'rejected'`
- `showRejectionReasons: boolean` — 拒绝按钮点击后展开原因选择器
- `selectedReason: string | null`

### Previous Story Intelligence（Story 4.1 + 4.2 + 4.3）

- **4.1 已处理 passive 跳过**：`get_trigger_times("passive")` 返回 `None`，调度器 `continue`。passive 角色不会有 pending 建议。
- **4.2 已建立 priority 体系**：`generate_suggestions` 返回的建议 `priority` 已归一化为小写。`CreateSuggestionInput` 含 `priority` 字段。
- **4.2 Review Findings**：空上下文角色短路跳过 LLM（已修复）、批次内同名建议去重（已修复）。
- **4.3 已建立过滤逻辑**：`filter_suggestions_by_proactivity` 在 `scheduler.rs:167-170` 调用，写入 DB 的建议已按主动性过滤。
- **4.3 Review Findings**：`moderate` 档仅排除精确 `"low"`，未校验 priority 取值域 — pre-existing，本 story 不修复。
- **4.3 测试命令**：`cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` — 447 passed, 0 failed。本 story 新增测试后应维持 0 failed。
- **4.3 前端测试**：`npm run test:frontend` 有 4 个既有失败（SettingsTab/TasksTab），与本 story 无关。

### Git Intelligence

- `1d8249e`（HEAD）— feat(4.3): 主动性三档刻度盘行为接通（最新提交，本 story 的直接前置）
- `c910664` — Refine role settings and task filter UI（前端 UI 调整）
- `2afbdf7` — fix(4.2): patch AC4 empty-role short-circuit and batch dedup
- `e764743` — feat(4.1): 可配置调度时间点
- 范本来源：`commands/task.rs`（命令实现模式）、`hooks/useTasks.ts`（hook 模式）、`services/chatService.ts`（service 层模式）

### Testing Requirements

- **Rust 单测**（`db/suggestions.rs` 测试模块新增 ~8 个测试，`services/suggestion_generator.rs` 新增 ~4 个测试，`commands/suggestion.rs` 新增 ~3 个测试）：
  - `list_pending_suggestions`：返回 pending 建议 + 角色信息、排除 confirmed/rejected
  - `confirm_suggestion`：状态更新 + converted_task_id 写入
  - `reject_suggestion`：状态更新 + rejection_reason 写入
  - `list_rejected_suggestions`：按角色过滤、按时间范围过滤
  - `build_suggestion_prompt` 含拒绝建议段落、不含时不输出
  - 命令层输入校验和错误路径
- **前端测试**（`ActionCard.test.tsx` ~5 个测试，`useSuggestions.test.ts` ~4 个测试）：
  - ActionCard 渲染、确认回调、拒绝原因选择 + 回调、动画
  - useSuggestions 初始加载、确认后移除、拒绝后移除、错误处理
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`
  - `npm --prefix "egosync-app" run test:frontend`

### Project Structure Notes

- **新增文件（5）**：
  - `egosync-app/src-tauri/src/commands/suggestion.rs` — 3 个 Tauri 命令
  - `egosync-app/src/types/suggestion.ts` — TypeScript 类型定义
  - `egosync-app/src/services/suggestionService.ts` — 前端 service 层
  - `egosync-app/src/hooks/useSuggestions.ts` — 建议 hook
  - `egosync-app/src/components/butler/ActionCard.test.tsx` — ActionCard 测试
- **修改文件（6）**：
  - `egosync-app/src-tauri/src/db/suggestions.rs` — 新增 4 个 DB 函数 + SuggestionWithRole 查询
  - `egosync-app/src-tauri/src/models/suggestion.rs` — 新增 SuggestionWithRole 结构体
  - `egosync-app/src-tauri/src/services/suggestion_generator.rs` — 修改 build_suggestion_prompt + generate_suggestions
  - `egosync-app/src-tauri/src/commands/mod.rs` — 添加 suggestion 模块
  - `egosync-app/src-tauri/src/lib.rs` — 注册 3 个新命令
  - `egosync-app/src/components/butler/ActionCard.tsx` — 完全重写
  - `egosync-app/src/components/butler/ButlerView.tsx` — 集成 ActionCard 区域
- **无新增依赖、无新增迁移、无 DB schema 改动** — suggestions 表已有所有需要的字段
- 符合项目规则：Rust 三层、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 数据边界 / 前端 service 层模式）
- `_bmad-output/planning-artifacts/epics.md:1685-1721`（Story 4.4 定义）
- `_bmad-output/planning-artifacts/architecture.md:220`（suggestions 表 schema）、`:373-376`（IPC 命令按域分模块）、`:390-393`（事件驱动状态同步）、`:427-451`（前端架构 + service 层）
- `_bmad-output/planning-artifacts/ux-design-specification.md:709-716`（ActionCard 组件规范）、`:802-814`（按钮层级）、`:816-827`（反馈模式 — 不使用 toast/snackbar）、`:862-870`（通知级别 — ActionCard = L3 敲门）、`:960-973`（无障碍 — ActionCard role/aria-label）
- `_bmad-output/implementation-artifacts/4-3-proactivity-dial-behavior.md`（Story 4.3 — 主动性过滤已实现，NotificationLevel 枚举已定义）
- `_bmad-output/implementation-artifacts/4-2-proactive-suggestion-generation.md`（Story 4.2 — 建议生成器 + DB 层已实现）
- `_bmad-output/implementation-artifacts/4-1-background-scheduler-work-loop.md`（Story 4.1 — 调度器已实现）
- `egosync-app/src-tauri/migrations/016_suggestions.sql`（suggestions 表 DDL）
- `egosync-app/src-tauri/src/db/suggestions.rs:1-58`（现有 DB 函数 — create/get/list_recent）
- `egosync-app/src-tauri/src/models/suggestion.rs:1-22`（Suggestion + CreateSuggestionInput 结构体）
- `egosync-app/src-tauri/src/services/suggestion_generator.rs:52-111`（build_suggestion_prompt — 需修改签名）
- `egosync-app/src-tauri/src/services/suggestion_generator.rs:151-269`（generate_suggestions — 需注入拒绝反馈查询）
- `egosync-app/src-tauri/src/commands/task.rs:14-51`（task_create 命令 — 确认创建任务的参考）
- `egosync-app/src-tauri/src/db/tasks.rs:11-49`（create_task DB 函数 — 确认时直接调用）
- `egosync-app/src-tauri/src/models/task.rs:39-49`（CreateTaskInput 结构体）
- `egosync-app/src-tauri/src/lib.rs:278-349`（invoke_handler — 需注册新命令）
- `egosync-app/src-tauri/src/commands/mod.rs:1-10`（命令模块注册）
- `egosync-app/src/components/butler/ActionCard.tsx:1-22`（当前 mock 组件 — 需完全重写）
- `egosync-app/src/components/butler/ButlerView.tsx:1-111`（管家视角容器 — 需集成 ActionCard）
- `egosync-app/src/hooks/useTasks.ts:1-60`（hook 模式参考）
- `egosync-app/src/services/chatService.ts:1-25`（service 层模式参考）
- `egosync-app/src/types/chat.ts:14-24`（ChatMessage 类型 — 类型定义模式参考）

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Change Log

### Review Findings

- [x] [Review][Patch] 确认/拒绝动画在真实集成中不显示（原 Decision，已采用「动画后再移除」方案）— `ActionCard` 新增 `onDismiss`，动画结束（300ms）后再通知列表移除；`useSuggestions` 改为 confirm/reject 成功不立即过滤，新增 `removeSuggestion` 供动画驱动移除。AC2/AC3 视觉反馈得以完整呈现。 [useSuggestions.ts:54-82] [ActionCard.tsx:77-84] [ButlerView.tsx:80-92]
- [x] [Review][Patch] suggestion_confirm 三步写入非原子 — 改为先 `get_suggestion` 校验 pending → 先 `create_task` → 再 `confirm_suggestion` + `set_converted_task_id`；任务创建失败时建议保持 pending 可重试，消除「已确认但无任务」的静默丢失。 [commands/suggestion.rs:31-58]
- [x] [Review][Patch] 后端拒绝原因校验 — `validate_reject_reason` 原为白名单校验，后放宽为仅拒绝空字符串，接受任意非空自定义原因（配合前端"其他"选项的文本输入框）。 [commands/suggestion.rs:66-72]
- [x] [Review][Patch] 建议卡片区无最大高度/滚动 — 原在 `ButlerView` 卡片区加 `max-h-[40%] overflow-y-auto`，后卡片嵌入 ChatStream 对话流，由 ChatStream 滚动容器统一管理。
- [x] [Review][Patch] 命令层正常路径缺测试 — 抽出 `confirm_and_create_task` / `reject_with_reason` 可测辅助函数，新增任务创建+回填、已处理不建任务、错误路径、白名单等集成测试。 [commands/suggestion.rs:173-235]
- [x] [Review][Patch] 冗余包装函数 — 删除 `get_suggestion_with_converted_task_id`，直接调用 `get_suggestion`。 [commands/suggestion.rs:57]

**dismissed（噪音/已处理，3）**：`actionInFlight` 未被 ButlerView 消费（导出 API 无害，ActionCard 内部 status 守卫已防重复点击）；`onConfirm/onReject` 类型为 `void` 实为 async（`Promise.resolve()` 包装已处理）；确认成功后对卸载组件 setState 警告（采用 onDismiss 方案后已消除）。

**验证**：`cargo test ... suggestion` 53 passed / 0 failed；前端 `ActionCard.test.tsx` + `useSuggestions.test.ts` 17 passed；`tsc && vite build` 通过。
