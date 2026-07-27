# Investigation: Think 思考耗时与执行过程宽度

## Hand-off Brief

1. **What happened.** 用户报告执行过程中的 `Think 思考了0秒` 未展示实际耗时，且执行过程面板宽度未与对话框对齐。
2. **Where the case stands.** 两个问题的前端根因已确认；本案只完成分析，未修改业务代码。
3. **What's needed next.** 先确认产品是否要求离线/历史消息也保留思考耗时；确认后按下述最小方案实现并补回归测试。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-25 |
| Status | Concluded |
| System | Windows; EgoSync workspace |
| Evidence sources | React/TypeScript source, Rust source, frontend tests, Git working tree |

## Problem Statement

用户希望 `Think 思考了xx秒` 显示真实思考时长，并要求“执行过程”的视觉宽度直接与对话框对齐。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| Frontend source | Available | 已定位实时计时、历史回退和布局类名 |
| Backend source | Available | 后端已在思考过程事件中保存 `elapsedSeconds` |
| Tests | Available | 已有实时计时测试，但初始状态和历史回退仍明确断言 `0秒` |
| Runtime logs/screenshot | Missing | 未提供具体运行截图、事件序列或浏览器 DOM 尺寸 |

## Confirmed Findings

### Finding 1: 历史/回退思考块硬编码为 0 秒

**Evidence:** `egosync-app/src/components/chat/ChatBubble.tsx:279-290` 在没有 thinking 类型过程事件时，用 `message.thinkingContent` 创建 fallback block，并固定设置 `elapsedSeconds: 0`。

**Detail:** 这条路径不读取消息已有耗时，也不读取后端过程事件的 `rawJson`。因此当过程事件尚未加载、加载失败或历史数据只提供 `thinkingContent` 时，标题必然显示 `Think 思考了0秒`。

### Finding 2: 后端具备真实耗时，但并非所有前端路径都使用它

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:856-894` 的 `flush_thinking_process_event` 使用 `started_at.elapsed().as_secs()`，并将 `elapsedSeconds` 写入持久化事件的 JSON；`egosync-app/src/components/chat/ChatStream.tsx:469-489` 从 `rawJson.elapsedSeconds` 读取该值。

**Detail:** 实时流式路径由 `ChatStream.tsx:617-657` 的计时器更新；后端持久化路径也有真实值。因此“系统完全没有计算秒数”不成立，问题是 UI 存在固定 0 的降级显示路径，以及历史/事件加载时序可能让该路径可见。

### Finding 3: 执行过程使用独立的自适应宽度，不是对话框宽度模型

**Evidence:** `egosync-app/src/components/chat/ChatBubble.tsx:223-248` 的 `ExecutionTrace` 根节点只有 `max-w-[85%]`，没有 `w-full` 或共享的宽度容器；对话框本体在 `ChatBubble.tsx:307-312` 另行使用 `max-w-[85%]`。父级 `ChatBubble.tsx:300-303` 使用 `items-start`，所以两个兄弟节点不会自动拉伸到同一宽度。

**Detail:** 两者虽然都出现 `max-w-[85%]`，但没有共享同一个实际宽度盒子；执行面板内部内容长度会影响其 auto width，导致视觉上与消息气泡不齐。

## Deduced Conclusions

### Deduction 1: 思考耗时问题是数据源覆盖不完整，不是计时算法本身缺失

**Based on:** Findings 1 and 2。

**Reasoning:** 后端和实时前端均能产生非零耗时；只有 `ChatBubble` 的 fallback 明确写死 `0`。当该 block 被优先渲染时，用户看到的 0 来自 fallback，而不是 `Date.now()`/`Instant` 的计算结果。

**Conclusion:** 修复应统一思考块的数据模型，让 fallback 也携带或解析耗时；不应重新设计计时器。

### Deduction 2: 宽度问题来自布局层级，不是百分比数值单独错误

**Based on:** Finding 3。

**Reasoning:** 修改单个 `85%` 数值无法保证对齐，因为执行区和对话框仍在不同的 auto-width 盒子中。

**Conclusion:** 修复应让执行区与对话气泡共享同一宽度约束/包装层，或明确让两者使用同一 `w-full max-w-[85%]` 规则。

## Hypothesized Paths

### Hypothesis 1: 用户看到的 0 秒来自事件加载时序

**Status:** Confirmed as a contributing path; exact runtime trigger remains unobserved.

**Theory:** 消息先以 `thinkingContent` 渲染，过程事件尚未返回或返回空数组，于是 fallback block 先显示 0。

**Supporting indicators:** `ChatStream.tsx:1051-1078` 异步加载历史过程事件；`ChatBubble.tsx:279-290` 在没有 thinking event 时立即使用固定 0 的 fallback。

**Would confirm:** 浏览器 DOM/React snapshot 显示消息先经过 fallback，随后过程事件到达后才替换。

**Would refute:** 实际运行中始终只有实时 stream block，而无 fallback block。

**Resolution:** 代码路径已确认，运行时具体时序未采集。

## Source Code Trace

| Element | Detail |
| --- | --- |
| Thinking start | `ChatStream.tsx:617-636` 创建 running thinking process event |
| Live elapsed update | `ChatStream.tsx:649-659` 每秒更新 `elapsedSeconds` |
| Thinking finish | `ChatStream.tsx:639-647` 计算最终秒数并标记 completed |
| Historical conversion | `ChatStream.tsx:469-489` 解析 `rawJson.elapsedSeconds` |
| Faulty fallback | `ChatBubble.tsx:279-290` 固定 `elapsedSeconds: 0` |
| Execution width | `ChatBubble.tsx:223-226`, `300-308` 两个独立宽度上下文 |

## Conclusion

**Confidence:** High

`Think 思考了0秒` 的确定性根因是思考内容 fallback block 将 `elapsedSeconds` 硬编码为 0；实时计时和后端持久化均已存在。执行过程未与对话框共享实际宽度容器，`items-start` 下的两个 auto-width 兄弟节点造成宽度不一致。

## Recommended Next Steps

### Fix direction

1. **耗时数据统一：** 为 `ChatMessage` 或其过程事件提供可持久化的 thinking elapsed 字段；fallback 优先读取该值，其次从已加载的 `MessageProcessEvent.rawJson` 读取，无法获得时才显示 0。
2. **避免时序闪烁：** 在历史过程事件加载完成前，保留 loading/unknown 状态或直接延后生成 fallback，避免先显示错误的确定值 `0秒`。
3. **宽度对齐：** 将 `ExecutionTrace` 与消息气泡放入同一宽度包装层，或统一使用 `w-full max-w-[85%]`；不要只改百分比。
4. **回归测试：** 增加 fallback 非零耗时测试、过程事件加载失败时的明确行为测试，以及执行区与对话框共享宽度类/DOM 结构测试。

### Diagnostic

若要先验证用户现场路径，可采集一次 `ChatBubble` 渲染时的 `executionTraceBlocks` 和 `message.thinkingContent`，确认是否存在 fallback 与持久化过程事件之间的时序窗口；无需增加长期诊断日志。

## Reproduction Plan

1. 发送一条会产生 thinking token 的消息，等待超过 1 秒后结束。
2. 在流式期间观察实时 `ExecutionTrace` 的秒数；再刷新或切换会话，观察历史消息的 `Think` 标题。
3. 模拟 `getMessageProcessEvents` 延迟、返回空数组或失败，检查是否回退到 `elapsedSeconds: 0`。
4. 对比 `ExecutionTrace` 外框与 assistant message bubble 的 `getBoundingClientRect().width`。

## Side Findings

- 当前测试 `ChatStream.test.tsx:183-198` 和 `1610-1637` 明确断言即时/回退场景为 `0秒`；这些断言会固化当前缺陷，需要随修复意图更新。
- 当前已有 `ChatStream.test.tsx:200-258` 覆盖实时计时和分段计时，说明计时器本身已有可复用行为，不宜重复实现。

## Follow-up: 2026-07-26 #2

### New Evidence

- Current worktree still contains the live thinking timer. `ChatStream.tsx:617-657` starts a `Date.now()` timer and updates it every second; `ChatStream.tsx:639-644` computes the completed duration. The frontend test suite passes 56/56 for `ChatStream.test.tsx`, but its assertions explicitly accept the initial `Think 思考了0秒` state (`ChatStream.test.tsx:183-196`, `200-254`).
- Current live/history conversion still coerces absent historical `elapsedSeconds` to `0` at `ChatStream.tsx:480-489`; the current fallback in `ChatBubble.tsx:281-292` is already `null` and therefore is not the remaining hard-coded-zero source on this worktree.
- The outer app root still declares `border border-slate-200 dark:border-slate-700` at `App.tsx:347-349`; there is no current App diff removing it. The window is deliberately frameless and transparent in `src-tauri/tauri.conf.json` (`decorations: false`, `transparent: true`), and `src-tauri/src/lib.rs:41-46` disables the Windows shadow.

### Additional Findings

1. **Think duration — confirmed remaining defect:** the UI renders any numeric duration, including the timer's initial `0`, as `Think 思考了0秒` (`ChatBubble.tsx:174-176`). Missing/invalid persisted duration is also converted to numeric `0` (`ChatStream.tsx:482-488`). The timer itself is present; the defect is display/fallback semantics and test intent, not absence of calculation.
2. **Outer frame — premise partially contradicted:** source has not removed the CSS border. If the missing line means the native gray perimeter/shadow, its removal is explained by the frameless/transparent configuration plus `set_shadow(false)`. If it means the CSS line, runtime computed style/geometry is still missing; likely candidates are a 1px low-contrast edge at the transparent viewport boundary or Windows/WebView2 edge clipping.

### Updated Hypotheses

- **H1 (Think, Confirmed):** initial/invalid duration is presented as a completed numeric value. Confirm by holding the stream in thinking state or returning an event with no `elapsedSeconds`; refute only if runtime DOM shows a nonzero completed value and the user still reports 0.
- **H2 (Outer frame, Medium):** the visible gray perimeter was native DWM shadow rather than the React border; `set_shadow(false)` removed it. Confirm with DevTools/computed style plus a build that temporarily restores only the shadow; do not restore native decorations because that would conflict with the custom `TitleBar`.
- **H3 (Outer frame, Open):** the React border is present but visually clipped/blended at the transparent window edge. Confirm by inspecting the root element's computed border and `getBoundingClientRect()` at normal and maximized sizes.

### Updated Conclusion

The two symptoms have different mechanisms. For Think, retain the existing timer, stop treating active/missing duration as a completed `0`, and add a regression test for completed nonzero duration plus unknown historical duration. For the frame, do not search for a removed App border: it is still present. First identify whether the expected line is the CSS border or the native shadow; the safer fix direction is a deterministic inset frame (for example an inset ring/box-shadow) while keeping the custom frameless window, with `set_shadow(false)` changed only if the desired effect is specifically the native perimeter shadow.

## Follow-up: 2026-07-26 #3 — 白色背景下外框轮廓变淡的提交定位

### Stronghold Evidence

- 提交 `d07e776de82096b20a1e74171c4fc7d5a649ed5f` 于 `2026-06-21 20:21:02 +0800`，提交说明为“feat: 自定义标题栏 + L型布局 + 无边框窗口圆角”。
- 在该提交之前，`egosync-app/src-tauri/tauri.conf.json` 的窗口配置只有 `fullscreen: false`，未关闭系统装饰，也未开启透明窗口；`App.tsx` 根节点使用 `bg-[#F8F9FA]`，没有根节点 CSS border，但窗口仍由系统原生外框/阴影提供外部轮廓。
- `d07e776` 同时完成了四项影响外框观感的修改：
  1. `tauri.conf.json` 增加 `decorations: false`；
  2. `tauri.conf.json` 增加 `transparent: true`；
  3. `src/index.css` 将 `body`、`html` 背景改为 `transparent`；
  4. `src-tauri/src/lib.rs` 在 Windows setup 中调用 `window.set_shadow(false)`，注释明确写着“移除 DWM 边框”。
- 同一提交虽增加了 React 根节点 `border border-slate-200 dark:border-slate-700 rounded-lg`，但这是 1px 的浅色 CSS 边框，不是原先的系统外框/阴影等价物；`App.tsx:348` 当前仍然保留该边框。

### Timeline

| 时间 | 提交 | 变化 | 对轮廓的影响 |
| --- | --- | --- | --- |
| 2026-06-20 之前 | `d07e776` 的父提交 | 普通窗口配置；未设 `decorations:false`、`transparent:true`、`set_shadow(false)` | 系统原生窗口边框/阴影存在 |
| 2026-06-21 20:21 | `d07e776` | 自定义标题栏、无边框透明窗口、关闭 DWM 阴影；新增浅色 1px CSS border | 原生明显轮廓被移除，改由低对比度 CSS 边框承担 |
| 2026-07-03 20:28 | `9609922` | 增加 `visible:false`，启动后再 show；保持 `decorations:false`、`transparent:true` | 影响启动显示时机，不是轮廓变淡的首次来源 |
| 后续至当前 HEAD | 多个业务提交 | 未再改变上述外框关键配置或根节点边框 | 未发现后续删除边框的提交 |

### Findings

1. **Confirmed：轮廓能力的关键改变发生在 `d07e776`，不是最近某个业务提交删除了 CSS 边框。**
2. **Confirmed：原生轮廓被显式移除。** `decorations:false` 取消系统标题栏/边框；`set_shadow(false)` 又关闭 Windows DWM 阴影；`transparent:true` 和 HTML/body 透明使窗口边缘依赖前端自身绘制。
3. **Deduced：白色背景下“不清楚”来自视觉替代不足。** 原生边框/阴影通常比当前 `border-slate-200` 的 1px 线更明显；当前根背景是 `#F1F3F5`，边框又位于透明窗口最外缘，因此在白色背景上容易混合或被 WebView2 边界裁掉。
4. **Open：尚不能仅凭静态代码区分用户看到的“原来轮廓”究竟是 DWM 阴影、系统边框，还是 CSS border。** 当前源代码只能确认 CSS border 仍在；需要运行时 computed style/截图或临时对照构建确认是否发生边缘裁剪。

### Not Yet Executed

- 未修改业务代码；
- 未恢复系统装饰或 DWM shadow；
- 未执行运行时视觉对照构建。

### Safe Fix Direction (analysis only)

保留自定义无边框标题栏，不直接恢复 `decorations:true`。若目标是稳定的四边灰色轮廓，优先把根节点边框改为在窗口内容内部绘制的 inset 描边（如 `ring-inset` 或 inset `box-shadow`），再用运行时截图验证；只有确认用户要的是 Windows 原生阴影时，才单独评估恢复 `set_shadow(true)`。

## Implementation Follow-up: 2026-07-26 #4

### Applied Changes

- `egosync-app/src/components/chat/ChatStream.tsx`: thinking duration starts as `null` rather than `0`; persisted events with missing `elapsedSeconds` remain `null`; a completed thinking segment reports at least 1 second so a real thinking token cannot finish as `Think 思考了0秒`.
- `egosync-app/src/components/chat/ChatBubble.tsx`: active thinking with no elapsed value (or still at zero) displays `Think 思考中`; active segments with a measured positive duration continue to show the live seconds; completed unknown historical durations display `Think 思考时长未知`.
- `egosync-app/src/App.tsx`: replaced the outer 1px CSS border with `ring-1 ring-inset ring-slate-300 dark:ring-slate-600`, keeping the frameless transparent window while drawing the outline inside the content viewport.
- `egosync-app/src/components/chat/ChatStream.test.tsx`: updated expectations for active thinking, short completed thinking, and unknown historical duration.

### Verification

- Targeted chat tests: 2 files, 74/74 passed.
- Full frontend suite: 42 files, 432/432 passed.
- Production build (`tsc && vite build`): passed.
- Existing unrelated worktree changes were preserved; no reset or cleanup was performed.

## Follow-up: 2026-07-26 #5 — Think 完成态归零、首个 Think 延迟与外框复核

### Scope

本轮仅基于当前工作树和 Git 历史分析，未修改业务代码、未重置工作树、未执行修复。

### Confirmed Findings

1. **Think 流式显示与历史显示使用了两套事件来源。**
   - 前端收到首个 `payload.thinking` 后，在 `ChatStream.tsx:617-636` 创建 synthetic running thinking event；每秒在 `ChatStream.tsx:649-657` 更新耗时；离开 thinking 阶段时在 `ChatStream.tsx:639-647` 以 `Math.max(1, floor(...))` 写入 completed synthetic event。
   - 完成时，`ChatStream.tsx:895-915` 将该 synthetic event 暂存到本地完成消息的 process-event 映射；随后 `ChatStream.tsx:932-962` 异步重新读取会话历史并用 `mergeHistoryWithLocalMessages()` 替换本地 `__completed__...` 消息。
   - 历史消息替换为真实 ID 后，`ChatStream.tsx:1067-1078` 会触发 `getMessageProcessEvents(realMessageId)`；`ChatStream.tsx:469-489` 再从持久化事件 `rawJson.elapsedSeconds` 渲染标题。由此，完成后的显示可能不再使用实时 synthetic 的 1 秒，而改用后端持久化值。

2. **后端持久化 thinking 时长仍允许写入 0。**
   - `agent_engine.rs:856-897` 的 `flush_thinking_process_event()` 使用 `started_at.elapsed().as_secs()`，没有最小值保护。
   - 该函数在 `agent_engine.rs:2677-2687`、`2752-2763` 等“思考切换到可见文本”路径被调用；思考片段短于 1 秒时，持久化 `elapsedSeconds` 必然为 `0`。
   - 函数只持久化、不发第二个 live process event（`agent_engine.rs:879-894`），所以前端实时期间看到的 synthetic 1 秒与完成后重新加载的持久化 0 秒可以先后出现。

3. **当前 `ChatBubble` 会把历史持久化的数值 0 当成完成态真实耗时。**
   - `ChatBubble.tsx:174-178` 对非 active block 只要 `elapsedSeconds` 是 number，就显示 `Think 思考了${elapsedSeconds}秒`；因此后端事件的 0 会直接显示为完成态 `0秒`。
   - `ChatStream.tsx:480-488` 对历史事件缺失/非法时长保留 `null`，这条路径本身不会生成数值 0；本轮主因是后端确实可能持久化 0，以及完成后的历史重载覆盖实时事件。

4. **首个 Think 内容在前端没有占位显示。**
   - `ChatStream.tsx:1109-1135` 发送 query 时只设置 streaming/input lock、清空旧过程状态，没有创建空的 thinking block。
   - `ChatStream.tsx:842-857` 只有收到第一条 `payload.thinking === true` 的 Tauri 事件后才创建过程块。因此从 query 发出到后端首个 thinking event 到达期间，UI 不显示 Think 内容；这部分等待会被用户感知为“模型没有开始思考”。

5. **首个 thinking event 之前确实存在多段后端前置等待；具体耗时需要运行时日志才能拆分。**
   - `chat.rs:381-441` 的 Tauri command 先 spawn 后台 `run_stream`，所以前端 `sendMessage()` 返回并不等于模型已经开始产出 token。
   - `agent_engine.rs:2364-2413` 先检查禁用 Skill/动态配置及 runtime refresh；`2414-2457` 解析工作目录、MCP scope、同步 MCP 配置，并在无缓存 session 时调用 `create_session()`；`2461-2479` 构建角色/管家动态 prompt；`2482-2502` 注册委派会话并订阅事件；`2517-2544` 还可能先持久化显式 Skill 过程事件。只有 `agent_engine.rs:2551-2560` 启动 `send_message()`/`send_command()` 后，sidecar 才真正收到 prompt。
   - 事件订阅发生在发送前（`agent_engine.rs:2501-2502`），当前代码没有证据表明因订阅晚导致首 token 丢失。
   - `agent_bridge.rs:131-160` 的 `send_message()` 是同步等待 opencode HTTP response body 的调用，但它与事件消费并行运行；因此“首 Think 显示延迟”不能仅归因于等待完整 HTTP response。仍需区分前置初始化、opencode/provider 首 token、或事件路由延迟。

6. **应用外框的当前 `ring` 已生成，但其绘制位置容易被子层背景覆盖。**
   - `App.tsx:348-350` 在根节点上使用 `ring-1 ring-inset ring-slate-300 ... rounded-lg overflow-hidden`，而 `TitleBar.tsx:25-28`、`Sidebar.tsx:59-63` 及 App 内部主布局均绘制覆盖整个可用区域的实色背景。
   - 已检查生产 CSS：`dist/assets/index-DS-UeYM_.css` 包含 `.ring-1`、`.ring-inset` 和 `.ring-slate-300`，所以不是 Tailwind 未生成类。
   - CSS ring 是根元素的 inset box-shadow；在当前子元素铺满根节点且带背景的 DOM 层级中，父层 ring 位于子层背景之后，四边可能被覆盖，因此“改成 ring 后仍看不到”有确定的布局/绘制机制依据。

7. **上次 ChatInput 外框修改不是 App 外框消失的直接来源。**
   - 提交 `27a743f`（2026-07-25）只修改 `ChatInput.tsx` 的 input class：将 `focus:ring-2` 改为 `focus:border-transparent focus:ring-1 ... focus-visible:!outline-none`；提交文件列表不包含 `App.tsx`、`index.css`、Tauri 窗口配置或 `lib.rs`。
   - 当前全局 focus 规则在 `index.css:66-73`，作用对象是 `:focus`/`:focus-visible`，不匹配 App 根节点的普通 `ring`，没有证据表明它会清除 App 外框。

8. **更早的真正外框能力变化发生在 `d07e776`（2026-06-21 20:21:02 +0800）。**
   - 该提交把窗口改为 `decorations:false`、`transparent:true`（`src-tauri/tauri.conf.json:22-25`），把 `html/body` 背景改为透明（`src/index.css:57-64`），并在 Windows setup 调用 `window.set_shadow(false)`（`src-tauri/src/lib.rs:41-46`）。
   - 同一提交新增的 React 外框只是浅色 1px CSS border；之后本工作树才将它替换为 inset ring（`App.tsx:348`）。因此当前缺失的“原来更明显的灰色轮廓”首先是原生 DWM 边框/阴影被主动移除后的结果，不是 ChatInput 提交删除了它。

### Deduced Conclusions

1. **Think 的“1 秒 → 0 秒”是完成后数据源切换造成的覆盖问题，根因置信度：High。**
   证据链为：实时 synthetic event 受前端最小值保护 → 完成后 history reload 换成真实 message ID → process-event reload 读取后端事件 → 后端 `as_secs()` 对短片段写入 0 → `ChatBubble` 将数值 0 当完成耗时显示。尚未采集用户现场事件序列，因此“每一次复现是否都走该 reload 时序”仍是运行时层面的待验证项。

2. **首个 Think 延迟至少有一个 Confirmed 前端原因，整体首 token 归因目前为 Medium。**
   确定的是：前端不会在 query 发出时展示占位 Think；另外后端在首 token 前有明确的 DB/MCP/session/prompt/注册等前置步骤。尚不能仅凭静态代码判断用户看到的几秒主要来自哪一段，也不能把它直接归咎于模型 provider。

3. **外框不可见的当前直接原因更可能是 ring 被覆盖，历史根因是无边框透明窗口移除了原生轮廓，置信度：Medium-High。**
   `ring` 类确实存在且已进入生产 CSS；当前 DOM 子层背景覆盖根节点 inset ring 的条件成立。若用户指的是比 CSS 线更宽的原生阴影，则其消失点已由 `d07e776` 确认。

### Recommended Fix Plan (analysis only)

1. **Think 时长统一为单一可信值。**
   - 后端持久化时不要让短 thinking segment 写入完成态 0；采用与产品语义一致的最小值策略，或持久化毫秒/更高精度后由前端统一取整。前端实时与历史都从同一语义字段渲染。
   - 完成后的 history/process-event reload 应保留实时 synthetic 值，或在真实事件加载时合并同一 message 的本地完成事件，避免较差的持久化值覆盖已测得的更精确值。
   - 测试至少覆盖：实时跨 1 秒后完成、短于 1 秒完成、完成后历史重载、多个 thinking segment、历史缺失耗时。

2. **降低首 Think 感知延迟并定位真实耗时。**
   - 发送时立即创建 `Think 思考中` 的前端占位块；收到首个 thinking event 后填充内容并继续计时。这样只解决可见性，不伪造首 token。
   - 在 `chat_send_message`/`run_stream`/`try_run_opencode_stream` 添加阶段时间戳诊断（query accepted、session resolved、prompt build done、subscription done、POST start、first event received、first thinking emitted）；确认后移除临时诊断日志。
   - 根据时间戳再决定优化：缓存/预同步 MCP scope、复用 session、并行不相互依赖的 prompt/config 查询、或检查 opencode/provider 的首 token time-to-first-byte。不要在无数据时猜测 provider 或 event router。

3. **外框采用不会被子背景覆盖的结构。**
   - 保留 `decorations:false` 和自定义 TitleBar；不要直接恢复系统 decorations。
   - 将描边放在最上层/独立绝对定位 overlay，或将根节点 padding/inset 与内容区域明确分离；确保四边由同一层绘制且不被 TitleBar/sidebar/main 背景覆盖。
   - 若产品要的是原生阴影而非内容内灰线，再单独评估恢复 `set_shadow(true)`；这会改变窗口边缘行为，不能和 CSS 线方案混为一谈。
   - 用白色背景、深色背景、最大化和普通窗口四种状态验收。

### Missing Evidence / Diagnostic Plan

| Gap | Why it matters | Minimal evidence |
| --- | --- | --- |
| 完成后实际返回的 thinking events 数量、顺序与 `elapsedSeconds` | 确认是否为单个 0 秒事件覆盖 synthetic 1 秒，或多个片段中的某个 0 秒 | 临时记录 message ID、event ID、status、elapsedSeconds；复现后移除 |
| 首个 `llm:stream` 的阶段耗时 | 区分前置初始化、sidecar/provider 首 token 与事件消费延迟 | 按上列阶段添加 monotonic timestamp；复现一轮 |
| 根节点与子层的运行时 computed styles/geometry | 确认 ring 是否被子背景覆盖、是否发生 WebView 边界裁剪 | 记录 root/TitleBar/main `getBoundingClientRect()`、`box-shadow`、`background-color`，并截取普通/最大化窗口 |
| 用户所说“原来轮廓”究竟是 CSS 线还是 Windows DWM shadow | 决定 CSS overlay 与 `set_shadow(true)` 的方案 | 仅做临时对照，不直接修改正式配置 |

### Status

**Active — diagnosis is sufficient for a targeted fix, but runtime timing/visual evidence remains required before attributing the full first-token delay or native-vs-CSS frame appearance.**
