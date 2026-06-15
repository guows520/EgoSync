---
baseline_commit: f550b817924c2aef5353d55a7863ed5595bffbd7
created_at: 2026-05-31T20:22:51+08:00
---

# Story 2.7: 用户能在记忆面板查看、追溯记忆来源

Status: done

## Story

As a 用户,
I want 在管家和角色的记忆面板中查看已积累的结构化记忆，并展开每条记忆看到原始来源对话,
so that 我能理解 AI 为什么记住这些信息，并判断这些记忆是否可信。

## Acceptance Criteria

1. **AC-1 记忆列表展示真实数据且保持归属边界**
   - **Given** 用户在角色视图打开 `记忆档案` Tab
   - **When** 该角色有 5 条记忆
   - **Then** 按 `created_at DESC` 显示 5 张记忆卡片
   - **And** 每张卡片显示：类别标签、内容摘要、创建时间、来源对话入口
   - **And** 角色页只显示 `role_id = <当前角色 id>` 的记忆
   - **Given** 用户在管家视图打开 `管家记忆` Tab
   - **Then** 保留现有 `includeRoleMemories` 行为，显示全局记忆与角色记忆的总览，不误改成只看 `role_id = NULL`

2. **AC-2 类别筛选可用**
   - **Given** 记忆面板顶部有类别筛选器
   - **When** 用户选择 `偏好`
   - **Then** 仅显示 `category = 'preference'` 的记忆
   - **And** 筛选器支持：全部、偏好、任务状态、认知模式、事实
   - **And** 角色页筛选只作用于当前角色记忆；管家页筛选作用于全量总览记忆

3. **AC-3 来源对话可追溯且高亮来源消息**
   - **Given** 用户点击某条记忆卡片的 `查看原文`
   - **When** 前端调用 `memory_get_source_messages { memory_id }`
   - **Then** Rust 后端从主库 `memories` 读取 `source_conversation_id` 与 `source_message_ids`
   - **And** 再从独立 `conversations.db` 读取该 conversation 的 messages
   - **And** 返回 `source_message_ids` 对应的原始消息，按原始对话顺序排列
   - **And** 前端展开区显示这些原文片段，并用左边框 / 浅色背景高亮来源消息
   - **And** 不暴露 `thinking_content`、`routing_metadata` 或内部 system 脚手架消息

4. **AC-4 记忆数量 badge 显示正确**
   - **Given** 角色有 12 条记忆
   - **Then** `记忆档案` Tab 标签显示 `记忆档案 (12)` 或等价 badge
   - **Given** 管家记忆总览包含全局与角色记忆共 18 条
   - **Then** `管家记忆` Tab 标签显示总数 badge
   - **And** badge 计数与列表去重后的可见记忆数量一致，不把历史重复来源记忆重复计数

5. **AC-5 空态、错误态和可访问性符合 UX 约束**
   - **Given** 当前角色无记忆
   - **Then** 显示温暖空态文案：`还没有记忆，多和这个角色聊聊吧`
   - **Given** 管家记忆总览无记忆
   - **Then** 显示温暖空态文案：`还没有记忆，多聊几次，我会慢慢记住重要的事`
   - **And** 不出现 `暂无数据` 这类冷冰冰文案
   - **Given** 来源 conversation 已被删除或来源消息缺失
   - **Then** 展开区显示 `来源对话已不可用`，不崩溃、不报 toast
   - **And** 展开按钮保留 `aria-expanded` / `aria-controls`，筛选按钮支持键盘操作和清晰 focus 状态

6. **AC-6 后端 API 与数据边界正确**
   - **Given** Rust 后端
   - **Then** 现有 Tauri commands `memory_list` / `memory_list_all` 支持可选 `category`、`limit`、`offset`
   - **And** 新增 Tauri command：`memory_count { role_id?, include_role_memories?, category? }`
   - **And** 新增 Tauri command：`memory_get_source_messages { memory_id }`
   - **And** commands 层保持薄层；跨 `egosync.db` 与 `conversations.db` 的来源解析放在 service/db helper 中
   - **And** 不新增 SQLite 跨库外键，不修改 `004_memories.sql` / `005_memory_role_scoped_dedupe.sql`

7. **AC-7 测试与验证通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd egosync-app/src-tauri && cargo test`
   - `cd GUI && npm run build`
   - 因本 story 修改 UI，必须启动 `cd GUI && npm run tauri dev` 做人工验证：打开角色记忆页、管家记忆页，验证筛选、badge、展开来源、缺失来源空态。

## Tasks / Subtasks

### Phase 1: 后端查询与溯源 API（AC: #2, #3, #4, #6, #7）

- [x] T1.1 更新 `egosync-app/src-tauri/src/models/memory.rs`
  - 新增 `MemorySourceMessage` DTO：`id`, `conversation_id`, `role`, `content`, `created_at`, `is_source`
  - 使用 `#[derive(Debug, Clone, Serialize, Deserialize)]` 与 `#[serde(rename_all = "camelCase")]`
  - 不复用 `models::chat::Message` 直接返回前端，避免暴露 `thinking_content` / `routing_metadata`

- [x] T1.2 更新 `egosync-app/src-tauri/src/db/memories.rs`
  - 新增 `get_memory_by_id(pool, memory_id) -> Result<Option<Memory>, AppError>`
  - 扩展 `list_memories(pool, role_id, category, limit, offset)`；`category` 必须复用现有白名单校验
  - 扩展 `list_all_memories(pool, category, limit, offset)`，保持当前 `dedup_memories` 去重语义
  - 新增 `count_memories(...)`，计数必须与去重后的列表一致；可复用 list 后 `len()`，V1 不为此引入复杂 SQL
  - 不改 `insert_memories` 的来源归一化与去重逻辑

- [x] T1.3 新增或更新查询 service（建议 `egosync-app/src-tauri/src/services/memory_query.rs`）
  - `list_role_memories(...)` / `list_all_memories(...)` / `count_memories(...)`
  - `get_source_messages(main_pool, conv_pool, memory_id)`：
    - 主库查 memory；不存在返回 `AppError::NotFound`
    - 解析 `source_message_ids` JSON 数组；非法 JSON 返回 `ValidationError`
    - 调 `db::conversations::list_messages` 读取同 conversation 消息
    - 只返回 id 命中的消息，按 `list_messages` 原始顺序
    - 若 conversation 或消息已删除，返回空数组并写 warn，不 panic
  - 更新 `egosync-app/src-tauri/src/services/mod.rs` 导出该 service

- [x] T1.4 更新 `egosync-app/src-tauri/src/commands/memory.rs`
  - `memory_list(pool, role_id, category, limit, offset)` 调 service/db 查询
  - `memory_list_all(pool, category, limit, offset)` 调 service/db 查询
  - 新增 `memory_count(pool, role_id, include_role_memories, category)`
  - 新增 `memory_get_source_messages(pool, conversations_pool, memory_id)`
  - 保持 command 层仅做参数接收和调用，不把跨库过滤逻辑塞进 command

- [x] T1.5 更新 `egosync-app/src-tauri/src/lib.rs`
  - 在 `tauri::generate_handler!` 注册 `memory_count` 与 `memory_get_source_messages`
  - 不改已有 chat/role/opencode command 注册顺序语义

- [x] T1.6 Rust 测试
  - `db::memories`：category filter、limit/offset、count 与 dedup 后数量一致
  - source query：返回 source ids 对应消息，顺序与原始对话一致
  - source query：missing memory → `NotFound`
  - source query：source conversation/message 缺失 → `Ok(vec![])`，不崩溃
  - source query：返回 DTO 不包含 thinking/routing 字段

### Phase 2: 前端 service、hook 与 MemoryTab（AC: #1, #2, #3, #5, #7）

- [x] T2.1 更新 `egosync-app/src/types/memory.ts`
  - 提取 `MemoryCategory = 'preference' | 'task_status' | 'cognition_update' | 'fact'`，其中 `cognition_update` 的用户可见标签统一显示为 `认知模式`
  - `Memory.sourceMessageIds` 可暂时保持 `string`，不在 UI 直接解析作为真相来源
  - 新增 `MemorySourceMessage` 与 `MemoryListOptions`

- [x] T2.2 更新 `egosync-app/src/services/memoryService.ts`
  - `list(roleId, options?)` → invoke `memory_list`
  - `listAll(options?)` → invoke `memory_list_all`
  - 新增 `count({ roleId, includeRoleMemories, category? })` → invoke `memory_count`
  - 新增 `getSourceMessages(memoryId)` → invoke `memory_get_source_messages`
  - 继续使用 `@tauri-apps/api/core` 的 `invoke`，不引入新依赖

- [x] T2.3 新增 `egosync-app/src/hooks/useMemories.ts`
  - 参数：`roleId`, `includeRoleMemories`, `category`
  - 返回：`memories`, `isLoading`, `error`, `refetch`
  - 内部封装现有 `MemoryTab` 的取消标记模式，避免 role 切换后旧请求覆盖新状态
  - 错误状态只返回中文友好文案，不 toast

- [x] T2.4 更新 `egosync-app/src/components/role/MemoryTab.tsx`
  - 用 `useMemories` 替换组件内直接 list/listAll 调用
  - 添加类别筛选器；选中状态可见，键盘可操作
  - 展开来源时懒加载 `memoryService.getSourceMessages(memory.id)`，每条 memory 独立 loading/error/source state
  - 展开区渲染 `MemorySourceMessage`：用户/助手角色标识、时间、原文内容；`isSource=true` 的消息高亮
  - `来源原文将在后续故事中接入` 占位必须删除
  - 空态文案改为温暖文案，遵守 UX 空状态规则
  - `遗忘` 按钮属于 Story 2.8；本 story 不实现 `memory_delete`。如果保留按钮，必须显式 disabled 或不触发任何删除副作用，避免“看起来可用但无行为”

- [x] T2.5 更新 Tab 数量 badge
  - `egosync-app/src/components/role/RoleWorkspacePanel.tsx`：`记忆档案` 标签显示当前角色记忆总数
  - `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx`：`管家记忆` 标签显示全量总览记忆总数
  - 计数通过 `memoryService.count` 获取；组件 unmount/role 切换时避免 stale state 写入

- [x] T2.6 前端测试
  - `MemoryTab.test.tsx`：加载真实记忆、类别筛选、展开来源、缺失来源空态、warm empty copy
  - `memoryService` mock 必须增加 `count` / `getSourceMessages`
  - 如有面板测试或新增测试：验证 Role/Butler tab badge 调用 count 并显示数字
  - 删除旧断言 `来源原文将在后续故事中接入`

### Phase 3: 质量门禁与人工验证（AC: #7）

- [x] T3.1 运行 `cd GUI && npx tsc --noEmit`
- [x] T3.2 运行 `cd GUI && npm run test:frontend`
- [x] T3.3 运行 `cd egosync-app/src-tauri && cargo test`
- [x] T3.4 运行 `cd GUI && npm run build`
- [x] T3.5 启动 `cd GUI && npm run tauri dev` 人工验证 UI
  - 角色记忆页：列表、筛选、badge、展开来源
  - 管家记忆页：全量总览、筛选、badge
  - 来源缺失：显示温和不可用文案
  - 无记忆：显示温暖空态，不出现 `暂无数据`

### Review Findings

_代码评审 (2026-06-01) — 三层对抗式评审（盲审 / 边界 / 验收）。AC-1~AC-7 全部 PASS，无 High。以下为待处理项。_

- [x] [Review][Patch] 来源溯源改为显示可用子集（决策 2026-06-01: 不再「全有或全无」）— `get_source_messages` 过滤掉 system 消息与缺失消息后，展示剩余有效来源，不再因部分消息缺失而整体 `return Ok(Vec::new())`；仅当无任何有效来源时返回空数组（前端「来源对话已不可用」）。需调整后端 len 比较逻辑并补「部分缺失」与「含 system 仍展示有效项」的测试。[egosync-app/src-tauri/src/services/memory_query.rs:67-76]
- [x] [Review][Patch] 记忆 badge 随 category 筛选联动（决策 2026-06-01: badge 跟随当前选中 category）— 将 category 选中状态提升到 `RoleWorkspacePanel` / `ButlerWorkspacePanel`，受控传入 `MemoryTab`；count useEffect 依赖 category 并传入 `count({ ..., category })`，使 badge 与筛选后列表口径一致；badge 在任意 tab 下仍常驻显示。需更新两个 Panel 测试断言。[egosync-app/src/components/role/RoleWorkspacePanel.tsx, egosync-app/src/components/butler/ButlerWorkspacePanel.tsx, egosync-app/src/components/role/MemoryTab.tsx]
- [x] [Review][Patch] 来源加载失败后无法重试 — `toggleSource` 的早返回 guard 检查 `sourceStates[memoryId]?.messages`，而 catch 分支把 `messages` 设为 `[]`（JS 真值），导致一次瞬时 IPC 失败后再次展开仍被 guard 拦截、永不重新请求，用户只能持续看到「来源对话已不可用」。[egosync-app/src/components/role/MemoryTab.tsx:~55,~75]
- [x] [Review][Defer] sourceStates/sourceRequestIds 在 role/category 切换时未清理 [egosync-app/src/components/role/MemoryTab.tsx] — deferred, memory id 全局唯一不串号，仅轻微内存累积，无功能危害
- [x] [Review][Defer] categoryLabels 对未知/历史 category 无兜底标签 [egosync-app/src/components/role/MemoryTab.tsx] — deferred, insert_memories 有 validate 护栏，仅影响潜在历史脏数据
- [x] [Review][Defer] 缺少部分来源缺失 / 前端展开竞态 / Tauri State 注入护栏的测试 [egosync-app/src-tauri/src/services/memory_query.rs, egosync-app/src/components/role/MemoryTab.test.tsx] — deferred, 测试增强项，非阻塞

## Dev Notes

### Story Foundation

- Epic 2 的目标是让用户在多角色间切换对话，系统从对话中提炼记忆，用户能查看、溯源、删除记忆，并追问“为什么”获得透明推理。[Source: `_bmad-output/planning-artifacts/epics.md`:279-302]
- Story 2.7 原始 AC 要求：MemoryTab 显示记忆卡片、点击卡片显示原始对话片段、来源 messages 高亮、类别筛选、Tab 总数 badge、无记忆暖文案，并提供 `memory::list_by_role` / `memory::get_source_messages` 后端能力。[Source: `_bmad-output/planning-artifacts/epics.md`:1081-1115]
- PRD FR-8 要求记忆可查询与溯源：用户追问或查看时能追溯到原始对话记录，原始日志独立存储，日常推理不加载原始日志。[Source: `_bmad-output/planning-artifacts/prd-egosync.md`:201-209]

### Architecture Compliance

- 主数据 `egosync.db` 与对话日志 `conversations.db` 是独立数据边界；`source_conversation_id` 不能做 SQLite 跨库外键，只能应用层校验和 graceful fallback。[Source: `_bmad-output/planning-artifacts/architecture.md`:196-208,941-947]
- 前端不能直接访问 DB；必须通过 service 层封装 Tauri invoke，再由 Rust commands/services/db 查询。[Source: `_bmad-output/planning-artifacts/architecture.md`:371-394,932-939]
- Rust Command 层只做参数解析 → 调 service/db → 返回结果，业务逻辑不要塞在 command 中。[Source: `_bmad-output/project-context.md`:104-114]
- 新增 Tauri Command 时必须同时更新前端 service/types/hook（如需要），并在 `lib.rs` 注册。[Source: `_bmad-output/project-context.md`:183-190]

### Current State of UPDATE Files

| Path | Current state | This story changes | Must preserve |
|---|---|---|---|
| `egosync-app/src/components/role/MemoryTab.tsx` | 已调用 `memoryService.list` / `listAll` 加载真实记忆；有展开按钮，但展开内容仍是“后续故事中接入”占位。[Source: `egosync-app/src/components/role/MemoryTab.tsx`:30-56,81-100] | 增加筛选、懒加载来源消息、真实原文渲染、温暖空态。 | loading/error/展开 aria 基础行为；角色页与管家页的数据边界。 |
| `egosync-app/src/services/memoryService.ts` | 只有 `list` 和 `listAll`。[Source: `egosync-app/src/services/memoryService.ts`:4-7] | 增加 options、count、getSourceMessages。 | 继续使用 Tauri v2 `@tauri-apps/api/core` invoke。 |
| `egosync-app/src/types/memory.ts` | `Memory.sourceMessageIds` 仍是 JSON 字符串。[Source: `egosync-app/src/types/memory.ts`:1-9] | 增加 category/type/source DTO。 | 不让 UI 直接信任前端解析出来的 source ids；来源以后端 command 为准。 |
| `egosync-app/src-tauri/src/commands/memory.rs` | 只有 `memory_list` / `memory_list_all` 两个 command。[Source: `egosync-app/src-tauri/src/commands/memory.rs`:8-19] | 增加筛选参数、count/source commands。 | command 层保持薄。 |
| `egosync-app/src-tauri/src/db/memories.rs` | 已有 insert、list、list_all、dedup；去重基于 role/source/category/source ids。[Source: `egosync-app/src-tauri/src/db/memories.rs`:54-109] | 增加 get_by_id、filter/pagination/count。 | 不破坏 insert 去重和历史重复隐藏。 |
| `egosync-app/src-tauri/src/models/memory.rs` | `Memory` / `ExtractedMemory` 已存在，serde camelCase。[Source: `egosync-app/src-tauri/src/models/memory.rs`:1-19] | 增加 `MemorySourceMessage` DTO。 | 不改变现有 Memory 字段名，避免破坏现有 UI。 |
| `egosync-app/src-tauri/src/db/conversations.rs` | 已有 `get_conversation` 和 `list_messages`，后者按 `created_at ASC, rowid ASC` 排序。[Source: `egosync-app/src-tauri/src/db/conversations.rs`:176-187,268-280] | 复用或增加按 ids 过滤 helper。 | 保持 source 原文顺序；不要创建 conversation。 |
| `egosync-app/src-tauri/src/lib.rs` | 已注册 `memory_list` / `memory_list_all`。[Source: `egosync-app/src-tauri/src/lib.rs`:218-219] | 注册新增 command。 | 不改其他 command 注册语义。 |
| `egosync-app/src/components/role/RoleWorkspacePanel.tsx` | `记忆档案` Tab 静态标签，内容区传 `role.id` 给 MemoryTab。[Source: `egosync-app/src/components/role/RoleWorkspacePanel.tsx`:12-17,27-30] | 添加当前角色记忆 count badge。 | 角色页只传当前 role id。 |
| `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx` | `管家记忆` Tab 静态标签，内容区 `<MemoryTab roleId={null} includeRoleMemories />`。[Source: `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx`:18-20,52] | 添加全量记忆 count badge。 | 保留 `includeRoleMemories` 总览行为。 |

### Previous Story Intelligence

- Story 2.6 已完成 `memories` schema、nullable `role_id`、source fields、去重索引、memory pipeline、`memory_list` / `memory_list_all` 和 MemoryTab 真实列表接入；2.7 不要重建这些能力。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:269-297,350-356]
- `source_conversation_id` 是跨库文本引用；2.6 明确裁决不得为满足外键字面要求合并 DB 或 attach DB。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:213-219]
- 管家全局记忆使用 `role_id = NULL`，角色专属记忆使用具体 role id；2.7 UI 必须继续体现这个归属边界。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:213-218]
- 2.6 已将管家全局事实同步到匹配角色记忆，同时排除 `task_status` 同步；2.7 只负责查询和展示，不应修改提炼/同步策略。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:231-239]
- 2.6 Review Deferred 记录了 source conversation 可能因删除产生悬挂引用；2.7 展开来源必须优雅显示不可用，不把这种历史边界当 panic。[Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:400-405]

### Git Intelligence Summary

- `f550b81 chore(bmad): refresh bundled skills and config`：仅刷新 BMAD/gstack 技能配置，对实现无直接影响。
- `31adb7e docs(chat): align opencode and memory implementation notes`：文档已强调 opencode 与 memory 实现边界，当前 story 以当前代码为准。
- `ef123f4 feat(chat): sync role memories and refine delegation`：近期刚调整角色记忆同步与委派关系；2.7 不应触碰 `memory_pipeline` 的角色归属/委派过滤。
- `34aab28 docs(2.3): refresh delegate bridge implementation notes`：委派桥文档更新，不影响 memory UI。
- `f814983 fix(chat): unlock input after stream completion`：chat completion 状态敏感；2.7 不应改 `llm:stream` payload、`StreamingState` 或 chat 输入锁逻辑。

### Library / Framework Requirements

- 前端使用 React 18.2、TypeScript 5.2 strict、Vite 5、TailwindCSS 3.3.5、Lucide React 0.292；不要新增 UI/状态管理依赖。[Source: `egosync-app/package.json`:14-24]
- Tauri 前端 API 版本为 `@tauri-apps/api` 2.11，invoke 从 `@tauri-apps/api/core` 引入，沿用现有 service 模式。[Source: `egosync-app/package.json`:16,26]
- Rust 使用 Tauri 2、SQLx 0.8、tokio、serde、chrono、uuid；本 story 不需要新 crate。[Source: `egosync-app/src-tauri/Cargo.toml`:18-35]
- 测试框架为 Vitest + React Testing Library 16.3.2；前端测试继续 co-located。[Source: `egosync-app/package.json`:25-39]

### UX Requirements

- Memory 面板属于 `RoleWorkspacePanel` 的右侧结构化工作台，应支持记忆浏览、追溯原文、后续遗忘；本 story 只接浏览和追溯。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:763-770]
- 所有错误/失败反馈通过面板内温和文案或管家自然语言，不使用 toast/snackbar。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:816-827]
- 空态禁止显示“暂无数据”；必须使用有温度文案。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:840-849]
- 导航深度保持管家 ↔ 角色两层，不为来源消息新建页面；在卡片内展开即可。[Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:829-839]

### Implementation Boundaries

- 不实现选择性遗忘：`memory_delete`、确认弹窗、删除后 badge -1 属于 Story 2.8。
- 不改 `memory_pipeline` 提炼 prompt、debounce、global-to-role sync 或去重 migration。
- 不改 `chat_send_message`、`agent_engine`、opencode sidecar、delegate bridge、`llm:stream` payload。
- 不把 `source_message_ids` 的前端字符串解析作为安全边界；来源消息必须由后端按 `memory_id` 查询。
- 不新增外部依赖；Tailwind utility + existing Lucide icon 足够。

### Testing Requirements

- Rust 测试应使用 in-memory SQLite，复用 `002_conversations.sql` / `004_memories.sql` / `005_memory_role_scoped_dedupe.sql`，不要依赖真实 LLM/API Key。
- 前端测试应 mock `memoryService`，覆盖异步展开来源时的 loading、成功、空结果、失败结果。
- Badge 测试可以 mock `memoryService.count`，不要为了显示数量重复 mock完整列表。
- 由于 UI 改动，完成实现后必须人工打开应用验证，不能只报类型检查和测试通过。

## Project Structure Notes

### 新增文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src/hooks/useMemories.ts` | NEW | 封装 MemoryTab 列表查询、category filter、loading/error/refetch。 |
| `egosync-app/src-tauri/src/services/memory_query.rs` | NEW | 推荐新增；跨主库与 conversations DB 的 memory 查询/溯源逻辑。 |

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src/components/role/MemoryTab.tsx` | UPDATE | 筛选、真实来源展开、温暖空态、删除占位处理。 |
| `egosync-app/src/components/role/MemoryTab.test.tsx` | UPDATE | 替换来源占位测试，补筛选/来源/空态。 |
| `egosync-app/src/components/role/RoleWorkspacePanel.tsx` | UPDATE | 角色记忆 count badge。 |
| `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx` | UPDATE | 管家记忆总览 count badge。 |
| `egosync-app/src/services/memoryService.ts` | UPDATE | options、count、getSourceMessages。 |
| `egosync-app/src/types/memory.ts` | UPDATE | category/source DTO/types。 |
| `egosync-app/src-tauri/src/models/memory.rs` | UPDATE | `MemorySourceMessage` DTO。 |
| `egosync-app/src-tauri/src/db/memories.rs` | UPDATE | get_by_id/filter/pagination/count。 |
| `egosync-app/src-tauri/src/db/conversations.rs` | UPDATE | 可选增加按 message ids 返回来源片段 helper；也可由 service 复用 `list_messages` 过滤。 |
| `egosync-app/src-tauri/src/commands/memory.rs` | UPDATE | 增加筛选参数、count/source commands。 |
| `egosync-app/src-tauri/src/services/mod.rs` | UPDATE | 若新增 `memory_query.rs`，导出模块。 |
| `egosync-app/src-tauri/src/lib.rs` | UPDATE | 注册新增 commands。 |

### 不应修改

- `egosync-app/src-tauri/migrations/004_memories.sql`
- `egosync-app/src-tauri/migrations/005_memory_role_scoped_dedupe.sql`
- `egosync-app/src-tauri/src/services/memory_pipeline.rs`
- `egosync-app/src-tauri/src/commands/chat.rs`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/src-tauri/src/services/agent_bridge.rs`
- `egosync-app/src-tauri/src/services/agent_config.rs`

## References

- [Source: `_bmad-output/planning-artifacts/epics.md`:1081-1115 — Story 2.7 AC and backend/frontend expectations]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md`:201-209 — FR-8 memory traceability]
- [Source: `_bmad-output/planning-artifacts/architecture.md`:196-208,941-947 — separate DBs and data boundaries]
- [Source: `_bmad-output/planning-artifacts/architecture.md`:371-394,932-939 — IPC/service boundaries]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:763-770 — Memory tab in RoleWorkspacePanel]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md`:816-849 — feedback and empty-state UX]
- [Source: `_bmad-output/project-context.md`:104-114,118-136,183-205 — Tauri layering, tests, command checklist, anti-patterns]
- [Source: `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`:213-245,350-356,400-405 — previous story memory implementation and known boundaries]
- [Source: `egosync-app/src/components/role/MemoryTab.tsx`:30-56,81-100 — current real list and source placeholder]
- [Source: `egosync-app/src/services/memoryService.ts`:4-7 — current memory service methods]
- [Source: `egosync-app/src/types/memory.ts`:1-9 — current Memory type]
- [Source: `egosync-app/src-tauri/src/commands/memory.rs`:8-19 — current memory commands]
- [Source: `egosync-app/src-tauri/src/db/memories.rs`:54-109 — list/dedup behavior]
- [Source: `egosync-app/src-tauri/src/db/conversations.rs`:176-187,268-280 — existing conversation/message query helpers]
- [Source: `egosync-app/src-tauri/src/lib.rs`:218-219 — current memory command registration]
- [Source: `egosync-app/package.json`:14-39 — frontend deps and test scripts]
- [Source: `egosync-app/src-tauri/Cargo.toml`:18-35 — Rust deps]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context)

### Debug Log References

- 2026-05-31: `python3` 不可用，按 skill 规则手动读取 `customize.toml` 并确认无 team/user override、无 activation prepend/append。
- 2026-05-31: 首次 Tauri dev 启动失败，根因为 Vite 5173 被既有 node 进程占用；经用户确认后结束旧进程并重启。
- 2026-05-31: 默认 Cargo target 编译失败，根因为旧 `egosync.exe` 锁定 `target/debug/egosync.exe`；经用户确认后结束旧进程。
- 2026-05-31: `src-tauri/target-claude-dev-story27` 编译失败，根因为 `sqlx-sqlite` 编译产物被占用；改用 `egosync-app/.tmp/target-claude-dev-story27` 避免锁定与 watcher 递归重建。
- 2026-05-31: gstack browse 二进制/安装目录缺失，无法使用 browse 工具；改用浏览器级验证和组件交互测试补充 UI 验证。
- 2026-05-31: 普通浏览器环境没有 Tauri bridge，真实 `invoke` 调用不可用；Tauri dev 日志显示 WebView2 已启动并访问 Vite，浏览器级验证使用临时 stub 验证 UI 形态，来源展开使用组件交互测试验证。

### Completion Notes List

- 后端新增 `MemorySourceMessage` 公开 DTO，来源消息只返回 `id`、`conversationId`、`role`、`content`、`createdAt`、`isSource`，避免暴露 `thinkingContent`、`routingMetadata` 或 system 脚手架消息。
- `memory_list` / `memory_list_all` 支持 category、limit、offset；记忆计数复用去重后的可见列表长度，保证 badge 与列表口径一致。
- 新增 `memory_query` service 承担跨主库和 conversations DB 的来源解析，command 层保持参数接收与 service 调用薄层。
- 角色记忆面板使用当前 `roleId` 查询；管家记忆面板继续使用 `includeRoleMemories` 总览全局与角色记忆。
- MemoryTab 支持类别筛选、真实来源懒加载、来源不可用文案、温暖空态、ARIA 展开状态和禁用的 `遗忘` 占位按钮。
- 为避免触碰不应修改边界，保留 `db::memories::list_all_memories(pool)` 旧签名供既有 `agent_engine` 调用，新筛选分页能力通过 `list_all_memories_with_options` 提供。
- 自动化验证全部通过：TypeScript 检查、前端测试、Rust 全量测试、前端生产构建。
- UI 验证已启动 Tauri dev，日志显示 `egosync.exe` 启动并初始化真实 `egosync.db` / `conversations.db`，WebView2 与 Vite 5173 建立连接；浏览器级验证确认 badge、筛选、空态/错误态不出现 `暂无数据`，来源展开由组件交互测试确认。

### File List

- `egosync-app/src-tauri/src/models/memory.rs`
- `egosync-app/src-tauri/src/db/memories.rs`
- `egosync-app/src-tauri/src/services/memory_query.rs`
- `egosync-app/src-tauri/src/services/mod.rs`
- `egosync-app/src-tauri/src/commands/memory.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/types/memory.ts`
- `egosync-app/src/services/memoryService.ts`
- `egosync-app/src/hooks/useMemories.ts`
- `egosync-app/src/components/role/MemoryTab.tsx`
- `egosync-app/src/components/role/MemoryTab.test.tsx`
- `egosync-app/src/components/role/RoleWorkspacePanel.tsx`
- `egosync-app/src/components/role/RoleWorkspacePanel.test.tsx`
- `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx`
- `egosync-app/src/components/butler/ButlerWorkspacePanel.test.tsx`
- `_bmad-output/implementation-artifacts/2-7-memory-panel-traceability.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-05-31: Ultimate context engine analysis completed - comprehensive developer guide created. Status set to ready-for-dev.
- 2026-05-31: Implemented memory filtering/count/source traceability API, MemoryTab source expansion and category filters, role/butler memory badge counts, and Story 2.7 tests.
- 2026-05-31: Validation completed (`tsc --noEmit`, `npm run test:frontend`, `cargo test`, `npm run build`) and Tauri dev/UI verification performed with documented browser/Tauri bridge limitations. Status set to review.
- 2026-06-01: 代码评审（三层对抗式：盲审/边界/验收，AC-1~AC-7 全部 PASS，无 High）。处理评审发现 3 项 patch / 3 项 defer / 5 项 dismiss。修复内容：(1) 溯源 `get_source_messages` 改为返回可用子集（不再因部分来源缺失或含 system 而整体置空）；(2) 记忆 badge 随 category 筛选联动（category 状态提升至 Panel，MemoryTab 改为受控/非受控双模式，count 依赖 category）；(3) 来源加载失败后 `messages` 保持 null 以允许重试。验证：tsc 通过、前端 3 文件 10 测试通过、Rust memory_query 6 测试通过。
