---
title: '修复 Agent 错误后流式内容与历史消息不一致导致的重复气泡'
type: 'bugfix'
created: '2026-07-23'
status: 'done'
baseline_commit: 'b9b75b1eb92c5fe5495fbe8c53ebd3110c33eb81'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/agent-error-duplicate-execution-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Agent 已流式显示部分执行提示后，如果后端再把用户可见错误文案只追加到持久化消息、却未通过 `llm:stream` 补发，前端会同时保留本地部分消息与历史完整消息，造成“好的，立即开始执行……”重复显示。该不一致也让错误呈现依赖历史回载，而非完整的流式协议。

**Approach:** 在后端所有“部分输出后追加用户可见错误后缀”的分支中，将同一后缀同步发出为流式 token，再照常持久化并发送 `done`；添加聚焦的前端回归测试，验证完成后的历史协调只留下一个包含执行提示和错误文案的助手气泡。

## Boundaries & Constraints

**Always:** 保持不变量 `streamed visible content == persisted visible content`；覆盖 `session.error`、POST 在已有部分输出后的失败、以及晚到 POST 结果失败三个已确认分支；沿用现有 `emit_stream_token`、消息持久化和 `done` 事件机制；修改必须局限于错误后缀同步及其回归测试。

**Ask First:** 若修复必须改变事件协议字段、消息 ID 生成/协调规则、数据库结构，或需要修改 sidecar 生命周期与 watchdog 策略，则停止并请求批准。

**Never:** 本次不调整 watchdog 超时、失败阈值或 active-request guard；不重构错误分类，不改变“模型服务暂时不可用”的映射文案；不新增诊断日志；不通过前端前缀匹配或模糊去重掩盖后端协议不一致；不顺手重构相邻代码。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 正常完成 | 流式 token 后正常完成并持久化 | 现有行为保持不变，仅显示一个完整助手消息 | 不新增错误处理 |
| `session.error` | 已有部分助手文本，随后收到 session 错误 | 错误后缀先作为流式 token 发出，再持久化同一完整文本并 `done` | 保留现有错误映射和结束流程 |
| POST 中途失败 | 已有部分助手文本，POST 请求失败 | UI 连续收到“部分文本 + 错误后缀”，历史回载后仍只有一个气泡 | 保留现有持久化与错误返回语义 |
| 晚到 POST 失败 | 主循环结束后才确认 POST 结果失败 | 与中途失败相同，流式可见内容和持久化内容逐字一致 | 保留现有晚失败分支控制流 |
| 无部分文本的失败 | 错误发生前没有可见 token | 不引入空 token、重复错误或额外助手气泡 | 沿用当前无输出失败行为 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 组装 `accumulated_text`/`final_text`、处理三类错误分支、发出 `llm:stream` 与 `done`。
- `egosync-app/src/components/chat/ChatStream.tsx` -- 将流式气泡转为完成消息并与重新加载的历史消息协调；预期无需生产代码修改。
- `egosync-app/src/components/chat/ChatStream.test.tsx` -- 已有 ChatStream 事件与历史回载测试基建；新增重复气泡回归用例。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 在三个已确认错误路径中，对追加到最终可见文本的错误后缀调用现有流式发送函数，确保发送顺序为“错误后缀 token → 持久化/结束处理 → done”。
- [x] `egosync-app/src/components/chat/ChatStream.test.tsx` -- 模拟先收到执行提示、最终历史包含执行提示与错误、再收到 `done` 的完整时序，防止本地部分消息与持久化消息并存。
- [x] 仅在现有 Rust 测试基建允许以小范围单元测试验证辅助行为时补充后端测试；不得为此引入新抽象或大规模重构。

**Acceptance Criteria:**
- Given Agent 已流式输出“好的，立即开始执行。先检查环境再生成 PDF。”，when 后端追加“抱歉，Agent 引擎返回错误：模型服务暂时不可用”并结束，then 用户最终只看到一个助手气泡，且执行提示和错误文案各出现一次。
- Given 任一已确认错误分支追加用户可见后缀，when `done` 发出，then 此次已流式发送的可见文本与数据库持久化的最终可见文本一致。
- Given 请求正常完成或错误发生前无部分输出，when 新逻辑运行，then 不产生额外空 token、重复错误文案或行为回归。
- Given 修复完成，when 执行定向前端测试和 Rust 测试，then 所有命令成功；任何失败或跳过必须明确报告，不能宣称验证通过。

## Spec Change Log

## Verification

**Commands:**
- `npm run test:frontend -- src/components/chat/ChatStream.test.tsx`（在 `egosync-app` 下）-- expected: 新增回归用例及现有 ChatStream 用例全部通过。
- `cargo test agent_engine`（在 `egosync-app/src-tauri` 下）-- expected: 与 Agent 引擎相关的 Rust 测试全部通过。
- `git diff --check` -- expected: 无空白错误；差异仅包含规格、必要后端修改及聚焦测试。





## Verification Results

- `npm run test:frontend -- src/components/chat/ChatStream.test.tsx`: PASS（51/51）。
- `cargo test services::agent_engine::tests::test_sse_error_maps_to_done_payload -- --exact`: PASS（1/1，补丁编译成功）。
- `cargo test agent_engine`: PARTIAL（109/110）；唯一失败为未修改区域的既有 Butler prompt 断言，已记录到 `deferred-work.md`。
- `cargo fmt --check`: BASELINE FAIL；报告大量未修改文件历史格式差异，未执行全仓格式化。
- `git diff --check`: PASS。

## Suggested Review Order

**错误流一致性**

- 按当前气泡路由追加并流送 `session.error`，保持持久化一致。
  [`agent_engine.rs:2895`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2895)

- POST 失败补发同一错误后缀，避免本地残缺消息。
  [`agent_engine.rs:2943`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2943)

- 晚到失败去重，防止 session 与 POST 双重报错。
  [`agent_engine.rs:3075`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3075)

**回归验证**

- 复现事故时序并明确断言只剩一个助手气泡。
  [`ChatStream.test.tsx:1716`](../../egosync-app/src/components/chat/ChatStream.test.tsx#L1716)

- 记录与本次补丁无关的既有 Rust 门禁失败。
  [`deferred-work.md:206`](deferred-work.md#L206)
