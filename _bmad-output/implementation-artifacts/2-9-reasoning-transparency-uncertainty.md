---
created_at: 2026-06-01T23:04:42+08:00
baseline_commit: 55582e24f21e8d3432fca61bb837c1a333bce95b
---

# Story 2.9: 用户追问“为什么”获得透明推理链，AI 不确定时主动声明

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want AI 做出建议时能解释推理依据，不确定时主动告诉我,
so that 我能判断建议是否可信。

## Acceptance Criteria

1. **AC-1 基于记忆的建议可被“为什么”追问到具体记忆依据**
   - **Given** 管家或角色基于结构化记忆做出建议
   - **When** 用户追问 `为什么`、`你怎么知道的`、`依据是什么` 或等价表达
   - **Then** 回复必须包含引用的记忆条目列表，格式为 `[记忆#<memory_id>] 内容摘要`
   - **And** `<memory_id>` 必须是真实存在于当前 `memories.id` 的 ID，不能编造、不能使用列表序号
   - **And** 内容摘要必须来自对应 memory 的 `content`，可截断但不能改变含义
   - **And** 若没有可引用记忆，应明确说明“我现在没有可溯源的记忆依据”，不得伪造来源

2. **AC-2 复合推理输出可读的证据链**
   - **Given** 一个建议同时依赖多条当前可见记忆
   - **When** 用户追问原因或模型主动解释建议
   - **Then** 回复输出简洁证据链，例如：`[记忆#12] 你之前说过周五有家庭聚餐 → [记忆#8] 产品评审也在周五 → 建议调整`
   - **And** 证据链只暴露“依据链/证据链”：记忆、规则、历史模式；不得暴露隐藏 chain-of-thought
   - **And** 多条记忆的顺序应服务于解释建议，不要求与创建时间一致

3. **AC-3 不确定性表达不被隐藏且不过度泛化**
   - **Given** AI 对建议的信心不足
   - **When** 最终用户可见回复包含 hedging 语言，例如 `可能`、`也许`、`不太确定`、`大概`、`看起来像`
   - **Then** 回复中必须主动声明不确定性，例如：`我不太确定这个判断，建议你自己评估一下`
   - **And** 不得把低置信度判断写成确定事实
   - **And** 不确定性提示只在存在真实不确定性或能力边界时出现，不得每条回复机械追加限定词

4. **AC-4 agent_engine 注入记忆时带真实 ID，并给管家与角色同等 prompt 规则**
   - **Given** `agent_engine` 为管家或角色构建 system prompt / opencode prompt 前缀
   - **Then** 注入的结构化记忆必须带真实 ID 标记，格式至少包含 `[记忆#<id>]`、类别、内容摘要，角色记忆还需保留 owner/role 语义
   - **And** 管家路径和角色路径都必须能看到对应范围内可引用的记忆 ID
   - **And** System Prompt 必须包含规则：引用记忆时标注来源；用户追问原因时输出依据链；不确定时主动声明；不得编造记忆 ID
   - **And** opencode 主路径与 LlmProvider fallback 路径都使用同一套 prompt/context 组装逻辑

5. **AC-5 遗忘后的记忆不再进入新推理依据**
   - **Given** Story 2.8 已删除某条 `memories` 可见记录并写入 `forgotten_memory_sources`
   - **When** 后续管家或角色构建 prompt、回答建议、解释“为什么”
   - **Then** 不得注入、引用或从 tombstone 恢复已遗忘记忆内容
   - **And** 历史对话中既有 `[记忆#ID]` 文本不被篡改
   - **And** 如果用户点击历史消息里的旧引用但目标 memory 已不存在，UI 给出温和不可用反馈或保持列表刷新后的空结果，不伪造来源

6. **AC-6 ChatBubble 将 `[记忆#ID]` 渲染为可点击链接**
   - **Given** assistant 消息正文包含 `[记忆#<id>]`
   - **When** 消息在 `ChatBubble` 中渲染
   - **Then** `[记忆#<id>]` 显示为可点击元素，具备 button/link 语义和键盘可达焦点
   - **And** 点击不会触发页面跳转或破坏 Markdown 渲染
   - **And** 用户消息中的普通文本不应被错误处理成系统跳转动作；只对 assistant 可见回答中的记忆引用启用跳转

7. **AC-7 点击记忆引用打开 MemoryTab 并定位目标卡片**
   - **Given** 用户在管家或角色对话气泡中点击 `[记忆#<id>]`
   - **When** 当前视图存在右侧工作台
   - **Then** 打开对应工作台的 MemoryTab
   - **And** MemoryTab 清空或调整当前 category filter，使目标 memory 可见
   - **And** 滚动到目标 memory 卡片并短暂高亮
   - **And** 管家视图使用全局 + 角色总览语义；角色视图优先定位当前角色范围内的 memory
   - **And** 若目标 memory 不属于当前角色或已不存在，应给出温和不可用反馈，不切到错误角色、不展示 tombstone

8. **AC-8 验证通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd GUI/src-tauri && cargo test`
   - `cd GUI && npm run build`
   - 因本 story 修改聊天 UI 与工作台跳转，必须启动 `cd GUI && npm run tauri dev` 做人工验证：管家记忆引用点击、角色记忆引用点击、category filter 下定位、已删除记忆引用降级、streaming 后完成态仍可点击、不确定性声明展示。

## Tasks / Subtasks

### Phase 1: 后端记忆引用上下文与 prompt 规则（AC: #1, #2, #3, #4, #5, #8）

- [x] T1.1 更新 `GUI/src-tauri/src/services/agent_engine.rs` 的管家记忆摘要格式
  - 修改 `build_butler_memory_summary`：当前格式是 `- [category] content`；必须改为包含真实 `[记忆#<memory.id>]`。
  - 建议格式：`- [记忆#<id>] [<category>] <content 摘要>`。
  - 继续使用 `memories::list_all_memories(main_pool)`，保持与 MemoryTab 总览一致的“当前可见 memories”来源，不读 `forgotten_memory_sources`。
  - 保留现有截断常量语义：`BUTLER_MEMORY_PER_ROLE`、`BUTLER_MEMORY_PER_LINE_CHARS`、`BUTLER_MEMORY_TOTAL_CHARS`。
  - 保留 role 分组：global 记忆在 `全局记忆`，role 记忆按角色名分组。

- [x] T1.2 为角色路径新增角色记忆摘要注入
  - 当前 `build_role_system_prompt(role)` 不接收 DB，只输出 `[context_injection] 以下历史消息是当前对话上下文；不要引入未提供的记忆。`
  - 新增 async helper，例如 `build_role_memory_summary(main_pool, role_id) -> Result<String, AppError>`，只查询该 `role_id` 当前可见记忆。
  - 将角色 prompt 组装改到可读取 `main_pool` 的路径：`build_role_messages` 与 opencode content 前缀都必须注入同一角色记忆摘要。
  - 不要把管家全局总览全部塞进角色私聊；角色路径只注入当前角色相关记忆，除非实现明确需要全局记忆且能避免跨角色污染。

- [x] T1.3 提取共享透明度规则 prompt
  - 在 `agent_engine.rs` 增加一个短规则块，例如 `[透明推理与不确定性规则]`。
  - 规则必须覆盖：
    - 基于记忆回答或建议时，使用 `[记忆#<真实id>]` 标注来源。
    - 用户问 `为什么` / `你怎么知道的` / `依据是什么` 时，输出简洁依据链。
    - 不确定时主动声明，不把推断写成确定事实。
    - 只能引用 prompt 中出现的记忆 ID；没有依据时说明没有可溯源记忆依据。
    - 不暴露隐藏 chain-of-thought，只给证据链。
  - 管家和角色都注入该规则；onboarding 不需要接入本 story 规则。

- [x] T1.4 确保 opencode 主路径与 fallback 路径一致
  - `try_run_opencode_stream` 当前会在 role path 调 `build_role_system_prompt(&role)`，butler path 调 `build_butler_system_prompt(...)`。
  - fallback `run_stream` 当前走 `build_role_messages` / `build_butler_messages`。
  - 重构后两条路径必须复用同一 prompt builder，避免只有 opencode 或只有 fallback 生效。
  - 不修改 `agent_bridge.rs` 业务语义；bridge 仍只负责 HTTP/SSE 封装和解析。

- [x] T1.5 增加 Rust 单元测试
  - `agent_engine`：管家 memory summary 包含真实 `[记忆#id]` 与 category/content，不再只有 `[category] content`。
  - `agent_engine`：角色 system prompt/messages 包含当前 role 的 `[记忆#id]`，不包含其它 role 的记忆。
  - `agent_engine`：透明度规则同时出现在 butler 与 role prompt，且包含“不编造记忆 ID / 不确定时声明 / 不暴露 chain-of-thought”语义。
  - `db::memories` 或 prompt helper 测试：删除后的 memory 不会从 `list_*` 注入；不读取 tombstone 内容。

### Phase 2: 前端记忆引用渲染与跳转数据流（AC: #6, #7, #8）

- [x] T2.1 扩展 `GUI/src/components/chat/ChatBubble.tsx`
  - 新增可选 props，例如 `onMemoryReferenceClick?: (memoryId: string) => void`。
  - assistant 消息正文中的 `[记忆#<id>]` 渲染为可点击元素；用户消息保持纯文本。
  - 继续使用 `ReactMarkdown` 渲染普通 Markdown，不破坏现有 prose 样式。
  - 不引入新 markdown 插件依赖；可通过预处理文本或自定义渲染组件实现。
  - 可点击元素使用 Tailwind utility，具备 `type="button"`、focus ring、清晰 hover 状态。

- [x] T2.2 扩展 `GUI/src/components/chat/ChatStream.tsx`
  - 新增可选 prop：`onMemoryReferenceClick?: (memoryId: string) => void`。
  - 将 callback 传给历史消息和 streaming 完成后的 `ChatBubble`。
  - 保持现有 streaming 状态机：`payload.thinking` 不进入正文；`messageId` bucket 不破坏委派两气泡；final done 解锁输入；segment done 不提前结束。
  - 不新增 stream metadata event；本 story 通过消息正文中的 `[记忆#ID]` 触发跳转即可。

- [x] T2.3 扩展 `GUI/src/components/role/MemoryTab.tsx` 定位能力
  - 新增可选 props，例如 `targetMemoryId?: string | null`、`onTargetMemoryHandled?: () => void`。
  - 为每个 memory 卡片增加稳定 DOM anchor，例如 `id={`memory-${memory.id}`}` 或 ref map。
  - 当 `targetMemoryId` 出现在当前可见列表时，滚动到该卡片并短暂高亮。
  - 如果当前 category filter 会隐藏目标，父级应先清空 filter；MemoryTab 不要在内部猜测跨父级状态。
  - 如果目标不存在或加载失败，显示温和不可用反馈，例如 `这条记忆现在不可用，可能已经被遗忘了`。
  - 保留现有 source 展开、遗忘确认、删除失败、request id 防竞态逻辑。

- [x] T2.4 扩展 `GUI/src/components/role/RoleWorkspacePanel.tsx`
  - 接收 `targetMemoryId` / `onTargetMemoryHandled` 或等价 props。
  - 当收到记忆引用跳转时，设置 `currentTab` 为 `memory` 并清空 `memoryCategory`，确保目标不被 category filter 隐藏。
  - 将目标 ID 传给 `MemoryTab`。
  - 保持 badge count 仍按当前 category 与 role.id 重新计算。

- [x] T2.5 扩展 `GUI/src/components/butler/ButlerWorkspacePanel.tsx`
  - 接收 `targetMemoryId` / `onTargetMemoryHandled` 或等价 props。
  - 打开 `memory` tab，清空 `memoryCategory`，并传递目标 ID 给全局总览 `MemoryTab`。
  - 保持 `includeRoleMemories` 与 `showOwnerLabel` 语义，不误改成只看 `role_id = NULL`。

- [x] T2.6 扩展 `GUI/src/components/role/RoleView.tsx` 与 `GUI/src/components/butler/ButlerView.tsx`
  - 在 view 层持有 `targetMemoryId` 状态，因为 `ChatStream` 与 WorkspacePanel 是兄弟组件。
  - 点击 ChatBubble 中 `[记忆#ID]` 时：设置 target ID，并打开 `openTab='memory'`。
  - `MemoryTab` 成功处理后清理 target，避免后续重复滚动。
  - 不在 `App.tsx` 新增组件定义；只在现有 view 组件中传递状态。

- [x] T2.7 前端测试
  - `ChatBubble.test.tsx`：assistant 内容 `[记忆#abc]` 渲染为可点击元素，点击调用 callback；用户消息不触发 callback。
  - `ChatBubble.test.tsx`：普通 Markdown 仍可渲染，thinking 展开行为不回归。
  - `ChatStream.test.tsx`：传入 `onMemoryReferenceClick` 后，历史消息与 streaming 完成消息中的引用都可触发 callback；thinking token 不被误识别为正文引用。
  - `MemoryTab.test.tsx`：传入 targetMemoryId 时滚动/高亮目标；目标不存在时显示温和不可用反馈。
  - `RoleWorkspacePanel.test.tsx`：记忆引用跳转后清空 category 并打开 memory tab，badge count 语义不变。
  - `ButlerWorkspacePanel.test.tsx`：管家总览跳转保留 `includeRoleMemories=true`。

### Phase 3: 不确定性兜底与验收验证（AC: #3, #8）

- [x] T3.1 后端不确定性 prompt 验证
  - 单元测试或 prompt snapshot 断言 transparent prompt 包含 hedging 触发后的主动声明规则。
  - 不要求新增 DB schema 或 stream payload metadata；除非实现中发现 prompt-only 无法稳定满足 AC。

- [x] T3.2 可选后处理兜底（仅在测试证明必要时）
  - 若 prompt-only 无法稳定让模型在 hedging 回复中声明不确定性，可在 `agent_engine.rs` 对最终 assistant 文本增加最小后处理 helper。
  - helper 只能在文本包含 hedging 词且没有现成不确定性声明时追加一句短声明。
  - 不要每条回复都追加；不要重复追加；不要改写用户消息、thinking_content 或 routing_metadata。
  - 如果实现该 helper，opencode 主路径与 fallback 路径都要走同一 helper，并补 Rust 单测。

- [x] T3.3 运行验证命令
  - `cd GUI && npx tsc --noEmit`
  - `cd GUI && npm run test:frontend`
  - `cd GUI/src-tauri && cargo test`
  - `cd GUI && npm run build`

- [x] T3.4 启动 Tauri dev 并做人工验证
  - `cd GUI && npm run tauri dev`
  - 管家对话：构造/使用已有全局或角色记忆，追问“为什么”，确认回复含 `[记忆#ID]`。
  - 角色对话：角色基于自身记忆回答，追问“你怎么知道的”，确认回复含角色 memory ID。
  - 点击管家气泡 memory link：打开管家 MemoryTab，滚动/高亮目标。
  - 点击角色气泡 memory link：打开角色 MemoryTab，滚动/高亮目标。
  - category filter 已选择时点击引用：filter 被清空或调整，目标仍可见。
  - 删除某记忆后点击历史引用：温和不可用，不展示 tombstone。
  - 诱导模型输出 `可能/也许/不太确定`：最终回复保留不确定性声明。

## Dev Notes

### Epic / PRD / UX Context

- Epic 2 目标是把 EgoSync 从基础对话升级为多角色个性化 Agent 系统，覆盖角色语调、记忆提炼、记忆查询、选择性遗忘、推理溯源与不确定性表达。[Source: _bmad-output/planning-artifacts/epics.md:60-77]
- Story 2.9 原始要求：用户追问 `为什么` / `你怎么知道的` 后，回复包含 `[记忆#ID] 内容摘要`；复合推理输出 `记忆 A → 记忆 B → 建议`；hedging 语言触发主动不确定性声明；`agent_engine` 注入记忆 ID；前端点击引用打开 MemoryTab 并滚动。[Source: _bmad-output/planning-artifacts/epics.md:1152-1180]
- PRD FR-29 要求推理链包含“引用的记忆条目、使用的规则、参考的历史模式”，且不是泛泛解释。[Source: _bmad-output/planning-artifacts/prd-egosync.md:455-461]
- PRD FR-30 要求信心不足时主动表达不确定性，且频率合理，不能每条都机械加限定词。[Source: _bmad-output/planning-artifacts/prd-egosync.md:463-469]
- UX 原则强调信任来自“可预期、可追问、可修正”；角色视图左侧 60% 聊天流，右侧 40% 工作台，MemoryTab 是记忆、来源和遗忘的落点。[Source: _bmad-output/planning-artifacts/ux-design-specification.md:537-540] [Source: _bmad-output/planning-artifacts/ux-design-specification.md:763-770]

### Architecture Compliance

- 严守分层：前端不直接访问 DB/LLM/opencode；commands 只做参数校验与 service 调用；services 承载业务逻辑；`agent_bridge.rs` 只做 HTTP/SSE 封装；db 只做 SQL。[Source: _bmad-output/planning-artifacts/architecture.md:932-939]
- 对话核心回路是 `ChatInput → chat_send_message → agent_engine → agent_bridge/opencode → app.emit("llm:stream") → ChatStream`，完成后由 `memory_pipeline.rs` 提炼记忆。[Source: _bmad-output/planning-artifacts/architecture.md:975-988]
- FR-29/FR-30 架构映射已经指向 `agent_engine` 上下文组装可溯源；不要新建平行推理系统。[Source: _bmad-output/planning-artifacts/architecture.md:1058-1061]
- TypeScript strict/noUnusedLocals/noUnusedParameters；Rust command 返回 `Result<T, AppError>`；serde DTO 使用 camelCase；UI 样式使用 Tailwind utility。[Source: _bmad-output/project-context.md:60-80] [Source: _bmad-output/project-context.md:84-114] [Source: _bmad-output/project-context.md:192-223]

### Existing Code State: Must Reuse / Update

#### `GUI/src-tauri/src/services/agent_engine.rs`

- `build_butler_memory_summary` 当前用 `memories::list_all_memories(main_pool)`，输出 `- [category] content`，没有 memory ID；这是 AC-4 的主要后端缺口。[Source: GUI/src-tauri/src/services/agent_engine.rs:183-234]
- `build_butler_system_prompt` 已按“基线 → 角色清单 → 已知记忆 → 各角色近况 → 行为指南 → 角色涌现行为”拼接；透明推理规则应作为短规则块追加到管家/角色 prompt，不破坏现有顺序语义。[Source: GUI/src-tauri/src/services/agent_engine.rs:294-381]
- `build_role_system_prompt` 当前只接收 `Role`，不会读取 DB；`build_role_messages` 只注入角色定义、历史消息、当前用户消息，角色结构化记忆未注入。[Source: GUI/src-tauri/src/services/agent_engine.rs:431-510]
- opencode path 在 `try_run_opencode_stream` 中自己拼 `[系统指示]` 前缀，role path 调 `build_role_system_prompt(&role)`，butler path 调 `build_butler_system_prompt(...)`；fallback path 调 `build_role_messages` / `build_butler_messages`。[Source: GUI/src-tauri/src/services/agent_engine.rs:841-857] [Source: GUI/src-tauri/src/services/agent_engine.rs:1413-1424]
- `llm:stream` payload、thinking token、messageId bucket、委派两气泡状态机都已存在；本 story 不需要新增 stream event 类型。[Source: GUI/src-tauri/src/services/agent_engine.rs:105-122] [Source: GUI/src-tauri/src/services/agent_engine.rs:870-1176]

#### `GUI/src/components/chat/ChatBubble.tsx`

- Assistant 内容当前直接 `<ReactMarkdown>{message.content}</ReactMarkdown>`，没有 memory reference 特殊渲染。[Source: GUI/src/components/chat/ChatBubble.tsx:106-115]
- 已有 thinking UI：streaming 时“思考中...”，完成后“思考过程”折叠；不要破坏空 content + thinking 的显示能力。[Source: GUI/src/components/chat/ChatBubble.tsx:42-84]
- 角色身份显示通过 `assistantName` / `assistantIcon` / `assistantColor`，管家默认 `Home + 管家`；memory link UI 不应覆盖身份逻辑。[Source: GUI/src/components/chat/ChatBubble.tsx:17-28] [Source: GUI/src/components/chat/ChatBubble.tsx:45-104]

#### `GUI/src/components/chat/ChatStream.tsx`

- `ChatStream` 目前 props 只有 `role?: Role | null`；需要增加 memory click callback 并传给 `ChatBubble`。[Source: GUI/src/components/chat/ChatStream.tsx:11-16]
- `handleStreamEvent` 把 `payload.thinking` 追加到 `thinkingContent`，其它非 done payload 追加到 stream bubble content；不要把 metadata 事件加入本 story，否则会被拼进正文。[Source: GUI/src/components/chat/ChatStream.tsx:257-347]
- streaming 完成时通过 `completedAssistantMessagesFromBubbles` 生成本地完成消息，并保留首个 bubble 的 `thinkingContent`。[Source: GUI/src/components/chat/ChatStream.tsx:18-53] [Source: GUI/src/components/chat/ChatStream.tsx:276-289]
- 必须保留 conversationId 过滤、messageId bucket、final done 解锁、delegation segment done 不提前结束、history refresh merge 逻辑。[Source: GUI/src/components/chat/ChatStream.tsx:257-350]

#### `GUI/src/components/role/MemoryTab.tsx`

- Props 当前没有 target/highlight/scroll 能力；新增能力应在这里承接。[Source: GUI/src/components/role/MemoryTab.tsx:8-18]
- 现有 category filters 只显示全部、事实、偏好、认知模式；`visibleMemories` 过滤 `task_status`。[Source: GUI/src/components/role/MemoryTab.tsx:20-32] [Source: GUI/src/components/role/MemoryTab.tsx:187-190]
- 卡片当前没有稳定 memory anchor；只有来源展开区有 `id="memory-source-${memory.id}"`。[Source: GUI/src/components/role/MemoryTab.tsx:222-332]
- 保留现有 source 展开、request id 防竞态、遗忘确认、删除中禁用、失败内联反馈。[Source: GUI/src/components/role/MemoryTab.tsx:74-185]

#### `GUI/src/components/role/RoleView.tsx` / `GUI/src/components/butler/ButlerView.tsx`

- `ChatStream` 与 WorkspacePanel 是兄弟组件；memory link 点击需要在 View 层持有 target state 并打开 `openTab='memory'`。[Source: GUI/src/components/role/RoleView.tsx:29-72] [Source: GUI/src/components/butler/ButlerView.tsx:7-51]
- `RoleWorkspacePanel` 与 `ButlerWorkspacePanel` 已持有 `memoryCategory` 并将 `MemoryTab` 接到受控 category；跳转时应清空 category 让目标可见。[Source: GUI/src/components/role/RoleWorkspacePanel.tsx:10-63] [Source: GUI/src/components/butler/ButlerWorkspacePanel.tsx:10-90]

#### Memory data layer

- `Memory.id` 是 UUID string，并通过 serde camelCase 暴露给前端；`MemorySourceMessage` 只暴露公开 trace 字段，不包含 `thinkingContent` 或 `routingMetadata`。[Source: GUI/src-tauri/src/models/memory.rs:1-30] [Source: GUI/src-tauri/src/models/memory.rs:36-54]
- `memoryService` 已有 `list`、`listAll`、`count`、`getSourceMessages`、`delete`，前端新增跳转不应绕过该 service 层直接 invoke/DB。[Source: GUI/src/services/memoryService.ts:10-30]
- `db::memories::delete_memory` 会先写 `forgotten_memory_sources` tombstone，再删除 `memories` 记录；后续注入只应来自 `list_*` 当前 records，不读 tombstone。[Source: GUI/src-tauri/src/db/memories.rs:261-304]
- `list_memories` / `list_all_memories_with_options` 默认排除 `task_status` 并做 visible dedupe；prompt 注入最好使用同一 visible 口径，避免引用用户在 MemoryTab 看不到的 ID。[Source: GUI/src-tauri/src/db/memories.rs:306-390] [Source: GUI/src-tauri/src/db/memories.rs:393-407]

### Previous Story Intelligence

- Story 2.8 完成了选择性遗忘：删除 `memories` 可见记录，记录轻量同源屏蔽，不重跑推理、不改 history/opencode。[Source: _bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:348-350]
- 2.8 已实现 MemoryTab 自定义确认、删除中禁用、失败内联反馈、成功后清理来源状态并刷新 list/count。[Source: _bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:351-354]
- 2.8 新增 `GUI/src-tauri/migrations/007_forgotten_memory_sources.sql`，并在 `insert_memories` / `update_memory_from_extracted` 中跳过已遗忘同源候选。[Source: _bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:357-358]
- 2.8 修改/触及了本 story 相关文件：`agent_engine.rs`、`agent_bridge.rs`、`models/agent.rs`、`ChatStream.tsx`、`ChatStream.test.tsx`、`ChatBubble.test.tsx`、`MemoryTab.tsx`、`RoleWorkspacePanel.tsx`、`ButlerWorkspacePanel.tsx`。[Source: _bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:362-386]
- 2.8 已通过 tsc、frontend tests、build、Rust memory tests、完整 Rust suite、Tauri dev 验证；延续同样验证门禁。[Source: _bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:354-360]

### Git Intelligence

- 当前 baseline commit：`55582e24f21e8d3432fca61bb837c1a333bce95b`。
- 最近提交 `fix(memory): stabilize selective forgetting` 表明 2.8 后还有稳定选择性遗忘修复；2.9 实现必须基于当前工作区实际代码，而不是只信 2.8 story 初稿。
- 近期提交集中在 memory ownership/display、opencode/delegation、chat/memory 同步；不要引入平行系统，优先复用当前 `MemoryTab`、`ChatStream`、`agent_engine`。

### Implementation Boundaries

- 不新增外部依赖；不需要 Web 研究或库升级。
- 不新增 DB schema，除非实现中证明 prompt-only 不确定性无法满足 AC；默认通过 prompt + 可选最终文本 helper 解决。
- 不修改 opencode config 手工文件；角色 agent 配置仍由现有 service 管理。
- 不把 `routing_metadata` 改成通用 reasoning metadata；它当前语义是管家委派审计。
- 不把 `thinking_content` 暴露成用户可点击依据；可见依据链来自 memory IDs / rules / history patterns。
- 不让前端直接读取 DB、opencode 或 LLM；跳转只在 React state 与现有 memory service 能力内完成。
- 历史消息中的旧 `[记忆#ID]` 文本不篡改；目标不存在时优雅降级。

## Project Structure Notes

- 后端业务逻辑集中在 `GUI/src-tauri/src/services/agent_engine.rs`；commands/db/bridge 分层不变。
- 前端聊天渲染改 `GUI/src/components/chat/ChatBubble.tsx` 与 `ChatStream.tsx`；兄弟组件联动改 `ButlerView.tsx` / `RoleView.tsx`。
- Memory 定位能力改现有 `GUI/src/components/role/MemoryTab.tsx`，不要新增 parallel memory panel。
- 工作台接线改现有 `RoleWorkspacePanel.tsx` / `ButlerWorkspacePanel.tsx`，保持 badge/category count 逻辑。
- 测试 co-located：`ChatBubble.test.tsx`、`ChatStream.test.tsx`、`MemoryTab.test.tsx`、`RoleWorkspacePanel.test.tsx`、`ButlerWorkspacePanel.test.tsx`；Rust 单测放在对应 `.rs` 文件底部。

### References

- [_bmad-output/planning-artifacts/epics.md:1152-1180](../planning-artifacts/epics.md) — Story 2.9 原始 AC。
- [_bmad-output/planning-artifacts/prd-egosync.md:455-469](../planning-artifacts/prd-egosync.md) — FR-29 / FR-30。
- [_bmad-output/planning-artifacts/ux-design-specification.md:537-540](../planning-artifacts/ux-design-specification.md) — 角色视图 60/40 布局。
- [_bmad-output/planning-artifacts/ux-design-specification.md:763-770](../planning-artifacts/ux-design-specification.md) — RoleWorkspacePanel MemoryTab 用途。
- [_bmad-output/planning-artifacts/architecture.md:932-939](../planning-artifacts/architecture.md) — 分层规则。
- [_bmad-output/planning-artifacts/architecture.md:975-988](../planning-artifacts/architecture.md) — 对话核心回路。
- [_bmad-output/project-context.md:192-223](../project-context.md) — 禁止事项与数据边界。
- [_bmad-output/implementation-artifacts/2-8-selective-memory-forget.md:348-360](./2-8-selective-memory-forget.md) — 前序 story 完成经验。

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M context)

### Debug Log References

- `npx tsc --noEmit` — passed.
- `npm run test:frontend` — 13 files / 80 tests passed.
- `cargo test --manifest-path GUI/src-tauri/Cargo.toml --offline -- --nocapture` — 228 tests passed.
- `npm run build` — passed.
- `npm run tauri dev` — Vite ready at localhost:5173 and Tauri dev build reached ready marker for manual verification.
- Review fix targeted validation: `npx tsc --noEmit` — passed; targeted frontend tests — 4 files / 46 tests passed; `test_sse_thinking_is_not_mapped_to_user_visible_payload` — passed.
- Review fix full validation: `npx tsc --noEmit` — passed; `npm run test:frontend` — 13 files / 84 tests passed; `cargo test --manifest-path GUI/src-tauri/Cargo.toml --offline -- --nocapture` — 227 lib tests + 1 integration test passed; `npm run build` — passed.
- Review fix dev restart: stopped the stale current-project Vite process on port 5173, restarted `npm run tauri dev`, and reached the ready marker.

- Second review follow-up targeted validation: MemoryTab/Workspace target regression tests — 3 files / 22 tests passed; bus delta hidden-thinking tests — 3 tests passed; `npx tsc --noEmit` — passed.
- Second review follow-up full validation: `npm run test:frontend` — 13 files / 85 tests passed; `cargo test --manifest-path GUI/src-tauri/Cargo.toml --offline -- --nocapture` — 230 lib tests + 1 integration test passed; `npm run build` — passed.
- Second review follow-up dev restart: stopped the stale current-project Vite listener, restarted `npm run tauri dev`, and reached the ready marker.
- Source navigation and memory-link follow-up validation: targeted frontend tests for `ChatBubble`, `MemoryTab`, and `ChatStream` passed; full frontend suite passed 13 files / 99 tests; `npm run build` passed.
- Source navigation and prompt follow-up Rust validation: memory summary / transparency-rule targeted tests passed using project Cargo config and external target directories.
- Source navigation and prompt follow-up dev restarts: restarted `npm run tauri dev` after fixes and reached ready markers for manual verification.

### Completion Notes List

- Story 2.9 implemented end-to-end: backend prompts now inject real `[记忆#id]` references for butler and role paths, share transparency/uncertainty rules, and exclude forgotten memories from new prompt context.
- Added guarded uncertainty post-processing for final visible assistant text on both opencode and fallback streaming paths without touching thinking content or routing metadata.
- Assistant-visible `[记忆#ID]` references now render as accessible clickable controls while user messages and thinking text remain non-navigational.
- Role and butler views route memory-reference clicks through their workspace panels to `MemoryTab`, clear category filters, scroll/highlight target cards, and show unavailable feedback for missing/deleted memories.
- Review follow-up fixed target-memory navigation races by deriving an immediate unfiltered effective memory category while a target ID is active, so `MemoryTab` cannot first evaluate the target under a stale category filter.
- Review follow-up removed raw thinking from user-visible UI and stream persistence: reasoning/thinking SSE parts are ignored for display, `ChatBubble` no longer renders `thinkingContent` or streaming thinking text, and completed-message fallback only uses visible text.
- Added Rust, component, stream, workspace, and view-level regression tests for memory references, target navigation, forgotten memory exclusion, uncertainty handling, category race prevention, and hidden-thinking suppression.
- Validation passed after review fixes: TypeScript check, full frontend tests, full Rust tests, and production build.
- Tauri dev was restarted after validation and reached a ready marker for manual verification.

- Second review follow-up fixed the remaining real `MemoryTab` stale-data race by having `useMemories` hide previous-query results until the current request completes, so target handling waits for the unfiltered result before deciding unavailable.
- Second review follow-up hardened opencode bus delta filtering: unknown part deltas now wait for part-type confirmation, `reasoning` and `thinking` parts are both treated as hidden, and visible text deltas only emit after explicit text classification.
- Validation passed after the second review follow-up: TypeScript check, full frontend tests, full Rust tests, and production build.
- Tauri dev was restarted again after the second review follow-up and reached a ready marker for manual verification.
- Source navigation follow-ups fixed memory source routing across butler and role views by carrying the source conversation `roleId`, validating source messages before navigation, and centering/highlighting the exact source message.
- Memory reference follow-ups changed user-visible references to time labels backed by internal `egosync-memory://<memory-id>` links, preventing same-minute collisions while keeping UUIDs hidden.
- Transparency prompt follow-ups now keep ordinary memory answers source-free, but require explicit source/why/依据 replies to preserve the original clickable memory link format instead of rewriting it as natural-language time.
- Validation passed after source navigation and prompt follow-ups: full frontend tests, targeted Rust prompt/memory tests, and production build; Tauri dev was restarted and reached ready markers.

### File List

- `GUI/src-tauri/src/commands/chat.rs`
- `GUI/src-tauri/src/lib.rs`
- `GUI/src-tauri/src/models/memory.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src-tauri/src/services/memory_query.rs`
- `GUI/src/App.tsx`
- `GUI/src/App.test.tsx`
- `GUI/src/components/butler/ButlerView.tsx`
- `GUI/src/components/butler/ButlerView.test.tsx`
- `GUI/src/components/butler/ButlerWorkspacePanel.tsx`
- `GUI/src/components/butler/ButlerWorkspacePanel.test.tsx`
- `GUI/src/components/chat/ChatBubble.tsx`
- `GUI/src/components/chat/ChatBubble.test.tsx`
- `GUI/src/components/chat/ChatInput.tsx`
- `GUI/src/components/chat/ChatStream.tsx`
- `GUI/src/components/chat/ChatStream.test.tsx`
- `GUI/src/components/role/MemoryTab.tsx`
- `GUI/src/components/role/MemoryTab.test.tsx`
- `GUI/src/components/role/RoleView.tsx`
- `GUI/src/components/role/RoleView.test.tsx`
- `GUI/src/components/role/RoleWorkspacePanel.tsx`
- `GUI/src/components/role/RoleWorkspacePanel.test.tsx`
- `GUI/src/hooks/useMemories.ts`
- `GUI/src/services/chatService.ts`
- `GUI/src/types/chat.ts`
- `GUI/src/types/memory.ts`
- `_bmad-output/implementation-artifacts/2-9-reasoning-transparency-uncertainty.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-01: Ultimate context engine analysis completed - comprehensive developer guide created. Status set to ready-for-dev.
- 2026-06-02: Implemented Story 2.9 reasoning transparency, uncertainty handling, clickable memory references, MemoryTab target navigation, and validation coverage. Status set to review.
- 2026-06-02: Addressed code review findings for stale category-filter target navigation and hidden raw-thinking exposure; reran full validation and restarted Tauri dev. Status set to review.
- 2026-06-02: Addressed second review findings for real MemoryTab stale-query handling and hidden thinking bus-delta ordering; reran full validation and restarted Tauri dev. Status set to review.
- 2026-06-02: Added source-message navigation across butler/role views, exact source-message validation, centered scrolling/highlighting, unique hidden memory links behind time labels, and prompt rules for on-demand source disclosure. Status remains review.
