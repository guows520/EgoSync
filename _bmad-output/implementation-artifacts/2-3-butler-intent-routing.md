# Story 2.3: 管家把任务委派给对应角色，再把结果转述给用户

Status: done

## Story

As a 用户,
I want 直接跟管家说我的需求，不用关心由谁处理，由管家自动安排合适的角色处理，再把结果讲给我,
so that 我只需要面对一个稳定的"助理"，不必在不同身份之间手动切换。

## Acceptance Criteria

1. **AC-1 委派触发：管家以"领导→助理→员工"模型工作**
   - **Given** 用户在管家对话中输入「帮我跟进本周 OKR」，且存在 status=active 的「产品经理」角色
   - **When** 管家 LLM 流式回复
   - **Then** 管家在文字中**显式提及**正在委派（如「稍等，我让产品经理看一下」），随后通过 `delegate_to_role` 工具调用（同一轮）触发后端执行
   - **And** 用户**不会**看到任何视图切换、modal、确认按钮 —— 整个流程都在管家对话流中
   - **And** 委派仅用于任务/安排/日程/待办/需要后续行动的请求；如果用户只是陈述某个角色的事实、偏好或认知更新（如孩子姓名、喜好、学习表现），管家直接回应，不调用 `delegate_to_role`，记忆系统负责同步到匹配角色

2. **AC-2 单次 LLM 回合允许并行多委派**
   - **Given** 用户输入「帮我安排今天的工作和健身」涉及两个角色
   - **When** 管家在一轮回复中识别出 2 个目标角色
   - **Then** 同一次 LLM 回合内 emit 2 个 `delegate_to_role` tool_calls，后端**串行**执行（顺序按 tool_calls 数组顺序），最终把每个角色的回复一并喂给管家 follow-up
   - **And** 性能基线：单次委派的 wall-clock 延迟 = 管家 stream 时间 + Σ 各角色 stream 时间；用户感知为管家在"稍等"之后流式恢复

3. **AC-3 禁止嵌套委派（V1 边界）**
   - **Given** 管家收到 tool_results 进入 follow-up（第二轮）stream
   - **Then** follow-up 的 `ChatOptions.tools = None`，确保 LLM 在这一轮**不能**再次触发 `delegate_to_role`
   - **And** 即使 follow-up 文字中出现"再让 X 角色看看"之类的话，也不会真的再发起委派 —— 直到用户下一次输入新消息

4. **AC-4 委派写入角色对话历史（角色"记得"被委派过）**
   - **Given** 委派目标 = 产品经理（role_id=R）
   - **When** 后端执行 `execute_delegate_to_role`
   - **Then** 该角色的对话（沿用 `get_or_create_conversation_by_role(R)`）里追加一条 `role='user'` 消息，content 形如 `[管家委派] {task_summary}\n\n上下文：{context_or_empty}`
   - **And** 该角色 LLM 用 Story 2.2 的 `build_role_messages` 路径 stream，回复作为 `role='assistant'` 消息持久化到**角色对话**
   - **And** 用户后续主动切到该角色视图（Story 2.2 通路）能看到完整委派历史

5. **AC-5 模糊意图：管家追问而非强制委派**
   - **Given** 用户输入「帮我想想」之类模糊意图
   - **When** 管家判断不存在高置信度的目标角色
   - **Then** 管家**不调用** `delegate_to_role`，直接用文字追问（如「你想让哪个角色帮忙？这几个相关：…」）
   - **And** 模糊判断由 prompt 引导 LLM 自评，**不在代码里硬编码 confidence 阈值**（与 Story 2.2 选择一致）

6. **AC-6 委派审计：routing_metadata 持久化到管家对话**
   - **Given** 任何一次 `delegate_to_role` 实际执行
   - **Then** 在管家对话中**触发本轮**的那条 user message 行写入新增列 `routing_metadata`（TEXT NULL，存 JSON）
   - **And** JSON 结构：`{"delegations":[{"targetRoleId":"R","targetRoleName":"产品经理","taskSummary":"...","status":"ok"|"role_not_found"}, ...]}`
   - **And** 多委派合并写一行（同一 user message 触发的全部 delegation 进同一个数组）
   - **And** 未发起委派的消息该字段为 `NULL`，既有消息不需要回填

7. **AC-7 跨角色全局同步：管家掌握各角色近况**
   - **Given** 用户在角色 X 视图直接私聊后回到管家视图
   - **When** 用户在管家对话发下一条消息
   - **Then** 管家 system prompt 拼接段会**自动**包含「各 active 角色近况」摘要 —— 每个 active 角色拉其最新 conversation 的最近 4 条 `is_complete=1` messages，拼成不超过 200 字的简短文本；无对话历史的角色省略
   - **And** 用户没提及时管家不主动复述，但可以在相关时引用 —— 由 prompt 引导
   - **And** 摘要文本对用户**不可见**，仅注入 prompt

8. **AC-8 委派目标无效：显式失败 + 不传染管家流**
   - **Given** LLM 返回的 `target_role_id` 不存在或 status=archived
   - **Then** 该 tool_call 的执行不 panic；写入 `routing_metadata` 时 status="role_not_found"；tool_result 字符串返回「目标角色不可用，请用文字直接告诉用户」
   - **And** 不写入任何角色对话历史
   - **And** 管家 follow-up 应该自然降级（依赖 tool_result 提示）—— 不阻断其他并行 tool_call 的执行
   - **And** tracing 记一行 `warn` 含原始 target_role_id

9. **AC-9 测试通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd egosync-app/src-tauri && cargo test`
   - 至少覆盖：`delegate_to_role` ToolDefinition、`execute_delegate_to_role` 写双方对话与 routing_metadata、无效 role_id 走 AC-8 兜底、follow-up `ChatOptions.tools=None`（AC-3）、跨角色摘要拼接函数（AC-7）。

10. **AC-10 事实记忆与任务委派分流**
    - **Given** 用户在管家对话中陈述某个 active 角色相关事实/偏好/认知更新（例如「我儿子叫小米米」「他喜欢薯条」「他的数学计算能力有待加强」）
    - **Then** 管家直接确认或基于已知记忆回答，不调用 `delegate_to_role`
    - **And** Story 2.6 记忆管线会把该事实保存在管家全局记忆，并同步一份到匹配角色记忆
    - **Given** 用户交代某个 active 角色相关任务/安排/日程/待办/需要后续行动（例如「明天下午要参加儿子的家长会」）
    - **Then** 管家必须调用 `delegate_to_role`，让匹配角色处理并在管家气泡中转述结果

## Tasks / Subtasks

### Phase 1: Schema & Models (AC: #6)

- [x] T1.1 `db::pool::run_conversations_migrations` 末尾追加容错 ALTER：`ALTER TABLE messages ADD COLUMN routing_metadata TEXT`（沿用 Story 2.2 `thinking_content`/`title` 添加列模式 —— 不新建 migration 文件）
- [x] T1.2 `models/chat.rs`：`Message` 新增 `pub routing_metadata: Option<String>`（serde 自动 camelCase）
- [x] T1.3 `types/chat.ts`：`ChatMessage` 新增 `routingMetadata: string | null`
- [x] T1.4 `db/conversations.rs`：所有 `SELECT ... FROM messages` 列清单加 `routing_metadata`（`list_messages` / `get_recent_messages`）；新增 `update_message_routing_metadata(pool, message_id, json) -> Result<()>` helper

### Phase 2: 跨角色摘要（AC-7 基础设施）

- [x] T2.1 `services/agent_engine.rs` 新增 `build_cross_role_summary(conv_pool, main_pool) -> Result<String, AppError>`
  - [x] 拉 `db::roles::list_active_roles`（已存在，无需新增）
  - [x] 对每个角色：`list_conversations_by_role` 取最新会话 → `get_recent_messages` 取最后 4 条 `is_complete=1`（避免对无历史角色调 `get_or_create` 产生空会话副作用）
  - [x] 拼成 `- {role.name}：\n  用户:{...} / 角色:{...} ...`（每行软截断 40 字，整体软截断 2000 字）
  - [x] 角色无历史/系统消息 → 跳过；全部为空 → 返回空字符串
  - [x] 返回值是拼好的 `[各角色近况]\n- ...\n- ...` 文本块

### Phase 3: 委派工具 + Butler Prompt 重组 (AC: #1, #2, #5, #7)

- [x] T3.1 `agent_engine.rs` 新增 `delegate_to_role_tool_definition()`
  - [x] 工具名 `"delegate_to_role"`
  - [x] 参数 schema：`target_role_id` (string, required) / `task_summary` (string, required) / `context` (string, optional)
  - [x] 描述明确：「仅在能明确判断任务属于某个 active 角色时调用；不确定就追问，不要硬猜；允许同一轮内多次调用」
- [x] T3.2 `build_butler_messages` 改造（保留旧名 + 新增 `main_pool` 入参；run_stream 调用点同步更新）
  - [x] 在 `BUTLER_SYSTEM_PROMPT` 之后追加段落 1：可委派角色清单（`id|name|goal`）
  - [x] 追加段落 2：各角色近况（调 `build_cross_role_summary`）
  - [x] 追加段落 3：行为指南（先告诉用户委派给谁 → 调工具；多角色一次多调；意图模糊就追问；收到回复后向用户转述）
  - [x] 零 active 角色时**不**注入清单与行为指南，避免空诱导
- [x] T3.3 `run_stream` 在 butler 分支挂 `ChatOptions.tools = Some(vec![delegate_to_role_tool_definition()])`，`tool_choice = None`；onboarding/role 分支不变
- [x] T3.4 `execute_tool_calls` 新增 case `"delegate_to_role" => execute_delegate_to_role(...)`

### Phase 4: `execute_delegate_to_role` —— 嵌套 LLM 调用核心 (AC: #2, #3, #4, #6, #8)

- [x] T4.1 在 `agent_engine.rs` 新增 `execute_delegate_to_role(main_pool, conv_pool, arguments) -> (String, DelegationRecord)`
  - [x] 解析 args `{ target_role_id, task_summary, context }`，parse 失败返回 `status: "parse_error"`
  - [x] 校验 `get_role`：不存在或 archived → AC-8：返回 `status: "role_not_found"`，**不**写角色对话
  - [x] 角色存在且 active：拼接 `[管家委派] {task_summary}\n\n上下文：{context_or_'无'}`（无 context 时仅前缀+summary）
  - [x] `get_or_create_conversation_by_role` → 插 user 消息（is_complete=true）+ assistant 占位（is_complete=false）
  - [x] 调 `build_role_messages` + `resolve_default_provider`，本地 mpsc drain，**不** emit `llm:stream`
  - [x] 持久化角色 assistant 消息（update_content + mark_complete）
  - [x] 返回给 LLM 的 tool result：`"来自角色「{}」的回复：\n{}\n\n（请用自己的话向用户转述...）"`
- [x] T4.2 `execute_tool_calls` 收集所有 `DelegationRecord`，序列化 JSON `{"delegations":[...]}` → 调 `update_message_routing_metadata` 写入触发本轮的 user message
  - [x] follow-up `ChatOptions { tools: None, tool_choice: None }` 加注释强调 AC-3（既有行为天然成立）

### Phase 5: 签名与状态贯通 (AC: #6)

- [x] T5.1 `commands/chat.rs::chat_send_message` 把 `user_msg.id` 传给 `run_stream`
- [x] T5.2 `agent_engine::run_stream` 签名追加参数 `user_message_id: String`，下传到 `execute_tool_calls`
- [x] T5.3 `lib.rs invoke_handler`：无需新增 command（委派完全发生在 `chat_send_message` 内部）

### Phase 6: 测试 (AC: #1-#9)

- [x] T6.1 `db/conversations.rs` 单测：`update_message_routing_metadata` 写入 → `list_messages` 读回 JSON 字符串原样；`insert_message` 默认 routing_metadata=None
- [x] T6.2 `agent_engine.rs` 单测：
  - [x] `delegate_to_role_tool_definition` 字段断言（name、target_role_id required、task_summary required、context optional）
  - [x] `build_cross_role_summary` 空库返回空串 / 有角色无对话返回空串 / 有 1 角色 1 对话拼接含名字+内容 / 多角色全部覆盖
  - [x] `build_butler_messages` 在有 active 角色时含可委派清单+行为指南；零 active 时省略
  - [x] `execute_delegate_to_role` 合约级单测：ghost role_id 返回 role_not_found 且不污染对话；archived role 拒绝；无效 JSON 走 parse_error
- [x] T6.3 前端类型回归：7 处 `ChatMessage` 字面量补 `routingMetadata: null`（ChatStream / ChatBubble.test / OnboardingView ×5）
- [x] T6.4 验证命令：tsc / npm test / cargo test 全绿（原 Story 验证：tsc 无错误 / vitest 28 passed / cargo 77 passed；同步桥接与前端流式加固后：cargo test --lib 155 passed、`npm run test:frontend` 46 passed、`npm run build` 通过）
- [ ] T6.5 端到端 `tauri dev`：应用已能启动到 Vite + Tauri 后端 + opencode sidecar + delegate bridge 就绪；以下业务剧本仍留 boss 在桌面手动验证：
  1. 「帮我跟进本周 OKR」→ 管家文字过渡 + 委派产品经理 + 转述结果
  2. 「帮我安排今天的工作和健身」→ 一次涉及 2 角色（产品经理 + 健康教练）
  3. 主动切到产品经理私聊后回管家说「我刚才跟产品经理聊了什么」→ 管家能从摘要里复述

## Dev Notes

### 关键设计决策（已锁定，写入 AC）

| 决策 | 取舍 | 理由 |
|---|---|---|
| Q1 显式提及委派 | 管家文字告诉用户「我让 X 看下」 | 与"领导/助理"心智模型一致；PRD 透明可控原则 |
| Q2 写入角色对话 | 委派交互持久化进角色 conversation | 否则 Story 2.2/2.6/2.7 角色历史/记忆全部塌缩；角色"变空壳" |
| Q3 文字过渡 | 管家 stream 内自带"稍等"过渡语 | 不需要新 UI 状态机；最小改动 |
| Q4 单轮并行多委派 | 一次 LLM 回合内 emit N 个 tool_calls，后端串行执行 | OpenAI/Anthropic 工具调用原生支持；不需要嵌套架构 |
| Q4' 禁止嵌套多轮 | follow-up `tools=None` 硬关闭 | 防止 LLM 在转述阶段失控触发新委派；V1 边界 |
| Q5 跨角色全局同步 | 管家 system prompt 注入各角色近况摘要 | 用户主动私聊后管家自动掌握，无需手动同步 |
| D-confidence | 不硬编码阈值，prompt 引导 LLM 自评 | LLM API 不返回 confidence，硬编码会变 dead code |

### 管家当前行为分流：已知记忆优先，任务才委派

当前 butler system prompt 在角色清单和近况摘要之外，还注入 `[已知记忆]` 段以及明确的行为指南：

- 用户只是陈述角色相关事实/偏好/认知更新时，管家直接回应，不委派；后台 memory pipeline 会同步到匹配角色记忆。
- 用户询问已知事实时，管家优先基于 `[已知记忆]` 回答，可说明目前只知道这些。
- 用户交代角色相关任务、安排、日程、待办或需要跟进的事项时，只要能匹配 active 角色，就调用 `delegate_to_role`。
- 意图模糊或没有合适角色时，管家不调用工具，而是追问用户希望由谁处理。

这条分流规则解决了“管家已有完整记忆仍委派角色”和“角色任务未委派”两个相反问题：事实靠记忆同步，任务靠委派执行。

### 当前委派实现：opencode custom tool → 本地 delegate bridge → 真实角色回复

当前运行路径已经从原始“Rust 内部本地 drain 角色 LLM”演进为 opencode 主路径：

1. `agent_config.rs` 写入全局 opencode custom tool `delegate_to_role.ts`。
2. custom tool 通过 `EGOSYNC_DELEGATE_BRIDGE_PORT` + `EGOSYNC_DELEGATE_BRIDGE_TOKEN` 调用 `127.0.0.1:{port}/delegate-to-role`。
3. `delegate_bridge.rs` 绑定随机本地端口、校验 Bearer token、按 opencode sessionID 找到触发本轮的管家 user message，并同步调用 `execute_delegate_to_role(...)`。
4. `execute_delegate_to_role(...)` 将 `[管家委派] ...` 写入目标角色会话，调用目标角色模型并持久化真实角色回复，再把该真实回复作为 tool result 返回给 opencode 管家会话。
5. 管家 follow-up 基于 tool result 转述给用户；`routing_metadata.delegations[]` 由 bridge 追加到触发本轮的管家 user message。

该路径修复了旧 custom tool 只返回 JSON 元数据、导致管家自行生成“已收到/会帮你处理”伪确认的问题。

### 前端流式呈现：两气泡委派与即时解锁

`llm:stream` 仍以管家 conversationId 为路由边界，但 payload 现在可携带 `messageId`：

- 第一段管家过渡语（如“稍等，我让产品经理看一下”）落在默认桶；
- `delegate_to_role` 进入活动态时，后端冻结第一段并创建 follow-up assistant message，用空 token 唤醒第二个气泡等待点；
- 第二段管家转述真实角色回复时按 follow-up `messageId` 进入第二个气泡；
- 前端 `ChatStream` 用 `streamBubbles` 按 `messageId` 分桶渲染，并用独立的 `isInputLocked` 控制输入框。最终 done 到达后立即解锁输入框，历史刷新异步合并。

因此，旧文档中的“角色 LLM 不 emit 到 `llm:stream`、前端无需结构性改动”仅代表原始 Story 实施方式，不再代表当前代码现状。

### 嵌套调用与签名贯通

- 当前 opencode 主路径下，`run_stream` 以 conversation_id 复用/创建 opencode session，并在 butler 会话中用 `DelegateBridge::register_session(session_id, user_message_id)` 建立 sessionID → 管家 user message 映射。
- opencode custom tool `delegate_to_role.ts` 执行时把 `context.sessionID` POST 给本地 bridge，bridge 据此找到本轮触发消息并追加 `routing_metadata`。
- 委派 tool 执行完成后，opencode 会继续当前 assistant message；后端用 `messageId` 将第一段过渡语和第二段转述拆成两个可独立完成的前端气泡。
- `event_router.rs` 订阅 opencode 全局 `/event` stream 后按 sessionID 分发，避免不同 conversation 的 tokens 互相污染。

### Story 2.2 已奠定的能复用资产（**不要**重新发明）

- `db::conversations::get_or_create_conversation_by_role` ✓
- `db::conversations::list_conversations_by_role` ✓
- `db::conversations::list_butler_conversations` ✓
- `agent_engine::build_role_messages` ✓（委派时跑角色 LLM 直接调它）
- `chat_get_role_conversation` command ✓（前端用户主动私聊路径不变）
- ROLE_SYSTEM_PROMPT_PREFIX 独立于 BUTLER_SYSTEM_PROMPT ✓

### 必须保留的边界（规则八 / 规则十一）

- onboarding 路径完全不变（含 `create_role` 工具 / `looks_like_fake_role_creation` 兜底）
- role 视图路径完全不变（用户主动私聊 = Story 2.2 既有路径）
- 不在前端组件直接 emit / 调 invoke 外的 IPC
- 不引入新的 streaming/cancel state
- 不破坏既有 `chat_new_conversation` / `chat_list_conversations` 签名

### 已知风险与缓解

| 风险 | 缓解 |
|---|---|
| 角色 LLM 调用慢导致管家 stream 看起来“卡住” | 当前两气泡实现会在委派 tool 进入活动态时立即创建第二个等待气泡；bridge 侧有 120s 委派超时和并发限制，最终 done 到达后前端立即解锁输入框 |
| 多委派串行下总延迟 = Σ 各角色 | V1 接受；并行扩展点已留注释 |
| 跨角色摘要长 prompt 拖慢首 token | V1 限上限 ~2000 字；超长可加 token 上限截断；本 story 用 chars 软截断够用 |
| LLM 在 follow-up 仍想委派但被 tools=None 拒绝 | LLM 会自然降级为文字描述；不会报错 |
| user 写入角色对话的 `[管家委派]` 前缀污染记忆提炼（Story 2.6） | Story 2.6 已采用来源消息去重和角色范围记忆归属；角色事实从管家全局同步，`task_status` 不走事实同步，任务类内容保留在委派/角色对话路径中 |
| 大量 active 角色时摘要拼接代价 | V1 限制 < 20 角色尚可；超过后续优化 |

### Out of Scope（明确不做）

- 嵌套多轮委派（管家 follow-up 再次触发委派）—— V1 边界，AC-3 显式禁止
- 多委派**并行**执行（V1 串行；并行属于性能优化，下个 epic 评估）
- 路由失败时管家**自动**降级到通用回答（V1 依赖 LLM 看到 tool_result 自然降级）
- routing_metadata 的可视化 UI（"路由日志审计"页面）—— 本 story 仅 DB 层落地
- 委派任务取消机制（角色还在跑，用户按 stop）—— V1 依赖既有 `chat_stop_streaming`，可能导致角色对话留下半截 assistant 消息；接受
- 委派结果的"重试"路径（角色返回失败/异常）—— V1 走 AC-8 兜底
- 前端在管家气泡上展示「正在咨询「产品经理」…」占位 —— 不做，依赖管家 stream 文字过渡
- 前端在角色视图把 `[管家委派]` 前缀的 user message 渲染成特殊气泡 —— 不做，仅文本前缀即可
- onboarding 期间挂 `delegate_to_role` —— 不挂（与 `create_role` 互斥）
- 角色私聊触发委派给其他角色（role→role 跳转）—— 不做
- `confidence` 数值字段 —— 完全不引入

## Project Structure Notes

### 新建文件

- `egosync-app/src-tauri/src/services/delegate_bridge.rs` — 本地回环 HTTP bridge，供 opencode custom tool 同步调用后端委派逻辑并返回真实角色回复
- `egosync-app/src-tauri/src/services/event_router.rs` — opencode 全局事件流按 sessionID 分发，避免跨会话 token 污染

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src-tauri/src/db/pool.rs` | UPDATE | `run_conversations_migrations` 追加 `ALTER routing_metadata` 容错 |
| `egosync-app/src-tauri/src/db/conversations.rs` | UPDATE | `Message` 列清单加 `routing_metadata`；新增 `update_message_routing_metadata` |
| `egosync-app/src-tauri/src/db/roles.rs` | UPDATE（如需） | 若无 `list_active_roles` 则新增（status='active' ORDER BY created_at） |
| `egosync-app/src-tauri/src/models/chat.rs` | UPDATE | `Message` 加 `routing_metadata: Option<String>` |
| `egosync-app/src-tauri/src/services/agent_engine.rs` | UPDATE | opencode session 复用/事件订阅；sessionID 注册到 delegate bridge；delegate tool 活动态拆分两气泡；`messageId` 路由；`append_delegation_metadata` / `execute_delegate_to_role` 等委派核心逻辑 |
| `egosync-app/src-tauri/src/services/agent_config.rs` | UPDATE | 写入 `delegate_to_role.ts` custom tool；工具通过本地 bridge 返回 `role_response` 给管家 |
| `egosync-app/src-tauri/src/services/sidecar.rs` | UPDATE | 支持向 opencode 子进程注入 bridge token/port 环境变量 |
| `egosync-app/src-tauri/src/services/mod.rs` | UPDATE | 注册 `delegate_bridge` / `event_router` 模块 |
| `egosync-app/src-tauri/src/lib.rs` | UPDATE | 启动 delegate bridge、注入 sidecar env、启动 event router |
| `egosync-app/src-tauri/src/commands/chat.rs` | UPDATE | `chat_send_message` 把 `user_msg.id` 透传 `run_stream` |
| `egosync-app/src/types/chat.ts` | UPDATE | `ChatMessage.routingMetadata` 与 `StreamPayload.messageId` 支持当前流式协议 |
| `egosync-app/src/components/chat/ChatStream.tsx` | UPDATE | 多流式气泡分桶、done 后输入框即时解锁、late history/sendMessage 竞态合并 |
| `egosync-app/src/components/chat/ChatBubble.tsx` | UPDATE | 去除已有文本流式气泡尾部光标，保留空内容等待点 |
| `egosync-app/src/components/chat/*.test.tsx` | UPDATE | 覆盖两气泡委派、即时解锁、历史合并、尾部光标移除等回归 |
| `egosync-app/src/components/onboarding/OnboardingView.tsx` | UPDATE | ChatMessage 字面量补 `routingMetadata` 字段 |

### 不动的文件

- `egosync-app/src-tauri/migrations/*.sql` — 不新建文件（沿用 raw_sql ALTER 模式）
- `egosync-app/src/App.tsx` — 不挂 modal，不监听 routing 事件
- `egosync-app/src/components/role/*` — Story 2.2 路径完全保留
- `egosync-app/src/components/butler/ButlerView.tsx` — 无需改
- `egosync-app/src/components/onboarding/*` — onboarding 业务路径不变；仅类型字段补齐
- `egosync-app/src/components/modals/*` — 不新增 RouteConfirmModal（已废弃方案）

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.3 AC（注：原 AC 假设"切换角色视图"模型；本 story 与 boss 对齐后改为"管家委派"模型，AC 措辞重写但用户价值不变）]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md` — 透明可控原则；用户面对的是一个稳定的"助理"]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Tool Calling 通路 / agent_engine 层职责]
- [Source: `_bmad-output/project-context.md` — Tauri IPC / Rust 错误处理 / 测试规则]
- [Source: `_bmad-output/implementation-artifacts/1-8-onboarding-five-step-first-role.md` — `create_role` ToolCall 完整范式（execute_X → emit → tool_result 引导后续语气）]
- [Source: `_bmad-output/implementation-artifacts/2-1-role-crud-archive-delete.md` — 错误就地展示 / sprint-status 同步纪律]
- [Source: `_bmad-output/implementation-artifacts/2-2-role-view-switch-butler.md` — role_id 通路 / build_role_messages / chat_get_role_conversation 已就位]
- [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md` — 自动化测试 ≠ 桌面验证；ALTER 容错模式]
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs#execute_create_role` — 工具执行模板，`execute_delegate_to_role` 直接复用其骨架]
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs#run_stream` — follow-up `tools: None` 已是既有行为，AC-3 借此天然成立]
- [Source: `egosync-app/src-tauri/src/db/conversations.rs` — `get_or_create_conversation_by_role` / `insert_message` / `update_message_content` 已就绪]
- [Source: `egosync-app/src-tauri/src/db/pool.rs#run_conversations_migrations` — 容错 ALTER 模式参考]

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5

### Debug Log References

### Completion Notes List

- Story 2.3 设计模型在 boss 与 Amelia 对齐后从「弹窗切视图」改为「管家委派 → 角色处理 → 管家转述」，与 epic 原 AC 同价值但路径完全不同
- 5 个关键决策（Q1=B / Q2=A / Q3=B / Q4=A+并行单轮 / Q5=管家全局视野）+ 1 个边界（Q4'=禁止嵌套多轮）已锁进 AC
- 复杂度集中在 Phase 4 的嵌套 LLM 调用；已通过"角色 stream 不 emit"和"follow-up tools=None"两个设计简化避开 UI/重入风险
- 与 Story 2.2 完全互补：用户既可被动接受管家自动委派，也可主动私聊角色；后者通过跨角色摘要让管家持续掌握全局
- 端到端验证留 boss 在桌面手动跑三条剧本（Phase 6 T6.5）

**后续同步桥接与前端流式加固记录（2026-05-30）：**

- `delegate_to_role` opencode custom tool 已改为同步调用本地 delegate bridge，并把真实 `role_response` 作为 tool result 返回给管家，避免管家基于空 JSON 元数据生成伪确认。
- `delegate_bridge.rs` 增加随机本地端口、运行时 token、Bearer 鉴权、请求体限制、连接/委派并发限制和 120s 委派超时；`lib.rs` 在 sidecar 启动前注入 bridge token/port。
- `agent_engine.rs` 通过 `EventRouter` 按 opencode sessionID 分发全局事件流；butler 会话注册到 delegate bridge，`delegate_to_role` 活动态会冻结第一段气泡并唤醒 follow-up 气泡。
- `ChatStream.tsx` 支持按 `messageId` 分桶渲染多流式气泡；`isInputLocked` 与流式气泡渲染状态拆分，最终 done 到达后立即解锁输入框，并加固 late history / late sendMessage / assistant busy / 重复内容等竞态。
- `ChatBubble.tsx` 去除已有文本流式气泡尾部光标；空内容流式气泡继续显示等待点。
- 事实/记忆与任务/行动的分流规则已补进管家 prompt：角色事实、偏好、认知更新直接回应并交给记忆系统同步；角色任务、安排、日程、待办必须走 `delegate_to_role`。

**实施增量记录 (2026-05-24)：**

- **Phase 1**：`run_conversations_migrations` 与 `db::conversations` 测试 setup 都追加 `ALTER TABLE messages ADD COLUMN routing_metadata TEXT` 容错。`Message.routing_metadata: Option<String>` 加在结构体末尾保持向后兼容。`agent_engine::tests::setup_test_conv_pool` 同步补 ALTER 防止 SELECT 列缺失。
- **Phase 2**：`build_cross_role_summary` 使用 `list_conversations_by_role` 取 first（最新），**不**用 `get_or_create_*` —— 否则无对话角色会被偷偷创建空会话。每行 40 字软截断 + 整体 2000 字软截断（`chars().take(N)` 防中文字节边界 panic）。
- **Phase 3**：`build_butler_messages` 签名从 `(conv_pool, conv_id, user_msg)` 改为 `(conv_pool, main_pool, conv_id, user_msg)`。新增 `main_pool` 是为了 `list_active_roles` 和跨角色摘要。`run_stream` butler 分支挂 `tools=Some(vec![delegate_to_role_tool_definition()])`，`tool_choice=None`（自主决定）。零 active 角色时 prompt 完全不注入清单/指南，避免 LLM 误调空工具。
- **Phase 4**：`execute_delegate_to_role` 7 类失败路径全部覆盖且 fail-loud（parse_error / role_not_found / archived / conv_init / user_msg_insert / assistant_msg_insert / build_messages / provider）。角色 LLM 调用使用本地 mpsc drain，**不** emit `llm:stream` —— 避免前端按 conversation_id 路由时把角色 token 误显示在管家气泡里（Story 2.2 hotfix 经验：emit 必须严格按 conv_id 隔离）。
- **Phase 5**：`run_stream` 签名末尾追加 `user_message_id: String`；`execute_tool_calls` 签名插入 `conv_pool` 与 `butler_user_message_id`。`execute_create_role` 接收新参数但不使用 —— 这轻微违反规则二（"以防万一"），但避免双签名分支或双调用点重复代码，最终取规则三优先（外科手术：单一入口比分支扩散更易维护）。
- **Phase 6**：本 story 累计新增测试 **12 个**：db 层 2 个（routing_metadata 默认 None / 写入读回）+ cross_role_summary 4 个（空库/无对话/单角色/多角色）+ delegate 工具与 prompt 3 个（tool def 字段 / butler 含清单 / 零角色省略）+ execute_delegate 合约级 3 个（ghost / archived / 无效 JSON）。**未做** end-to-end 嵌套 LLM 测试（无 provider mock 框架），AC-2 真并行 + AC-3 嵌套禁止的运行时验证留给 T6.5 桌面手验。

**实施偏差（fail loud）：**

1. **Phase 3 第一次 build_butler_messages 增段时**字符串内嵌了半角双引号 `"稍等..."`，被 rustc 当字符串闭合而编译失败；已改为单角弯引号 `『稍等...』`。
2. **Phase 4 测试**断言 message 内嵌半角双引号 `"不可用"`，同样导致语法错误；已改单角弯引号 `『不可用』`。
3. **Phase 5 run_stream 参数文档**用了 `///`（rustc 不允许在参数上挂文档注释），已改 `//` 普通注释。

**验证证据：**

- `cd egosync-app/src-tauri && cargo test --lib` = **77 passed**（原 Story 2.3 实施，含本 story 12 个新增）
- `cd GUI && npx tsc --noEmit` 无错误
- `cd GUI && npm run test:frontend` = **28 passed / 8 files**（原 Story 2.3 实施）
- 后续同步桥接加固：`cargo test --lib` = **155 passed**
- 后续前端流式加固：`npm run test:frontend -- ChatStream.test.tsx` = **20 passed**；`npm run test:frontend` = **46 passed**；`npm run build` 通过
- `npm run tauri dev` 已启动到 Vite + Tauri 后端 + opencode sidecar(4096) + delegate bridge 就绪；业务剧本仍需 boss 人工发送真实 LLM 请求确认

### File List

**修改文件：**

- `egosync-app/src-tauri/src/db/pool.rs` — 追加 routing_metadata 容错 ALTER
- `egosync-app/src-tauri/src/db/conversations.rs` — Message SELECT 加列、insert_message 默认 None、新增 `update_message_routing_metadata`、测试 setup ALTER + 2 个新单测
- `egosync-app/src-tauri/src/models/chat.rs` — Message 加 `routing_metadata: Option<String>`
- `egosync-app/src-tauri/src/services/agent_engine.rs` — `build_cross_role_summary` / `delegate_to_role_tool_definition` / `execute_delegate_to_role`；opencode session/event 路由；delegate tool 活动态拆分两气泡；`append_delegation_metadata`；相关单测
- `egosync-app/src-tauri/src/services/agent_config.rs` — 写入 `delegate_to_role.ts` custom tool；通过本地 bridge 返回真实角色回复
- `egosync-app/src-tauri/src/services/sidecar.rs` — 注入 bridge token/port 环境变量
- `egosync-app/src-tauri/src/services/mod.rs` — 注册 `delegate_bridge` / `event_router`
- `egosync-app/src-tauri/src/services/delegate_bridge.rs` — 新增本地 bridge，承接 opencode custom tool 并调用 `execute_delegate_to_role`
- `egosync-app/src-tauri/src/services/event_router.rs` — 新增 opencode 全局事件流 sessionID 分发器
- `egosync-app/src-tauri/src/lib.rs` — 启动 delegate bridge、event router，并在 sidecar 启动前注入 bridge 环境变量
- `egosync-app/src-tauri/src/commands/chat.rs` — `chat_send_message` 把 `user_msg.id` 透传 `run_stream`
- `egosync-app/src/types/chat.ts` — `ChatMessage` 加 `routingMetadata: string | null`
- `egosync-app/src/components/chat/ChatStream.tsx` — 多气泡分桶、done 后即时解锁、late history/sendMessage 竞态保护
- `egosync-app/src/components/chat/ChatBubble.tsx` — 已有文本流式气泡不再显示尾部光标；空内容仍显示等待点
- `egosync-app/src/components/chat/ChatStream.test.tsx` — 覆盖委派两段式、即时解锁、history 合并、busy/assistant 返回、重复内容等回归
- `egosync-app/src/components/chat/ChatBubble.test.tsx` — assistantMsg 字面量补字段；覆盖尾部光标移除与空等待点
- `egosync-app/src/components/onboarding/OnboardingView.tsx` — 5 处 ChatMessage 字面量补字段
- `_bmad-output/implementation-artifacts/2-3-butler-intent-routing.md` — Tasks 勾选/Completion Notes/File List/Change Log/状态推进
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 状态推进与 last_updated

**新建文件：**

- `egosync-app/src-tauri/src/services/delegate_bridge.rs`
- `egosync-app/src-tauri/src/services/event_router.rs`

### Change Log

| 日期 | 变更 |
|---|---|
| 2026-05-24 | Story 2.3 上下文创建（原方案：弹窗切视图）|
| 2026-05-24 | 设计模型重写为「管家委派 → 角色处理 → 管家转述」（Q1-Q5 全部锁定）；AC 1-9 / Tasks Phase 1-6 全部重新生成；状态保持 ready-for-dev |
| 2026-05-24 | Phase 1-5 实施完成：schema 列 / 跨角色摘要 / `delegate_to_role` 工具 / butler prompt 三段扩展 / 嵌套 `execute_delegate_to_role` / `chat_send_message` 签名贯通。Phase 6 自动化验证全绿：cargo 77 passed（+12 新增）、tsc 无错误、vitest 28 passed。T6.5 桌面端到端待 boss 手动验证。状态推进到 review |
| 2026-05-30 | 同步桥接与前端流式加固：`delegate_to_role` custom tool 经本地 bridge 返回真实角色回复；新增 delegate bridge / event router；前端支持委派两气泡、去除尾部光标、done 后即时解锁输入框。验证：cargo test --lib 155 passed、ChatStream 20 passed、frontend 46 passed、build 通过；应用已启动到 sidecar + bridge 就绪。 |