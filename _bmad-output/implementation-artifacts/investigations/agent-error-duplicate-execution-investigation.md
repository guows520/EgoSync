# Investigation: Agent 引擎错误与执行提示重复显示

## Hand-off Brief

1. **What happened.** opencode 在活跃请求期间被 watchdog 判定不健康并杀死重启；后端把错误追加到持久化消息但未同步发送给流式 UI，导致前端同时保留“部分临时消息”和“完整持久化消息”。
2. **Where the case stands.** 重复显示的因果链和引擎错误的直接触发机制均已确认；仅“opencode 为什么两次健康检查超时”因 stderr 在重启时未记录并被清空而无法追溯。
3. **What's needed next.** 优先修复流式/持久化一致性并增加回归测试，再改造 watchdog 的活跃请求保护和 stderr 诊断；本调查未实施任何业务代码修改。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-23 |
| Status           | Concluded |
| System           | Windows；EgoSync UAT；“UAT-全开角色”；Agent 引擎为 opencode sidecar（具体构建版本待核实） |
| Evidence sources | 用户提供的完整对话摘录；项目源码与运行日志尚未勘查 |

## Problem Statement

用户报告两个相邻症状：

1. Agent 在确认生成 PDF 后报错：“抱歉，Agent 引擎返回错误：模型服务暂时不可用”。
2. 进度消息“好的，立即开始执行。先检查环境再生成 PDF。”在错误前后重复显示。

本次范围仅为原因分析、证据规划与修复方案设计；不生成 PDF、不修改代码、不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的 UAT 对话摘录 | Available | 确认消息顺序和可见症状；没有精确时间戳、message/session ID |
| EgoSync 运行日志 `%APPDATA%\com.egosync.app\egosync.log` | Missing | 可确认 sidecar 请求、模型错误、重试/恢复与事件发送顺序 |
| opencode session/message 数据 | Missing | 可确认重复消息是否由引擎生成、重复落库或仅前端重复渲染 |
| 前端消息状态与事件订阅代码 | Partial | 项目源码可用，尚未进入源码追踪阶段 |
| Rust agent/delegate bridge 代码 | Partial | 项目源码可用，尚未进入源码追踪阶段 |
| 复现录像/控制台日志 | Missing | 可区分瞬时重复渲染、持久化重复和重启后回放 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 对齐 egosync.log 中该次请求的时间线 | High | Open | 找到模型服务错误的原始状态码、provider、请求阶段和是否触发重试 |
| 2 | 追踪进度消息从生成到 UI 渲染的完整链路 | High | Open | 区分模型输出、宿主合成消息、数据库回放、SSE/Tauri 事件重复 |
| 3 | 检查错误后的自动恢复、重试、resume/replay 逻辑 | High | Open | 判断错误后为何再次出现相同执行提示 |
| 4 | 核对前端事件监听注册与消息去重策略 | Medium | Open | 排查重复 listener、StrictMode 生命周期、缺少 message ID 去重 |
| 5 | 核对 opencode/provider 可用性与错误映射 | Medium | Open | 区分真实上游不可用、超时/鉴权/限流被统一映射为“暂时不可用” |
| 6 | 形成最小修复方案和回归验证矩阵 | Medium | Open | 仅在根因证据充分后输出，不直接实施 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 未提供 | 用户确认使用默认路径与文件名，输入“ok” | 用户对话摘录 | Confirmed |
| 未提供 | 系统显示“好的，立即开始执行。先检查环境再生成 PDF。” | 用户对话摘录 | Confirmed |
| 未提供 | 系统显示“抱歉，Agent 引擎返回错误：模型服务暂时不可用” | 用户对话摘录 | Confirmed |
| 未提供 | 系统再次显示完全相同的执行提示 | 用户对话摘录 | Confirmed |

## Confirmed Findings

### Finding 1: 重复内容跨越错误消息出现

**Evidence:** 用户提供的对话摘录（本案输入；无时间戳）

**Detail:** 相同的执行提示不是连续双渲染，而是分别位于 Agent 引擎错误消息之前和之后。这使“错误后的恢复/重放”成为必须检查的路径，但尚不能排除 UI 重排或持久化回放。

### Finding 2: 任务并未产生用户预期的 PDF 保存结果

**Evidence:** 用户提供的对话摘录（本案输入；无保存成功消息或输出路径）

**Detail:** 对话停留在执行提示和引擎错误，未出现工具执行结果、文件路径或完成确认。因此不能把前面的“已成功提取”或“立即开始执行”视为真实文件操作已完成。

## Deduced Conclusions

### Deduction 1: 两个症状不能在当前证据下合并为单一根因

**Based on:** Finding 1、Finding 2

**Reasoning:** 模型服务错误可能发生在 provider/sidecar 层；重复显示可能发生在引擎会话恢复、Rust 事件桥接、前端订阅或消息持久化层。当前仅有 UI 文本顺序，没有层级标识或 message ID。

**Conclusion:** 后续调查必须分别追踪“错误来源”和“重复消息来源”，再用时间线判断两者是否因果相连。

## Hypothesized Paths

### Hypothesis 1: 错误恢复逻辑重放了最后一条 assistant 进度消息

**Status:** Open

**Theory:** provider 请求失败后，会话被恢复或重新订阅；恢复流程把最后一条 assistant 消息重新发送/追加到 UI。

**Supporting indicators:** 相同消息恰好出现在错误前后。

**Would confirm:** 日志显示 error 后发生 resume/reconnect/replay，并再次发送相同 message ID 或内容。

**Would refute:** 引擎消息库中存在两条独立的新消息，且没有恢复/重放事件。

**Resolution:** 待查。

### Hypothesis 2: 前端存在重复事件订阅或缺少按 message ID 去重

**Status:** Open

**Theory:** 同一条后端事件被两个 listener 处理，或历史消息加载与实时事件合并时重复追加。

**Supporting indicators:** 重复文本完全一致；React 开发/UAT 环境中的 listener 生命周期是常见风险点。

**Would confirm:** 单个后端 message/event ID 在前端被处理两次，或 listener 注册数大于 1。

**Would refute:** 后端日志明确显示生成并发送了两个不同事件/消息。

**Resolution:** 待查。

### Hypothesis 3: provider 的具体错误被过度归一化为“模型服务暂时不可用”

**Status:** Open

**Theory:** 超时、鉴权失败、限流、模型不存在或网络错误被统一映射成同一用户提示，掩盖真实根因。

**Supporting indicators:** 当前错误文案没有 provider、HTTP 状态码或可诊断细节。

**Would confirm:** 原始日志中的错误类型不是服务端 5xx/unavailable，但 UI 最终仍显示该统一文案。

**Would refute:** 原始 provider 响应明确为服务不可用，且映射保留了正确语义。

**Resolution:** 待查。

### Hypothesis 4: 角色/编排层在错误后自动重新进入同一执行步骤

**Status:** Open

**Theory:** “UAT-全开角色”的循环或任务恢复机制在失败后再次投递相同指令，产生第二条独立 assistant 消息。

**Supporting indicators:** 第二条文本保持动作承诺语气，可能来自重新运行同一 prompt。

**Would confirm:** 存在两个不同 turn/message ID，且第二次由 orchestrator retry/resume 触发。

**Would refute:** 第二次显示没有新的引擎 turn，只是同一事件重复消费。

**Resolution:** 待查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 故障发生的精确时间 | 无法从日志定位对应请求 | 用户提供大致时间，或从当前日志按错误文案检索 |
| egosync.log 原始错误上下文 | 无法判定模型错误真实类型及重试行为 | 只读检查 `%APPDATA%\com.egosync.app\egosync.log` |
| session ID / message ID / event ID | 无法区分重复生成与重复渲染 | 从日志、opencode session 数据和前端事件中关联 |
| 使用的 provider、模型和 endpoint | 无法判断鉴权、限流、模型名、服务故障 | 检查脱敏后的配置与日志；不读取或展示 API Key |
| 重复消息刷新后是否仍存在 | 无法区分临时 UI 双渲染与持久化重复 | 用户复现并刷新/重启后观察，或检查消息数据库 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | 尚未追踪 |
| Trigger | 用户确认 PDF 导出后启动 Agent 执行 |
| Condition | provider/sidecar 返回错误；错误后的恢复或消息消费路径可能再次投递/渲染相同内容 |
| Related files | 待 CodeGraph 和日志证据定位 |

## Conclusion

**Confidence:** Low

已确认的只有 UI 层可见顺序：执行提示 → 引擎错误 → 相同执行提示。最有价值的调查方向是将日志中的 provider 错误、sidecar 会话恢复和前端消息事件按 ID/时间串联；当前不足以宣布根因，更不能据此直接修改重试或去重逻辑。

## Recommended Next Steps

### Fix direction

暂不定案。若后续确认是“同一事件重复消费”，应在稳定 message/event ID 边界做幂等去重并修正 listener 生命周期；若是“编排层重新执行”，应限定重试状态机，禁止在不可恢复错误后重放用户可见的执行承诺；若是“错误映射过度归一化”，应保留可诊断错误类别并向用户提供安全、明确的失败原因。

### Diagnostic

1. 只读检索 egosync.log 中错误文案及前后 100 行。
2. 用 CodeGraph 追踪错误文案、assistant 消息追加、SSE/Tauri event、session resume/retry 的调用链。
3. 对同一故障回合关联 session ID、message ID、event ID；若现有日志缺少这些字段，再提出最小诊断日志点位，获得用户确认后添加。
4. 检查刷新/重启后重复是否仍存在，以判定重复发生在持久层还是展示层。

## Reproduction Plan

1. 使用与故障相同的“UAT-全开角色”、provider 和模型。
2. 发起需要文件工具执行的请求并在确认步骤输入“ok”。
3. 人为触发可控的 provider unavailable/timeout（具体方法需在不影响真实配置的前提下另行设计）。
4. 记录 UI 文本、egosync.log、opencode session/message、前端事件 ID。
5. 验证：错误只出现一次；执行提示不被重复；不可恢复错误后任务状态明确终止；可恢复错误重试时使用同一任务 ID 且不重复追加已展示消息。

## Side Findings

- 用户最初要求“获取原 PDF 最后 5 页”，但后续方案称“用 reportlab 生成 PDF（保持原页结构）”。重新排版生成与直接裁切原 PDF 是两种不同语义；如果目标是保留原页结构，最简单且最可靠的实现应是直接提取第 65–69 页，而不是用 reportlab 重建内容。此项与当前引擎错误可能无关，但属于后续需求语义需要明确的独立风险。

## Follow-up: 2026-07-23

### New Evidence — Outcome 2: Evidence perimeter mapped

#### Diagnostic logs — Available

- `%APPDATA%\com.egosync.app\egosync.log` exists (2,904,097 bytes; modified 2026-07-23 17:45:22 +08:00).
- The affected `ok` request is identified as conversation `1e7c0c64-a7ca-4ec5-99d1-1a0348cf0023`, role `ed006308-0614-414e-bc59-b41997e43da3`, session `ses_071bbd84bffelkj95cQXLu7Rb0` at `2026-07-23T09:21:29.768649Z` (`egosync.log:17988-17991`).
- The sidecar failed two health checks and was declared unresponsive at `2026-07-23T09:21:51.704073Z`, then restarted (`egosync.log:18022-18028`).
- The event stream disconnected after restart and entered a 2-second reconnect loop (`egosync.log:18029-18038`).

#### Conversation database — Available

Read-only inspection of `%APPDATA%\com.egosync.desktop\conversations.db` found:

- User message `f864d636-92d6-4c46-a425-74871fab8e02`: content `ok`, created `2026-07-23T09:21:29Z`.
- Exactly one persisted assistant message follows it: `2d58b3a0-6621-42d2-a57b-9167d7ed0b0d`, created `2026-07-23T09:21:29Z`.
- That single row contains both the execution preface and the engine-error text in sequence; there is no second persisted assistant row containing the repeated preface.
- `message_process_events` contains zero rows for that assistant message, so tool/process-level event correlation is unavailable from this table.

#### Source code — Available, working tree modified

CodeGraph identified the principal source entry points:

- `egosync-app/src-tauri/src/services/agent_engine.rs:2302` — `try_run_opencode_stream`
- `egosync-app/src-tauri/src/services/agent_bridge.rs:239` — opencode HTTP bridge configuration/request path
- `egosync-app/src-tauri/src/commands/chat.rs:31` — per-conversation opencode session state

The relevant Rust and React files have uncommitted changes. Source tracing must distinguish current working-tree behavior from the installed build that produced the log.

#### Version control — Available

Recent commits are available; HEAD is `6192a38` dated 2026-07-23. The working tree contains many pre-existing modifications, including `agent_engine.rs`, `agent_bridge.rs`, `commands/chat.rs`, `ChatStream.tsx`, and associated tests. No files were changed by this investigation except this case report.

#### Tests/static analysis — Partial

Test source files are available, including `ChatStream.test.tsx`; no incident-specific test result or static-analysis artifact was found in the inventory scan.

#### Issue/UAT references — Partial

No issue dedicated to this exact incident was found. Existing ActionCard/UAT documents contain related generic engine-error wording but do not establish this incident's cause.

### Additional Confirmed Findings

#### Finding 3: The model-service error coincides with a confirmed sidecar liveness failure

**Evidence:** `egosync.log:18022-18028`, timestamps `2026-07-23T09:21:46.702897Z` through `2026-07-23T09:22:08.988405Z`.

The opencode process failed two health checks, was marked unresponsive, exited with code 1, and was restarted. Therefore the displayed service-unavailable error is not merely an unsupported UI guess; a real engine-side availability failure occurred during this request. The underlying reason the opencode process became unresponsive is not yet present in EgoSync's log.

#### Finding 4: The duplicate visible preface is not stored as two assistant messages

**Evidence:** `conversations.db`, conversation `1e7c0c64-a7ca-4ec5-99d1-1a0348cf0023`, assistant row `2d58b3a0-6621-42d2-a57b-9167d7ed0b0d`.

Only one assistant row exists after `ok`, and it contains `执行提示 + 错误提示`. This refutes the broad theory that two independent assistant messages were committed to the conversation database. The remaining duplication perimeter is transient streaming/render state, event replay during reconnect, or UI replacement of the provisional stream with the full persisted message.

#### Finding 5: Event-stream reconnection happened immediately after the sidecar restart

**Evidence:** `egosync.log:18029-18038`.

The global opencode event stream disconnected and repeatedly reconnected after the process restart. This provides a concrete mechanism capable of replaying or reapplying message state, but replay itself is not yet logged and remains unconfirmed.

### Updated Hypotheses

- **Hypothesis 1 — recovery replay:** remains **Open**, strengthened by confirmed event-stream reconnect.
- **Hypothesis 2 — duplicate frontend display:** remains **Open**, narrowed to provisional-stream/final-message reconciliation or duplicate event consumption; duplicate database insertion is **Refuted**.
- **Hypothesis 3 — error over-normalization:** remains **Open**. A real liveness failure is confirmed, but the crash/unresponsiveness trigger is missing.
- **Hypothesis 4 — orchestrator submitted a second turn:** substantially weakened. There is one `ok` chat request and one persisted assistant row; no second chat request is visible in the incident window.

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | Align incident log timeline | High | Done | Request, health failure, restart and reconnect identified |
| 2 | Trace provisional stream → persisted message → React rendering | High | Open | Highest-value next step |
| 3 | Trace sidecar health/restart interaction with active stream | High | Open | Determine how failure is converted into final assistant content |
| 4 | Locate opencode server stderr/crash detail | High | Open | Needed to explain why process exited code 1 |
| 5 | Inspect current-vs-installed-build difference | Medium | Open | Working tree is modified; installed binary may not match source |
| 6 | Incident-specific automated evidence | Medium | Missing | No matching test run/report found |

### Updated Conclusion

The evidence perimeter is no longer evidence-light. The engine error corresponds to a real opencode sidecar outage and restart. The duplicate display is a separate presentation/state-reconciliation problem until proven otherwise: the database contains only one assistant message, not two. The next investigation stage should trace the active stream's partial content, error finalization, reconnect behavior, and `ChatStream` reconciliation without modifying code.

### Source Trace — Outcome 3/4

#### Confirmed duplicate-display causal chain

1. During normal first-bubble streaming, backend text deltas are emitted with `messageId = None` and appended to `accumulated_text` (`egosync-app/src-tauri/src/services/agent_engine.rs:2643-2658`).
2. When the active opencode POST fails after partial text exists, backend appends `\n\n抱歉，Agent 引擎返回错误：...` only to the persisted accumulator (`agent_engine.rs:2913-2929`; equivalent late branch at `3042-3061`). It does **not** emit that newly appended error suffix as an `llm:stream` token.
3. Backend saves the full accumulator and emits only `done` (`agent_engine.rs:3145-3162`). Thus the database contains `执行提示 + 错误提示`, while the browser's stream bubble contains only `执行提示`.
4. On `done`, `ChatStream` converts the partial stream bubble into a synthetic completed message and appends it locally (`egosync-app/src/components/chat/ChatStream.tsx:59-80, 777-820`).
5. It then reloads history and merges it with local messages (`ChatStream.tsx:833-862`). Reconciliation recognizes a persisted/synthetic pair only when IDs correspond or contents are exactly equal (`ChatStream.tsx:126-152, 171-208`). Here the persisted content is a strict superset of the local content, so reconciliation preserves both.
6. Rendered result becomes: persisted `执行提示 + 错误提示`, followed by synthetic local `执行提示`. This exactly matches the reported sequence.

**Conclusion:** The duplicate visible text is a deterministic backend/frontend reconciliation defect, not two model responses and not duplicate database insertion.

#### Confirmed engine-error trigger chain

1. The watchdog runs every 5 seconds, gives each health request 2 seconds, and restarts after two consecutive failures (`egosync-app/src-tauri/src/services/sidecar.rs:13-18, 471-518`).
2. It has no active-request/in-flight guard before restart.
3. `restart()` calls `stop()`, and `stop()` immediately calls `child.kill()` (`sidecar.rs:313-335, 362-365`).
4. In the incident, the request started at `09:21:29Z`; health checks failed at `09:21:46Z` and `09:21:51Z`; the watchdog killed/restarted the process while the request was active (`egosync.log:17988-18031`).
5. The interrupted request error is categorized by substring matching; anything not recognized as 401/429/timeout/connect becomes `模型服务暂时不可用` (`agent_engine.rs:5194-5205`).

**Conclusion:** The immediate cause of the user-facing engine error was the watchdog restarting/killing opencode during the active request. Whether the two failed health checks represented a genuine opencode deadlock/crash or a false-positive health timeout under load remains unconfirmed because sidecar stderr/crash detail was not captured in the available log.

#### Refutation pass

- **Duplicate listener theory:** weakened. `useTauriEvent` registers one listener per effect and cleans it up on unmount/dependency change (`egosync-app/src/hooks/useTauriEvent.ts:4-30`). More importantly, the exact reported ordering is fully produced by the confirmed partial-vs-persisted merge mismatch without requiring duplicate event delivery.
- **Second orchestration turn:** refuted for this incident. There is one `ok` chat request and one assistant database row.
- **Event replay as necessary cause:** refuted. Reconnect occurred, but the UI merge defect reproduces the sequence even without replay. Reconnect may affect timing but is not required.

### Fix Direction Analysis — no implementation

#### A. Required consistency fix — preferred

When backend appends a user-visible error suffix to `accumulated_text`/`final_text`, emit the same suffix to `llm:stream` before `done`. Apply consistently to the `session.error`, immediate POST error-after-partial-output, and late POST error branches.

**Why preferred:** It restores the invariant `streamed visible content == persisted visible content`; existing exact-content reconciliation then works. It also prevents UI state from temporarily omitting the actual error.

**Verification:** add a test where stream sends `好的...`, backend final history contains `好的...\n\n抱歉...`, then `done`; assert only one assistant bubble and that it contains the error.

#### B. Defensive frontend reconciliation — secondary

On history merge, replace a synthetic completed assistant with a persisted assistant when they belong to the same preceding user turn and the synthetic content is a strict prefix of the persisted content produced during the same stream generation.

**Risk:** General prefix matching can swallow legitimate consecutive assistant messages. It should be a defense-in-depth measure, not the primary fix, unless a stable backend message ID is added.

#### C. Stable message identity — structurally strongest, broader change

Emit the persisted `assistant_message_id` for the normal first bubble rather than `messageId = None`. Then the synthetic ID can deterministically map to the persisted row without content heuristics.

**Trade-off:** Cleaner architecture but broader impact on delegation/multi-bubble semantics; requires wider regression coverage. Not the minimal first repair.

#### D. Watchdog safety fix — required independently

Do not kill the sidecar solely because two 2-second probes fail while active requests exist. Options, in preferred order:

1. Track in-flight opencode requests; during active work, use a longer threshold and require stronger evidence such as process exit or repeated failures over a larger window.
2. Separate liveness from readiness: a lightweight liveness endpoint should remain responsive during generation; readiness failure must not automatically kill active work.
3. Before forced restart, cancel/finalize active sessions explicitly and preserve a typed restart reason.

The current values (2-second timeout, 5-second interval, two failures) are too aggressive for a process performing long-running Agent/tool work unless the health endpoint is guaranteed non-blocking.

#### E. Error classification fix

Replace substring-only fallback with typed categories such as `SidecarRestarted`, `SidecarExited`, `ProviderUnavailable`, `Timeout`, and `Authentication`. A watchdog-terminated request should say the Agent engine restarted unexpectedly, not imply that the remote model provider was unavailable.

### Updated Confidence

- Duplicate display root cause: **High — Confirmed by database state plus deterministic source path.**
- Immediate engine-error trigger: **High — watchdog killed/restarted opencode during the request.**
- Why opencode failed health checks: **Low — missing opencode stderr/profile/crash evidence.**


## Final Conclusion — Outcome 5

**Status:** Concluded

### Final confidence

- **重复显示：High。** 后端错误追加未流式发送，前端以残缺本地消息完成并与完整历史消息合并；内容不相等导致现有去重失效。该链条由源码、数据库单行状态和用户可见顺序共同确认。
- **Agent 错误直接触发：High。** watchdog 在请求进行中连续两次健康检查失败后调用 `restart → stop → child.kill`，中断请求并触发错误收尾。
- **健康检查失败的底层原因：Low。** 现有证据无法区分真实死锁与繁忙状态下的健康探针假阴性。

### Why the remaining cause cannot be recovered from this incident

`SidecarManager` 确实把 stderr 读入 `stderr_buf`，但 stderr 行只按 `debug` 级别写日志（`egosync-app/src-tauri/src/services/sidecar.rs:269-295`）。该缓冲只在启动失败路径被读取（`sidecar.rs:415-437`）；watchdog 重启前没有输出缓冲，而下一次 `start()` 会先清空它（`sidecar.rs:260`）。因此本次进程被杀前的 stderr 已被覆盖，现有日志无法还原健康检查失败原因。`C:\Users\Admin\.local\share\opencode\log` 仅发现 2026-07-20 的旧日志，没有 2026-07-23 本次事故日志。

### Precise implementation scope — not executed

#### P0 — 修复重复显示

1. `egosync-app/src-tauri/src/services/agent_engine.rs`
   - 在三处错误追加路径中，将追加到 `accumulated_text`/`final_text` 的同一错误后缀同步通过 `emit_stream_token` 发给前端。
   - 保证先发错误 token，再发 `done`。
   - 不改变正常成功流、工具流或委派多气泡行为。

2. `egosync-app/src/components/chat/ChatStream.test.tsx`
   - 新增事故回归测试：先收到部分文本，最终历史内容为“部分文本 + 错误”，随后收到 `done`；断言只显示一个 assistant 气泡。
   - 断言错误提示可见且执行前缀只出现一次。

#### P1 — 防止 watchdog 中断正常活跃请求

1. 在 sidecar/agent bridge 边界维护 in-flight 请求计数或活跃 session 集合。
2. 活跃请求期间不因两次 2 秒探针失败立即 `child.kill()`；采用更长观察窗口或额外进程状态证据。
3. 重启必须记录结构化原因，并显式通知所有受影响 session。

#### P1 — 修正错误语义

将 watchdog 中断映射为 `SidecarRestartedDuringRequest` 一类的类型化错误；用户文案明确说明 Agent 引擎已重启且任务未完成，不再错误暗示远程模型 provider 不可用。

#### P2 — 补齐诊断能力

建议在获得用户确认后增加以下诊断点：

1. 每次健康检查失败：URL、耗时、HTTP 状态或网络错误类别、in-flight 数量。
2. watchdog 重启前：PID、运行时长、活跃 session、最近 stderr 尾部（限制长度并脱敏）。
3. `stop()`：记录终止原因是用户退出、手动重启、启动失败还是 watchdog。
4. Agent 请求失败收尾：conversation ID、session ID、assistant message ID、已流式字符数、最终持久化字符数、类型化错误类别。

这些诊断日志尚未添加，符合“先确认点位再执行”的项目规则。

### Verification plan

1. 单元测试覆盖部分流式输出后发生错误，确认没有重复气泡。
2. 模拟 sidecar 请求失败，确认流式可见内容与数据库最终内容完全相等。
3. 模拟健康探针在活跃请求期间连续超时，确认 watchdog 不立即杀死进程。
4. 模拟进程真实退出，确认 watchdog 仍能恢复，并向活跃请求返回准确的 `SidecarExited/Restarted` 错误。
5. UAT 使用原始 PDF 裁切任务复现，确认：只出现一次执行提示、错误只出现一次、失败时明确显示“任务未完成”、没有虚构输出文件路径。

### Recommended hand-off

最高价值下一步是使用 `bmad-quick-dev` 按 P0 范围实施最小修复并运行针对性测试；watchdog 与诊断改造应作为独立后续变更，避免把 UI 重复修复和生命周期策略调整混在同一补丁中。
