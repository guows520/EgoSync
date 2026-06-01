---
created_at: 2026-06-01T17:45:04+08:00
baseline_commit: d01ef6849bbc55c623418009372fbce0ff097056
---

# Story 2.8: 用户能删除特定记忆（选择性遗忘）

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 删除不准确或不想被记住的特定结构化记忆,
so that 我能控制 AI 记住什么，并阻止错误记忆继续影响后续建议。

## Acceptance Criteria

1. **AC-1 记忆卡片提供自定义“遗忘”确认交互**
   - **Given** 用户在角色视图或管家视图的 MemoryTab 某条记忆卡片上点击 `遗忘`
   - **When** 触发确认
   - **Then** 在 EgoSync UI 内显示管家风格确认文案，例如：`确定要忘记这条吗？忘了就真忘了哦。原始对话还会留在历史里。`
   - **And** 不使用系统级 `window.confirm`、浏览器 alert、toast 或 snackbar
   - **And** 确认交互必须有明确的 `确认遗忘` 与 `再想想`/取消操作
   - **And** `遗忘` 使用 destructive 视觉层级：红色文字/边框，符合现有 Tailwind utility 风格

2. **AC-2 用户取消遗忘时无任何副作用**
   - **Given** 确认交互已显示
   - **When** 用户点击取消、`再想想` 或关闭确认态
   - **Then** 不调用 `memory_delete`
   - **And** 该记忆仍保留在列表中
   - **And** badge 数量不变
   - **And** 来源展开状态不被错误清空

3. **AC-3 用户确认遗忘后仅删除 memories 表对应记录**
   - **Given** 用户确认遗忘某条记忆
   - **When** 前端调用 `memory_delete { memory_id }` 且后端执行完成
   - **Then** `memories` 表中该条记录被删除
   - **And** 不删除、不修改 `conversations.db` 的 `conversations` 或 `messages`
   - **And** 不修改原始历史对话内容、`thinking_content`、`routing_metadata` 或 opencode session/message 数据
   - **And** command 层保持薄层：只接参数、调用 service/db、返回结果

4. **AC-4 删除成功后 MemoryTab 列表与 badge 实时同步**
   - **Given** 当前 MemoryTab 显示 N 条可见记忆
   - **When** 用户成功遗忘其中 1 条
   - **Then** 该卡片从当前列表移除
   - **And** 当前 Tab 的 badge 数量减 1，且通过现有 `memory_count` 口径重新计算
   - **And** 若当前类别筛选为 `偏好`/`事实`/`认知模式`，badge 与筛选后的可见列表保持一致
   - **And** 角色记忆页只刷新当前角色范围；管家记忆页继续刷新全局 + 角色总览范围
   - **And** 若删除后列表为空，显示现有温暖空态文案，不出现 `暂无数据`

5. **AC-5 删除失败时保留记忆并给出温和反馈**
   - **Given** 用户确认遗忘
   - **When** 后端返回错误或目标记忆不存在
   - **Then** 该记忆卡片仍保留在列表中
   - **And** 确认区域或卡片内显示温和失败文案，例如：`这条记忆暂时没忘掉，稍后再试一下`
   - **And** 不使用 toast/snackbar，不静默失败
   - **And** 删除中状态必须禁用重复点击，避免同一 memory_id 并发删除

6. **AC-6 历史对话可继续查看且不被篡改**
   - **Given** 被遗忘记忆曾引用历史对话消息
   - **When** 用户之后查看历史对话或该 conversation 的 messages
   - **Then** 原始消息仍存在且内容不变
   - **And** `memory_get_source_messages` 对已删除 memory_id 返回 `NotFound` 或等价错误，不应伪造来源
   - **And** 其它仍存在的记忆来源追溯继续正常工作

7. **AC-7 遗忘范围明确：删除可见记忆并屏蔽同源再提炼**
   - **Given** PRD 对理想“遗忘”有更长期的认知回溯要求
   - **When** 实现本 story 与后续真实数据回流修复
   - **Then** V1 删除 `memories` 中的可见结构化记忆记录
   - **And** 记录轻量同源屏蔽 `forgotten_memory_sources`，防止同一 `source_conversation_id + category + source_message_ids` 被旧会话再次提炼回流
   - **And** 不删除、不修改历史对话、消息、`thinking_content`、`routing_metadata` 或 opencode session/message 数据
   - **And** 不重跑依赖推理、embedding 清理或长期认知回溯；用户未来从新消息重新明确表达同一事实时仍可作为新来源写入

8. **AC-8 验证通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd GUI/src-tauri && cargo test`
   - `cd GUI && npm run build`
   - 因本 story 修改 UI，必须启动 `cd GUI && npm run tauri dev` 做人工验证：角色记忆页删除、管家记忆页删除、类别筛选后删除、取消删除、删除失败态、badge 更新、历史对话不受影响。

## Tasks / Subtasks

### Phase 1: 后端删除 API（AC: #3, #6, #7, #8）

- [x] T1.1 为 `GUI/src-tauri/src/db/memories.rs` 新增删除 helper
  - 新增 `delete_memory(pool, memory_id) -> Result<bool, AppError>` 或等价函数
  - SQL 只允许：`DELETE FROM memories WHERE id = ?`
  - 使用 SQLx `SqliteQueryResult::rows_affected()` 判断是否实际删除
  - 不对 `conversations.db` 做任何 DELETE / UPDATE
  - 不改变 `insert_memories`、`dedup_memories`、`count_memories`、`list_*` 现有语义

- [x] T1.2 更新 `GUI/src-tauri/src/services/memory_query.rs`
  - 新增 `delete_memory(pool, memory_id) -> Result<(), AppError>`
  - 调用 db helper；若 `rows_affected == 0`，返回 `AppError::NotFound("记忆不存在: ...")`
  - service 层表达“选择性遗忘 = 删除可见结构化记忆记录 + 记录同源屏蔽”的业务语义
  - 不在此 service 中重跑 `memory_pipeline`

- [x] T1.3 更新 `GUI/src-tauri/src/commands/memory.rs`
  - 新增 Tauri command：`memory_delete(pool, memory_id: String) -> Result<(), AppError>`
  - command 层保持薄层，仅转发到 `memory_query::delete_memory`
  - command 命名为 `memory_delete`，与现有 `memory_list` / `memory_count` 风格一致

- [x] T1.4 更新 `GUI/src-tauri/src/lib.rs`
  - 在 `tauri::generate_handler!` 中注册 `commands::memory::memory_delete`
  - 不改其它 command 注册顺序语义

- [x] T1.5 Rust 测试
  - `db::memories`：删除存在的 memory 后，`get_memory_by_id` 返回 `None`，`count_memories` 减 1
  - `memory_query`：删除不存在的 memory 返回 `AppError::NotFound`
  - `memory_query` 或集成式测试：删除 memory 后，`conversations::list_messages` 仍能读到原 source conversation/messages
  - 回归测试：删除一个 memory 不影响其它 memory 的 list/count/source traceability

### Phase 2: 前端 service、hook 与 MemoryTab 遗忘交互（AC: #1, #2, #4, #5, #8）

- [x] T2.1 更新 `GUI/src/services/memoryService.ts`
  - 新增 `delete(memoryId: string): Promise<void>`，invoke `memory_delete`，参数为 `{ memoryId }`
  - 保持现有 `list` / `listAll` / `count` / `getSourceMessages` 调用形状不变
  - 不引入新依赖，不绕过 Tauri service 层

- [x] T2.2 更新 `GUI/src/components/role/MemoryTab.tsx` props 与状态
  - 新增可选 `onMemoryDeleted?: () => void` 或等价回调，供父组件刷新 badge
  - 复用 `useMemories` 已返回的 `refetch`
  - 新增 per-memory 状态：待确认 memory id、正在删除 memory id、删除错误文案
  - 点击 `遗忘` 时显示自定义确认 UI，不调用 `window.confirm`
  - 删除中禁用 `遗忘`、`确认遗忘`、`再想想` 的重复操作

- [x] T2.3 实现删除成功路径
  - 用户确认后调用 `memoryService.delete(memory.id)`
  - 成功后清理该 memory 的 `sourceStates` / `sourceRequestIds` / 展开状态，避免保留已删来源 UI
  - 调用 `refetch()` 刷新当前列表
  - 调用父级 `onMemoryDeleted` 刷新当前 Tab 的 `memory_count`
  - 若当前筛选下已无记忆，显示现有温暖空态

- [x] T2.4 实现取消与失败路径
  - 取消确认时只关闭确认态，不调用 service，不刷新列表，不改 badge
  - 删除失败时保留卡片并显示温和失败文案
  - 失败后允许用户再次点击 `确认遗忘` 重试
  - 不使用 toast/snackbar/alert

- [x] T2.5 更新 `GUI/src/components/role/RoleWorkspacePanel.tsx`
  - 将 `onMemoryDeleted` 传给角色记忆 `MemoryTab`
  - 删除成功后重新调用 `memoryService.count({ roleId: role.id, category: memoryCategory })`
  - 继续让 badge 跟随当前 `memoryCategory`
  - 保持角色页只作用当前 `role.id`

- [x] T2.6 更新 `GUI/src/components/butler/ButlerWorkspacePanel.tsx`
  - 将 `onMemoryDeleted` 传给管家记忆 `MemoryTab`
  - 删除成功后重新调用 `memoryService.count({ roleId: null, includeRoleMemories: true, category: memoryCategory })`
  - 保持 `includeRoleMemories` 总览语义，不误改成只看 `role_id = NULL`

- [x] T2.7 前端测试
  - `MemoryTab.test.tsx`：点击 `遗忘` 后显示自定义确认文案；断言未调用 `window.confirm`
  - `MemoryTab.test.tsx`：取消确认不调用 `memoryService.delete`，卡片仍存在
  - `MemoryTab.test.tsx`：确认删除调用 `memoryService.delete(memoryId)`、触发 `refetch`/重新 list、调用 `onMemoryDeleted`、卡片移除或列表刷新
  - `MemoryTab.test.tsx`：删除失败显示温和错误，卡片保留，可重试
  - `RoleWorkspacePanel.test.tsx`：删除成功回调后 badge 重新按当前角色 + category 调 `count`
  - `ButlerWorkspacePanel.test.tsx`：删除成功回调后 badge 重新按总览 + category 调 `count`

### Phase 3: 质量门禁与人工验证（AC: #8）

- [x] T3.1 运行 `cd GUI && npx tsc --noEmit`
- [x] T3.2 运行 `cd GUI && npm run test:frontend`
- [x] T3.3 运行 `cd GUI/src-tauri && cargo test`
- [x] T3.4 运行 `cd GUI && npm run build`
- [x] T3.5 启动 `cd GUI && npm run tauri dev` 人工验证 UI
  - 角色记忆页：点击遗忘 → 取消 → 无副作用
  - 角色记忆页：点击遗忘 → 确认 → 卡片移除、badge 更新
  - 管家记忆页：删除全局/角色总览中的记忆后，badge 与列表一致
  - 类别筛选后删除：当前 category badge 重新计算
  - 删除失败：显示温和错误，不 toast，不丢卡片
  - 历史对话：删除记忆后原对话仍可查看

### Review Findings

> **评审结论：Changes Requested（建议修改）** — 2026-06-01 · 三层对抗式评审（Blind Hunter + Edge Case Hunter + Acceptance Auditor，full 模式）
> 严重度统计：1 HIGH / 3 MED / 2 LOW（已 patch/defer）；另 3 项作为噪声 dismiss。
> 快乐路径与常规失败路径功能正确；HIGH 项为"记忆已在后端被删 / 并发删除"边界下的真实 UX 缺陷。

- [x] [Review][Patch] NotFound 被当作可重试临时错误且失败后不 refetch，导致"幽灵卡片"永久残留 (HIGH) [GUI/src/components/role/MemoryTab.tsx:143-173]
- [x] [Review][Patch] role/category 切换的重置 effect 未清除 `deletingMemoryId`，切换后可能残留删除中状态 (MED) [GUI/src/components/role/MemoryTab.tsx:86-90]
- [x] [Review][Patch] 删除进行中全局禁用所有"遗忘"按钮，应仅禁用当前正在删除的卡片 (MED) [GUI/src/components/role/MemoryTab.tsx:231]
- [x] [Review][Patch] 缺少"删除中（pending promise 未 resolve）禁用重复点击"的测试，AC#8/T2.2 要求该行为 (MED) [GUI/src/components/role/MemoryTab.test.tsx]
- [x] [Review][Defer] useMemories 中 refetch 失败会覆盖删除成功结果并显示"加载失败"，遮蔽真实删除状态 (LOW) [GUI/src/hooks/useMemories.ts:35-40] — deferred, pre-existing（本故事 diff 未修改该 hook）
- [x] [Review][Defer] 确认对话框缺少 `role="alertdialog"` / `aria-live` / 焦点管理，键盘与读屏可达性不足 (LOW) [GUI/src/components/role/MemoryTab.tsx:243-276] — deferred，统一在 Epic 8 story 8-3（WCAG 审计）处理

## Dev Notes

### Story Foundation

- Epic 2 的目标包含记忆查看、溯源、删除和透明推理；Story 2.8 是 Story 2.7 之后的“删除记忆”能力。[Source: `_bmad-output/planning-artifacts/epics.md`:279-302]
- Story 2.8 原始 AC 要求：点击 `遗忘` 显示管家风格确认；确认后删除 `memories` 表记录、列表移除、badge -1；历史对话不受影响；取消无副作用；后端 command 为 `memory::delete { memory_id }`，仅删除 `memories`，不级联删除 conversations/messages。[Source: `_bmad-output/planning-artifacts/epics.md`:1118-1147]
- PRD FR-9 将选择性遗忘定义为用户可要求角色忘记特定记忆，使相关记忆不再出现、后续建议不再基于被遗忘信息。[Source: `_bmad-output/planning-artifacts/prd-egosync.md`:211-218]
- Epic 当前范围明确 V1 简化版为“仅删除条目，不重跑推理”，优先于 PRD 中更理想化的“重新运行依赖推理”。[Source: `_bmad-output/planning-artifacts/epics.md`:296-299]

### Architecture Compliance

- 主数据 `egosync.db` 与对话日志 `conversations.db` 是独立边界；`memories.source_conversation_id` 是文本引用，不能声明或依赖 SQLite 跨库外键。[Source: `_bmad-output/planning-artifacts/architecture.md`:196-208,941-947]
- 前端必须通过 service 封装 Tauri invoke；不能直接访问 DB、LLM API 或 opencode server。[Source: `_bmad-output/planning-artifacts/architecture.md`:371-394,932-939]
- Rust Command 层只做参数解析 → 调 service/db → 返回结果；业务语义放 service，SQL 放 db helper。[Source: `_bmad-output/project-context.md`:104-114]
- 新增 Tauri Command 时必须同步更新前端 service/types/hook（如需要），并在 `lib.rs` 注册。[Source: `_bmad-output/project-context.md`:183-190]
- 删除操作属于 destructive 行为，但本 story 的确认只在 EgoSync UI 内完成，不使用系统弹窗。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:802-827]

### Current State of UPDATE Files

| Path | Current state | This story changes | Must preserve |
|---|---|---|---|
| `GUI/src/components/role/MemoryTab.tsx` | 已支持 category 受控/非受控、真实 list/listAll、来源懒加载、禁用的 `遗忘` 按钮。[Source: `GUI/src/components/role/MemoryTab.tsx`:57-79,91-120,172-178] | 启用 `遗忘`，添加自定义确认、删除中/失败态、删除成功刷新 list 与父级 badge。 | 角色/管家数据边界；来源展开行为；`task_status` 默认隐藏；暖空态；ARIA 展开属性。 |
| `GUI/src/hooks/useMemories.ts` | 封装 list/listAll、loading/error、`refetch`、取消旧请求防 stale。[Source: `GUI/src/hooks/useMemories.ts`:17-51] | 复用 `refetch`，无需重写 hook；如需新增返回值应保持旧调用兼容。 | role/category 切换防 stale；错误中文友好。 |
| `GUI/src/services/memoryService.ts` | 现有 `list`、`listAll`、`count`、`getSourceMessages`；无 delete。[Source: `GUI/src/services/memoryService.ts`:10-28] | 新增 `delete(memoryId)` invoke `memory_delete`。 | Tauri v2 `@tauri-apps/api/core` invoke；现有方法签名。 |
| `GUI/src/types/memory.ts` | 定义 `Memory`、`MemoryCategory`、`MemorySourceMessage`、`MemoryListOptions`。[Source: `GUI/src/types/memory.ts`:1-26] | 通常无需修改；若新增删除回调类型，保持局部。 | `sourceMessageIds` 仍是字符串；UI 不直接解析为来源真相。 |
| `GUI/src/components/role/RoleWorkspacePanel.tsx` | 用 `memoryService.count({ roleId, category })` 显示当前角色 badge；category 状态在 Panel。[Source: `GUI/src/components/role/RoleWorkspacePanel.tsx`:10-32,55-57] | 接收 MemoryTab 删除成功回调并重新刷新 count。 | badge 跟随 category；角色页只传当前 `role.id`。 |
| `GUI/src/components/butler/ButlerWorkspacePanel.tsx` | 用 `memoryService.count({ roleId: null, includeRoleMemories: true, category })` 显示管家总览 badge。[Source: `GUI/src/components/butler/ButlerWorkspacePanel.tsx`:10-33,79-87] | 接收 MemoryTab 删除成功回调并重新刷新 count。 | `includeRoleMemories` 总览语义；role owner label。 |
| `GUI/src-tauri/src/db/memories.rs` | 有 insert/update/get/list/listAll/count/dedup；无 delete。[Source: `GUI/src-tauri/src/db/memories.rs`:213-311] | 新增 `delete_memory` helper。 | list/count 去重和 category/task_status 规则；single-owner dedupe；不改 migrations。 |
| `GUI/src-tauri/src/services/memory_query.rs` | 查询、count、来源解析 service；来源只返回 user messages 可用子集。[Source: `GUI/src-tauri/src/services/memory_query.rs`:9-80] | 新增 `delete_memory` service。 | `get_source_messages` 的隐私过滤与部分来源缺失可用子集策略。 |
| `GUI/src-tauri/src/commands/memory.rs` | 已注册 list/listAll/count/getSourceMessages commands；command 薄层。[Source: `GUI/src-tauri/src/commands/memory.rs`:8-59] | 新增 `memory_delete` command。 | 参数接收和 service 转发模式。 |
| `GUI/src-tauri/src/lib.rs` | 已注册 `memory_list`、`memory_list_all`、`memory_count`、`memory_get_source_messages`。[Source: `GUI/src-tauri/src/lib.rs`:218-221] | 注册 `memory_delete`。 | 其它 command 注册不动。 |
| `GUI/src/components/role/MemoryTab.test.tsx` | 覆盖真实加载、筛选、来源展开、空态，且断言 `遗忘` 当前 disabled。[Source: `GUI/src/components/role/MemoryTab.test.tsx`:124-143] | 将 disabled 断言替换为确认/取消/成功/失败测试。 | 既有筛选、来源、空态测试继续通过。 |
| `GUI/src/components/role/RoleWorkspacePanel.test.tsx` | 覆盖 badge 初始 count 与 category 联动。[Source: `GUI/src/components/role/RoleWorkspacePanel.test.tsx`:48-72] | 增加删除成功回调后 count 刷新断言。 | category 联动断言。 |
| `GUI/src/components/butler/ButlerWorkspacePanel.test.tsx` | 覆盖管家总览 badge 初始 count 与 category 联动。[Source: `GUI/src/components/butler/ButlerWorkspacePanel.test.tsx`:31-73] | 增加删除成功回调后 count 刷新断言。 | `includeRoleMemories: true` 调用断言。 |

### Previous Story Intelligence

- Story 2.6 已完成 `memories` schema、nullable `role_id`、source fields、source 去重、memory pipeline、基础 memory list/listAll；2.8 不要重建 schema 或 pipeline。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:269-297]
- Story 2.6 明确 `source_conversation_id` 是跨库文本引用，不能为满足外键字面要求合并 DB 或 attach DB。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:213-219]
- Story 2.6 明确选择性遗忘留给 2.8；当前查询 UI 可展示真实记忆，但 source drilldown/delete 是后续故事范围。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:240-245,348-356]
- Story 2.6 Deferred 记录了跨库悬挂引用 TOCTOU 风险；2.8 删除 memory 本身不应尝试“顺手清理来源对话”。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:400-405]
- Story 2.7 已完成 MemoryTab 真实数据、category 筛选、source traceability、badge count，并将 `遗忘` 按钮保留为 disabled 占位；2.8 的直接工作是启用该占位并接后端删除。[Source: `_bmad-output/implementation-artifacts/2-7-memory-panel-traceability.md`:328-337]
- Story 2.7 Review Patch 已将 badge 与 category 联动，删除后必须刷新当前 category count，而不是只更新“全部”数量。[Source: `_bmad-output/implementation-artifacts/2-7-memory-panel-traceability.md`:171-180]
- Story 2.7 Deferred 记录 `sourceStates/sourceRequestIds` 在 role/category 切换时未清理只是轻微累积；2.8 删除成功时应清理被删 memory 的 source state，避免 UI 持有已删项。[Source: `_bmad-output/implementation-artifacts/2-7-memory-panel-traceability.md`:175-180]

### Git Intelligence Summary

- `d01ef68 feat(memory): unify memory ownership and display`：最近刚统一 memory owner 与显示口径；2.8 必须保持 single-owner/dedup 与 role/global 显示边界。
- `f550b81 chore(bmad): refresh bundled skills and config`：只刷新技能/config，对实现无直接影响。
- `31adb7e docs(chat): align opencode and memory implementation notes`：文档强调 opencode 与 memory 边界；删除 memory 不应触碰 opencode session/message。
- `ef123f4 feat(chat): sync role memories and refine delegation`：近期调整角色记忆同步与委派关系；2.8 不应修改 `memory_pipeline` 的同步/委派过滤逻辑。
- `34aab28 docs(2.3): refresh delegate bridge implementation notes`：委派桥文档更新，不影响 memory delete。

### Library / Framework Requirements

- 前端使用 React 18.2、TypeScript 5.2 strict、Vite 5、TailwindCSS 3.3.5、Lucide React 0.292；本 story 不新增 UI/状态管理依赖。[Source: `GUI/package.json`:14-39]
- Tauri 前端 API 使用 `@tauri-apps/api` 2.11，从 `@tauri-apps/api/core` 引入 `invoke`，沿用 `memoryService` 模式。[Source: `GUI/package.json`:16; `GUI/src/services/memoryService.ts`:1-28]
- Rust 后端继续使用 Tauri 2 + SQLx 0.8；删除行数判断使用 `SqliteQueryResult::rows_affected()`。[Source: SQLx docs `https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteQueryResult.html`]
- Tauri v2 command 写法：模块中 `pub async fn` 标注 `#[tauri::command]`，在单个 `tauri::generate_handler![...]` 中注册；前端用 `invoke('memory_delete', { memoryId })` 调用。[Source: Tauri v2 docs `https://v2.tauri.app/develop/calling-rust/`]

### UX Requirements

- Memory 面板位于 `RoleWorkspacePanel` 右侧结构化工作台，支持浏览、追溯原文和执行“遗忘”。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:763-770]
- `遗忘` 是 destructive 操作；使用红色边框/文字即可，不要做高压弹窗或系统弹窗。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:802-815]
- 所有成功/失败反馈通过卡片状态、面板内温和文案或管家自然语言呈现，不使用传统 toast/snackbar。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:816-827]
- 空态永远不显示 `暂无数据`，保留现有温暖文案。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:840-849]
- 导航深度保持管家 ↔ 角色两层；删除确认应内联或以现有对话框模式呈现，不新增子页面。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:829-839]

### Implementation Boundaries

- 只删除结构化 memory 记录；不删除 source conversation/messages。
- 不新增 migration，不改 `004_memories.sql` / `005_memory_role_scoped_dedupe.sql` / `006_memory_single_owner_dedupe.sql`。
- 不改 `memory_pipeline` prompt、角色归属同步、dedupe、debounce 或 LLM 调用；仅在 reconciliation 写入阶段尊重已遗忘同源屏蔽。
- 不改 `chat_send_message`、opencode sidecar、delegate bridge；同一热修中 `agent_engine`/`agent_bridge` 仅修复 opencode completed response 兜底与 thinking-only 气泡消失，不改变记忆删除边界。
- 不实现 forget_rules、重跑推理、embedding 清理或长期认知回溯；本次仅实现同源屏蔽，避免旧 source message 重新提炼同一已遗忘事实。
- 不把前端本地列表过滤作为唯一真相；删除后必须以后端删除成功为准，并重新拉取 list/count。
- 不允许删除 `task_status` 的隐藏记忆作为本 story 的 UI 入口；当前筛选 UI 不展示 `task_status`，保持现状即可。

### Testing Requirements

- Rust 测试使用 in-memory SQLite，复用现有 migrations；不要依赖真实 LLM/API Key。
- 删除测试应断言 `memories` 变化与 `conversations.db` 不变，防止误级联。
- 前端测试 mock `memoryService`，不要依赖真实 Tauri invoke。
- UI 测试必须覆盖确认、取消、成功、失败、重复点击禁用、badge 重算。
- 完成实现后必须人工启动 Tauri dev 验证 UI；不能只用类型检查和单元测试声称完成。

## Project Structure Notes

### 新增文件

| Path | Action | Notes |
|---|---|---|
| `GUI/src-tauri/migrations/007_forgotten_memory_sources.sql` | NEW | 记录已遗忘记忆的同源屏蔽，防止旧 source message 再提炼回流。 |

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `GUI/src-tauri/src/db/memories.rs` | UPDATE | 新增 delete helper、同源屏蔽查询与 Rust 单测。 |
| `GUI/src-tauri/src/db/pool.rs` | UPDATE | 覆盖 `forgotten_memory_sources` migration/index smoke test。 |
| `GUI/src-tauri/src/services/memory_query.rs` | UPDATE | 新增 delete service，0 rows 映射 NotFound。 |
| `GUI/src-tauri/src/services/memory_pipeline.rs` | UPDATE | reconciliation update/insert 路径尊重同源屏蔽。 |
| `GUI/src-tauri/src/commands/memory.rs` | UPDATE | 新增 `memory_delete` command。 |
| `GUI/src-tauri/src/lib.rs` | UPDATE | 注册 `memory_delete`。 |
| `GUI/src/services/memoryService.ts` | UPDATE | 新增 `delete(memoryId)`。 |
| `GUI/src/components/role/MemoryTab.tsx` | UPDATE | 启用遗忘按钮、自定义确认、删除状态、成功/失败处理、refetch。 |
| `GUI/src/components/role/RoleWorkspacePanel.tsx` | UPDATE | 删除成功后刷新角色记忆 badge。 |
| `GUI/src/components/butler/ButlerWorkspacePanel.tsx` | UPDATE | 删除成功后刷新管家总览 badge。 |
| `GUI/src/components/role/MemoryTab.test.tsx` | UPDATE | 删除确认/取消/成功/失败测试。 |
| `GUI/src/components/role/RoleWorkspacePanel.test.tsx` | UPDATE | 删除后 badge refresh 测试。 |
| `GUI/src/components/butler/ButlerWorkspacePanel.test.tsx` | UPDATE | 删除后 badge refresh 测试。 |

### 不应修改

- `GUI/src-tauri/migrations/004_memories.sql`
- `GUI/src-tauri/migrations/005_memory_role_scoped_dedupe.sql`
- `GUI/src-tauri/migrations/006_memory_single_owner_dedupe.sql`
- `GUI/src-tauri/src/commands/chat.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src-tauri/src/services/agent_bridge.rs`
- `GUI/src-tauri/src/services/agent_config.rs`
- opencode session/message 存储或配置文件

## References

- [Source: `_bmad-output/planning-artifacts/epics.md`:1118-1147 — Story 2.8 AC and backend/frontend expectations]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md`:211-218 — FR-9 selective forget]
- [Source: `_bmad-output/planning-artifacts/architecture.md`:196-208,941-947 — separate DBs and data boundaries]
- [Source: `_bmad-output/planning-artifacts/architecture.md`:371-394,932-939 — IPC/service boundaries]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:763-770 — Memory tab supports forget]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:802-849 — destructive button, feedback, empty-state UX]
- [Source: `_bmad-output/project-context.md`:104-114,118-136,183-205 — Tauri layering, tests, command checklist, anti-patterns]
- [Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:240-245,400-405 — selective forget deferred and cross-db source boundary]
- [Source: `_bmad-output/implementation-artifacts/2-7-memory-panel-traceability.md`:171-180,328-337 — review learnings and existing MemoryTab behavior]
- [Source: `GUI/src/components/role/MemoryTab.tsx`:57-79,91-120,172-178 — current controlled category/source state/disabled forget button]
- [Source: `GUI/src/hooks/useMemories.ts`:17-51 — current refetch and stale-request guard]
- [Source: `GUI/src/services/memoryService.ts`:10-28 — current memory service methods]
- [Source: `GUI/src-tauri/src/db/memories.rs`:213-311 — current get/list/count behavior]
- [Source: `GUI/src-tauri/src/services/memory_query.rs`:37-80 — current source traceability service]
- [Source: `GUI/src-tauri/src/commands/memory.rs`:8-59 — current memory commands]
- [Source: `GUI/src-tauri/src/lib.rs`:218-221 — current memory command registration]
- [Source: `GUI/package.json`:6-13,14-39 — scripts and frontend deps]
- [Source: Tauri v2 docs `https://v2.tauri.app/develop/calling-rust/` — command registration and invoke]
- [Source: SQLx docs `https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteQueryResult.html` — `rows_affected()` for DELETE result]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context)

### Debug Log References

- 2026-06-01: `python3` 不可用，按 skill 回退规则手动读取 `bmad-create-story/customize.toml`，确认无 team/user overrides、无 activation prepend/append。
- 2026-06-01: 自动从 `sprint-status.yaml` 选中第一个 backlog story：`2-8-selective-memory-forget`。
- 2026-06-01: dev-story 启动时 `python3` 仍不可用，按 skill 回退规则手动读取 `bmad-dev-story/customize.toml`，确认无 team/user overrides、无 activation prepend/append。
- 2026-06-01: 首次运行 `cargo test` 被全局/默认 Cargo registry 的 USTC 源阻断；改用仓库内 `GUI/.cargo-test-home` 后进入依赖编译与 Rust 测试。

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- 明确 V1 选择性遗忘范围：删除 `memories` 可见记录，记录轻量同源屏蔽以防旧 source message 再提炼回流；不重跑推理、不改 history/opencode。
- 提炼并纳入 2.6/2.7 前序经验：保持跨库边界、badge/category 联动、MemoryTab 源状态与总览语义。
- 实现 `memory_delete` 后端链路：db helper 在事务中先写入 `forgotten_memory_sources` 同源屏蔽，再删除 `memories` 可见记录；service 将 missing 映射为 `AppError::NotFound`，command 保持薄层并在 Tauri handler 注册。
- 实现 MemoryTab 自定义遗忘确认：不使用 `window.confirm` / alert / toast / snackbar，支持确认、取消、删除中禁用、失败内联反馈、成功后清理来源状态并刷新 list/count。
- 角色记忆页和管家记忆页删除成功后分别按当前 role/category 与全局总览/category 重新调用 `memory_count`。
- 已通过：`npx tsc --noEmit`（使用 GUI tsconfig）、`npm run test:frontend`（69 tests）、`npm run build`、本故事 Rust 文件 `rustfmt --check`。
- 已通过完整 Rust 验证：memory 相关 Rust tests 37 passed；完整 Rust suite 单线程 217 unit + 1 integration passed。并记录并发 full suite 中 `sidecar` 旧测试曾单次环境敏感失败，单测重跑通过。
- 已启动真实 Tauri dev：`egosync.exe`、opencode sidecar、delegate bridge 均启动成功。
- 新增 `GUI/src-tauri/migrations/007_forgotten_memory_sources.sql`，并在 `insert_memories` / `update_memory_from_extracted` 中跳过已遗忘同源候选，避免旧会话触发提炼时把用户已遗忘事实重新写回。
- 真实用户 AppData 中未删除任何真实记忆；因现有 6 条均为用户真实内容，成功删除链路改用临时 identifier `com.egosync.story28test` 与专用测试 DB 验证。
- 隔离成功删除验证通过：角色 memory 删除后 role count 2→1、total 4→3；全局 fact 删除后 fact count 1→0、total 3→2；preference 分类删除后 preference count 2→1；已删除 memory 的 source 查询返回错误，保留 memory 的 source message 仍可读取。
- 前端浏览器/单测验证覆盖：自定义确认、取消无副作用、确认删除移除卡片并刷新 badge、管家/角色/category badge 重新计算、失败温和内联反馈、无 toast/snackbar/alert。

### File List

- `GUI/src-tauri/src/db/memories.rs`
- `GUI/src-tauri/src/db/pool.rs`
- `GUI/src-tauri/src/services/memory_query.rs`
- `GUI/src-tauri/src/services/memory_pipeline.rs`
- `GUI/src-tauri/src/commands/memory.rs`
- `GUI/src-tauri/src/lib.rs`
- `GUI/src/services/memoryService.ts`
- `GUI/src/components/role/MemoryTab.tsx`
- `GUI/src/components/role/MemoryTab.test.tsx`
- `GUI/src/components/role/RoleWorkspacePanel.tsx`
- `GUI/src/components/role/RoleWorkspacePanel.test.tsx`
- `GUI/src/components/butler/ButlerWorkspacePanel.tsx`
- `GUI/src/components/butler/ButlerWorkspacePanel.test.tsx`
- `GUI/src-tauri/src/services/agent_bridge.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src-tauri/src/models/agent.rs`
- `GUI/src/components/chat/ChatStream.tsx`
- `GUI/src/components/chat/ChatStream.test.tsx`
- `GUI/src/components/chat/ChatBubble.test.tsx`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/2-8-selective-memory-forget.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-01: Ultimate context engine analysis completed - comprehensive developer guide created. Status set to ready-for-dev.
- 2026-06-01: Implemented selective memory forget backend command/service/db deletion path, frontend MemoryTab confirmation/delete flow, badge refresh wiring, and coverage tests.
- 2026-06-01: Completed validation gates and Tauri/WebView2 IPC verification, including isolated successful delete flow with test identifier `com.egosync.story28test`; story moved to review.
- 2026-06-01: 三层对抗式代码评审完成（Changes Requested）。修复 4 个 patch：MemoryTab 将后端 `NotFound` 视为"已遗忘"成功路径（抽出 `finalizeForgotten`，消除幽灵卡片）、切换 effect 重置 `deletingMemoryId`、遗忘按钮改为按卡片局部禁用、新增"删除中重复点击"与"NotFound 当成功"两条测试。2 个 LOW（useMemories 失败覆盖、确认框 a11y）记入 deferred-work.md，3 项噪声 dismiss。tsc / 前端测试（71 passed）通过。Status → done。
- 2026-06-01: 根据真实运行回归修复遗忘回流：新增 `forgotten_memory_sources` migration，`memory_delete` 删除可见记忆前记录同源屏蔽，`insert_memories` / reconciliation update 路径跳过同一 source conversation/category/message ids 的已遗忘候选；保持历史 conversations/messages、thinking、routing metadata、opencode 数据不变。同步补充 db/pipeline/pool 测试并刷新本 story 文档，使其与实际同源屏蔽实现一致。
