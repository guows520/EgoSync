---
title: 'chat/agent_engine 域迁移与 ChatSessionRegistry 抽取（Story 15.3）'
type: 'refactor'
created: '2026-09-18'
status: 'done' # 2026-09-18 步骤 4 评审三层全过（4 补丁已应用 / 5 项 defer 入册 / 3 项驳回，分诊日志在案）；特征测试前后双绿、test:all 全绿（壳 107+3+37 / engine 741）、build/grep/冒烟/e2e 归因全收口
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'aba1b8c'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-15-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 15.2 后 chat 域是引擎无头化最后的深耦合域：agent_engine.rs（8553 行、22 处 `tauri::`、9 处 emit）与 chat 命令层事件面、六组会话状态全部留壳；同会话并发 busy 语义现状零测试覆盖——「桌面全绿」防线对它是空集。delegate_bridge 因深依赖 agent_engine 于 15.2 裁决留壳，等本故事一并收编。

**Approach:** 特征测试**先行**（busy 互斥三断言在抽取前代码上落绿）→ engine events.rs 补 chat 域事件常量 → 六组状态抽为 engine `registry.rs` ChatSessionRegistry（一次性接线，不留壳内桥接）→ agent_engine + delegate_bridge 整体平移进 engine（emit 全走 EventBus + 常量；managed state/密钥/路径改参数注入）→ 特征测试重跑锁粒度守恒 + 桌面全量回归收口。

## Boundaries & Constraints

**Always:** 平移逐字节等价，允许改动仅四类（import 路径调整 / 接缝调用替换 / emit 机械改写「事件名→常量 + 强类型→serde_json::to_value」/ 常量与函数搬移换引用不换值）+ 测试使能豁免（**OQ1 裁决 A**：chat_send_message 签名泛型化 `<R: tauri::Runtime>` + dev-dep 启用 tauri "test" feature，与特征测试同批落地——测试基建非抽取，发布物零影响；spike 实证失败即回报重裁决）；busy 经 **Ok 通道**返回（chat.rs:298-312 亲证：锁内 contains→insert busy 消息→`Ok(busy_msg)`），companion 以 `msg.role != "user"`（companion_dispatch:516-520）探测 busy 的隐式协议冻结，busy 禁止改 Err 通道；busy 分支**持锁跨 await** 是恰一条 busy 的根，禁止拆成 check-then-insert 两段；特征测试在抽取前代码上全绿是动 Registry 的前置；锁粒度守恒五不变量：①busy check+insert 与完成 remove（chat.rs:437-439）同锁 ②StreamingState 忙分支唯一跨 await 持锁点 ③OpencodeSessions Arc 身份共享（mcp 整表 clear vs agent_engine 按会话写）④McpScopeLock 串行化「MCP 同步+会话创建」vs「mcp 命令+sidecar 重启」⑤Memory replace 锁内 cancel 旧 token；`llm:stream` payload 逐字节一致（StreamPayload 已在 engine models/chat.rs:60-79 camelCase）；六组状态一次性迁入（Arc 组合、无 Tauri 类型）；events.rs 常量是唯一发射源；companion_dispatch:708 / companion_snapshot:803 监听经 TauriEventBus 转发仍可达（转发链路无感）；CI `grep -rn 'tauri::' crates/egosync-engine/src/` 零命中（含注释——15.2 event_router 注释改写先例）；收口 = `npm run test:all` + `npm run build` 全绿 + **e2e 沿用 15.2 裁决 A 口径（OQ2 裁决 A：执行一次 + 逐项归因既有缺陷，证据链入 Implementation Notes；busy/stop 语义由本故事特征测试在 Rust 层钉死，不新增 e2e spec）** + 启动冒烟（release xvfb ≥30s 无 reactor panic）。

**Never:** 不建 server binary（15.4）；不做前端 transport / useEngineEvent / vitest mock 迁移（15.5；架构文档 Gap #3「15.3 完成」为陈旧编号，以 epics.md 为准）；不迁四个 companion_* service（listen 是桌面宿主第四类耦合，架构裁决留壳）；不迁 memory_pipeline / task_decomposition（15.2 Never「留 15.3」措辞与 epics AC 未点名冲突，以 epics 为准留壳、仅改回引路径；引擎化随 15.4 server 闭包裁决——见 Design Notes）；不收编 commands/app.rs perf-test 注入器（:155-213 `#[cfg(feature="perf-test")]` 壳侧测试设施，仅字面量改引常量）；不收编纯壳侧其余事件名（role:created/updated、task:created 等非本域发射面，留 15.5）；不顺手重构（锁粒度细化、三份 provider 构造合并、TestSecretStore 去重、WRITE_SIGNAL_EVENTS 提取器解析常量——deferred-work 在案）；不引入第五接缝；不改 WRITE_SIGNAL_EVENTS 25 名清单本体（仅按 15.2 补丁 1 先例追加 contains 钉子）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 同会话并发双发 | 同 conversation_id 两条 chat_send_message 并发，首条在途（mock opencode 挂起） | 恰一次 LLM 调用（mock 服务请求计数=1）、恰一条 busy 落库（assistant/「我还在想上一个问题，请稍等片刻...」）、busy 经 Ok(Message) 返回 | 拒绝方零报错、不插 user 消息、不建 token、不 spawn |
| 异会话并发 | 两个不同 conversation_id 并发 | 互不阻塞、各自进入流式（busy 零命中）——锁粒度非全局串行的证据 | 各自独立错误处理 |
| llm:stream 全链路 | 正常流式回复 | 事件名/payload JSON 逐字节同现状；companion_dispatch/companion_snapshot 监听照收 | emit 失败按站点现状（`let _ =` 忽略 / :5104 warn） |
| run_stream Err 兜底 | 流任务失败 | chat.rs:421-434 兜底 done 帧照发（防前端 isInputLocked 卡死） | 经 bus + LLM_STREAM_EVENT |
| 应用启动 | lib.rs setup | 单 manage(ChatSessionRegistry)、各入口正常派生、无 panic | 冒烟无 "there is no reactor running" |

</frozen-after-approval>

## Code Map

**调查结论（三路子代理 + 主会话亲证，2026-09-18，基线 aba1b8c）：**

- **agent_engine 现状**（`egosync-app/src-tauri/src/services/agent_engine.rs`，8553 行）：tauri:: 22 处 = use ×2（:4、:42 局部）+ AppHandle 参数 19（:3280 owned run_stream）+ 泛型 1（:4022 `handle_tool_part<R: tauri::Runtime>`）；**零** async_runtime::spawn（spawn 全 tokio::：:2498/:3439/:3687/:4005/:4548/:5284）、零 listen、零窗口 API。Manager：path() :43、try_state ×6（:2316/:2327/:2343/:2365 AgentConfigService、:2358 OpencodeMcpScopeLock、:3267 SidecarManager）。emit 9 处：`llm:stream` ×4（:224/:297/:819/:953，四个 helper：emit_stream_token/emit_tool_status/emit_process_event/emit_stream_done，全 StreamPayload 强类型）、`role:proposed` ×3（:3797/:4118/:5104，RoleProposedPayload）、`role:delegated` :4162、`task:tool-action` :4188（后二 json!() 动态）。StreamPayload/STREAM_PHASE_*/RoleProposedPayload 已在 engine `models/chat.rs:54-83`——改写只剩常量名 + to_value。emit 错误语义：8 处 `let _ =`、:5104 match warn。
- **agent_engine 真壳依赖仅 3 组**：① `crate::commands::chat::{OpencodeSessionState`（:16 use、:3292 run_stream 参数 `Arc<Mutex<HashMap<String, OpencodeSessionState>>>`）、`OpencodeMcpScopeLock}`（:2358 try_state→state.0.clone()，None 时无锁降级）② `KeyringSecretStore`（:31 use；:3421/:4549/:4932 即席 new()）③ `DelegateBridge`（:2265/:3295 参数 + take_runtime_refresh :2319/request_runtime_refresh :2333/register_role_session :2425/unregister_session :2562/:2811/:2983/:3046/:3112/clone :3323）。其余 crate::{db,models,error,llm} 与 services::{llm_config,role_context,butler_config,role_config,mcp_server,agent_config,agent_bridge,event_router,sidecar,task_classifier} 全是 engine 回引（壳 lib.rs:15-17 pub use + services/mod.rs:5-14）——迁移仅改路径。`:25 pub use resolve_default_provider` re-export 断链：壳调用点 commands/chat.rs:1021、memory_pipeline.rs:40 改引 `egosync_engine::services::llm_config`。
- **路径接缝**：resolve_opencode_project_dir :41-54（app_data_dir + dirs::home_dir 兜底）、resolve_requested_working_directory :58-74（空回落前者）→ 注入 project_dir。
- **被依赖面**（agent_engine 外部调用全集）：commands/chat.rs:13 use、:396 run_stream、:1021；memory_pipeline.rs:11/:40；delegate_bridge.rs:703 `execute_delegate_to_role_with_source`（agent_engine:4810，纯 db+LLM 无 app_handle；内部 :4932 provider、:4947 build_role_messages、:4758→:4709 collect_local_stream 本地 drain 不发 llm:stream）/:729 `append_delegation_metadata`（:4275 纯 db）。
- **agent_engine 测试**：:5336-8553 约 3217 行 114 测试，in-memory sqlite（:6485 setup_test_main_pool + include_str! 直引 engine migrations 002/004-009/011-015/020/027——迁移后相对路径 4 级→2 级）；ScriptedDelegateProvider :5348-5365 为 mock LlmProvider 先例。
- **delegate_bridge 现状**（壳，1223 行）：:17 use{AppHandle,Emitter,Manager}、:39 Option<AppHandle> 字段、:218 try_state、:289 path().app_data_dir()（skills_root）、:411 emit SKILL_REGISTRY_UPDATED（已常量）、:509 `tauri::async_runtime::spawn`（任务体内→tokio::spawn 安全，架构决策 #3）、:526 emit TASK_CLASSIFIED（已常量）。
- **六组状态**（commands/chat.rs:15-39，全 tokio::sync::Mutex、键全 conversation_id、构造仅 lib.rs:73-78 六 manage `::default()`）：OpencodeMcpScopeLock(pub Arc<Mutex<()>>) :15、StreamingState(pub Arc<Mutex<HashSet<String>>>) :18、CancelTokens(pub Arc<Mutex<HashMap<String,CancellationToken>>>) :21、OpencodeSessionState（值对象）:24、OpencodeSessions :30、OnboardingConversations(pub Arc<Mutex<HashMap<String,u8>>>) :35、MemoryExtractionState(:38 +Clone)。engine 已有 tokio-util 0.7。
- **busy 互斥**（chat.rs:298-312 亲证）：:299 lock → :300 contains → busy 分支**持锁跨 await** insert_message（:301-308）→ `Ok(busy_msg)` :309；:311 insert。busy 不发 message:saved、不插 user 消息、不建 token、不 spawn → 恰一次 LLM。spawn :396 `tokio::spawn`（#[tokio::test] 兼容）；清理 :437-439 streaming.remove / :441-444 token remove（闭包内 app_handle.state 取）/ :446-451 schedule_memory_extraction；返回 `Ok(user_msg)` :453。命令签名 :221-230（State 注入 StreamingState/Onboarding + app_handle.state 混合取 MemoryExtraction :294 / CancelTokens :351 / OpencodeSessions :360 / AgentBridge :361 / EventRouter :365 / DelegateBridge :370）。
- **消费方全景**：chat.rs（chat_send_message :221、chat_stop_streaming :557（CancelTokens:562/OpencodeSessions:566）、chat_delete_conversation :584（5 组：:591/:595/:600/:605/:610）、chat_new_conversation :621（MemoryExtraction:667））；mcp.rs 9 命令全注入 McpScopeLock+OpencodeSessions（:35/:49/:66/:81/:109/:125/:142/:160/:178，首行 lock guard 跨整命令；refresh_opencode_runtime :196-204 整表 clear）；skill.rs:167/:211（OpencodeSessions；:202/:248 复用 refresh）；app.rs:45-56（McpScopeLock）；companion_dispatch.rs:504-513 直调 chat_send_message 注入 StreamingState+Onboarding（**axum handler 薄调用的现成范本**）；agent_engine :3292 Arc 参数 + :2358 try_state。
- **chat 事件面**：emit_chat_event chat.rs:46-50（app_handle.emit 直发、失败仅 warn）——message:saved :344（Message）、conversation:deleted :617（裸 String）、conversation:created :636/:679（Conversation）、conversation:title-updated :1075/:1087、llm:stream 兜底 done 帧 :421-434。监听：companion_snapshot WRITE_SIGNAL_EVENTS :739-772（:763/:767/:768）+ llm:stream 特判 :803（done=true 才转发 :778-783）；前端 ChatStream.tsx:1012（llm:stream）/ :1020（conversation:title-updated）、OnboardingView.tsx:49；companion_dispatch.rs:708 listen。**契约锁失明风险**：companion_snapshot:890-924 include_str! 字面量扫描（EXEMPT=["llm:stream"]）——chat.rs 改常量后对三事件失明，须按 15.2 补丁 1 先例追加 `WRITE_SIGNAL_EVENTS.contains(常量)` 钉子。
- **app.rs perf-test 注入器**：:155-213（:157 feature 门控）:177/:197 llm:stream 字面量——留壳，仅改引常量。
- **测试基建**：壳 dev-deps 已有 tempfile/filetime/relay-server/axum 0.8（Cargo.toml:47-53）；tauri 2.11.2 "test" feature（Cargo.toml:123）→ `tauri::test::{mock_builder :165, mock_app :184}`（lib.rs:1099 门控；MockRuntime 无窗口、headless 可用）；命令宏结构支持泛型命令（wrapper 原样再发射 ItemFn + macro_rules per-runtime 实例化，宏源码核验；**首任务 spike 实证**）。AgentBridge（engine services/agent_bridge.rs:9-26，Clone）reqwest **无超时**——挂起 TcpListener 确定性 hold（首调 POST /session :36-59）。tests/test_app.rs 占位（注释指名 test_chat.rs 落位）；test_companion.rs:749-754 axum+TcpListener 先例、:1738 gated 并发先例。chat.rs tests :683-974 setup_conversation_pool（:688-712）/setup_main_pool（:729）可复用。
- **双 TauriEventBus**：lib.rs:72 manage + :391-393 scheduler `Arc::new` 独立构造——deferred-work 指名随 15.3 统一（TauriEventBus 加 Clone，:391 改 `app.state::<TauriEventBus>().inner().clone()`）。
- **CI**：ci.yml:274-293 engine 零 tauri:: 断言（三态 rc 守卫）——两文件迁入后注释亦不得含 tauri::。

## Tasks & Acceptance

**Execution:**

- [x] `egosync-app/src-tauri/tests/test_chat_busy_mutex.rs` + `Cargo.toml` + `commands/chat.rs` -- 特征测试先行（OQ1 裁决 A 落地）：dev-dep 加 `tauri = { version = "2", default-features = false, features = ["test"] }`；chat_send_message 签名泛型化（spike 实证，失败回报重裁决）；mock_app manage 全状态（双池 in-memory sqlite、六组状态、AgentBridge→挂起 TcpListener、EventRouter/DelegateBridge/AgentConfigService/SidecarManager 按 run_stream 依赖清单）+ 门控并发双发；断言：恰一次 LLM（mock 请求计数=1）、恰一条 busy 落库、busy 经 Ok(Message{role:assistant}) 返回 + 异会话并发对照；现状代码全绿 -- 护栏先行 **【主会话补齐】实现子代理死于磁盘满 ENOSPC、未留存抽取前执行证据——主会话以基线还原手术（git checkout aba1b8c + 仅施加 OQ1 测试使能补丁：Cargo.toml dev-dep / chat_send_message / run_stream 及 emit 辅助族泛型化）在抽取前代码上补跑：2/2 全绿（same_conversation 恰一次 LLM+恰一条 busy+Ok 通道；distinct_conversation 互不阻塞）；迁移后重跑同断言 2/2 全绿——前后一致的执行级证据闭环（详见 Implementation Notes）**
- [x] `crates/egosync-engine/src/events.rs` -- +7 常量（MESSAGE_SAVED/CONVERSATION_CREATED/CONVERSATION_DELETED/CONVERSATION_TITLE_UPDATED/ROLE_PROPOSED/ROLE_DELEGATED/TASK_TOOL_ACTION）+ 值钉 -- 发射源补全
- [x] `crates/egosync-engine/src/registry.rs` + `src/lib.rs` -- 六类型原样迁入（含 Clone 等 derive）+ `ChatSessionRegistry`（pub 字段组合六类型，derive Default）+ chat.rs:58-92 辅助函数迁为方法；lib.rs 声明；壳 chat.rs 顶部 pub use 回引六类型（全部既有消费者零改动）-- 类型归位（纯搬移）
- [x] `egosync-app/src-tauri/src/{lib.rs,commands/chat.rs,commands/mcp.rs,commands/skill.rs,commands/app.rs,services/companion_dispatch.rs,services/agent_engine.rs}` -- Registry 接线一次性切换：lib.rs 六 manage→单 manage(ChatSessionRegistry) + TauriEventBus 单实例统一（加 Clone）；chat.rs 删回引、State 注入与 busy 分支/清理闭包改经 Registry（锁结构逐字节保持）、emit_chat_event→`&dyn EngineEvents` + 常量（含 :422 llm:stream 兜底、:1075/:1087）、命令签名 +State<TauriEventBus>；mcp/skill/app 12 命令换 State<ChatSessionRegistry>；companion_dispatch:504-513 换 Registry；agent_engine 两触点最小补丁（:16 use 改 engine 路径、:2358 try_state::<ChatSessionRegistry>、chat.rs:360 改传 registry.opencode_sessions.0.clone()）；companion_snapshot 契约测试追加三常量 contains 钉子；特征测试 setup 适配重跑绿 -- 一次性接线（无壳内桥接） **【主会话补齐】companion_snapshot contains 钉子实为 4 事件（message:saved/created/deleted/title-updated——title-updated 为 15.2 后新增监听，一并钉住）**
- [x] `crates/egosync-engine/src/services/{agent_engine,delegate_bridge}.rs` + 壳 `services/mod.rs`/`lib.rs` -- 两文件整体平移：agent_engine 9 emit→bus+常量（4 helper 签名 AppHandle→bus）、AppHandle 参数→接缝（run_stream：bus `Arc<dyn EngineEvents>` + registry `Arc<ChatSessionRegistry>` + AgentConfigService/SidecarManager/secret/project_dir 参数化；try_state ×6 全消）、KeyringSecretStore ×3→`&dyn SecretStore`、:41/:58 路径函数→注入、:4022 泛型随 AppHandle 消失退化、测试 include_str! 路径 4→2 级；delegate_bridge：字段→`Option<Arc<dyn EngineEvents>>`、:218 try_state→参数、:289→skills_root 入参、:509→tokio::spawn、2 emit→bus；engine services/mod 声明 + 壳 pub use 回引；壳侧 resolve_default_provider 调用点（chat.rs:1021、memory_pipeline.rs:40）改引 engine 路径 -- 域归位 **【主会话补齐】chat.rs tests 模块补 `use std::collections::HashMap`（六类型迁出使顶层 use 移除、测试经 super::* 失去该导入——两处编译错修复）**
- [x] `egosync-app/src-tauri/src/commands/app.rs` -- :177/:197 perf-test 字面量→LLM_STREAM_EVENT 常量 -- 发射源唯一化收尾 **【主会话补齐】子代理遗漏此任务——主会话审读时发现并补上（两处字面量→crate::events::LLM_STREAM_EVENT，`cargo check --features perf-test` 零错验证；默认构建不编译该特性、坏引用不会自然暴露，故特特性门控编译核验）**
- [x] 收口验证 -- 特征测试重跑（断言不变）、`npm run test:all`、`npm run build`、grep 断言、启动冒烟、e2e（15.2 裁决 A 口径）-- 硬边界

**Acceptance Criteria:**

- Given 特征测试先行，when 落位并运行于抽取前代码，then 同会话并发双发恰一次 LLM 调用、恰一条 busy 落库、busy 经 Ok 通道返回；busy 消息 role=assistant（companion 探测协议根）
- Given agent_engine 迁移，when 平移完成，then engine 内零 `tauri::`（含注释）、9 emit 全经 EventBus + events.rs 常量、`llm:stream` payload 逐字节一致、delegate_bridge 随迁收编、内联 114 测试随迁全绿
- Given chat 命令层事件面，when emit_chat_event 改造，then message:saved / conversation:created / conversation:deleted（+ title-updated + llm:stream 兜底）经注入 EventBus 常量发射；companion_snapshot 监听仍达（contains 钉子护住契约锁盲区）
- Given ChatSessionRegistry，when 六组状态迁入，then Arc 组合无 Tauri 类型、lib.rs 单 manage、chat/mcp/skill/app/companion_dispatch/agent_engine 全部薄调用同一 Registry
- Given 锁粒度守恒，when 抽取后特征测试重跑，then 五不变量保持、断言逐项与抽取前一致（setup 仅接线适配）
- Given 手机伴侣流式镜像，when llm:stream 经 TauriEventBus 转发，then companion_dispatch:708 / companion_snapshot:803 监听照收（壳内消费者无感）
- Given 桌面回归收口，when test:all + build + grep + 冒烟 + e2e（15.2 裁决 A 口径），then 全绿/豁免成立

## Implementation Notes

**执行记录（2026-09-18）：**

- **迁移落地**：engine 新增 events.rs 7 常量（值钉测试 chat_domain_event_constants_pin_values）、registry.rs（六 newtype 逐字节迁入 + ChatSessionRegistry pub 字段组合 + token 族辅助方法）、services/{agent_engine,delegate_bridge}.rs（8553/1223 行整体平移）；壳侧 lib.rs 六 manage→单 manage(Arc\<ChatSessionRegistry>) + TauriEventBus 单实例（手写 Clone impl——derive 会错误要求 R: Clone）、chat.rs 重接线（emit_chat_event 改 &dyn EngineEvents、busy 分支经 registry.streaming_state、命令泛型化 +State\<TauriEventBus>）、mcp/skill/app 12 命令换 State\<Arc\<ChatSessionRegistry>>、companion_dispatch 三调用点换参、companion_snapshot 追加 4 常量 contains 钉子；壳 services/mod.rs 两模块改 engine pub use 回引。
- **特征测试前后双绿（锁粒度守恒执行级证据）**：抽取前代码 = 基线 aba1b8c + 仅 OQ1 测试使能补丁（Cargo.toml dev-dep tauri test feature；chat_send_message/run_stream/emit 辅助族 11 函数泛型化 `<R: tauri::Runtime>`——发布物零影响，MockRuntime 注入）。运行 `cargo test --test test_chat_busy_mutex`：**2/2 通过**。迁移后同断言重跑：**2/2 通过**。断言逐项：恰一次 LLM（挂起 TcpListener accept 计数=1）、恰一条 busy 落库（role=assistant + 固定文案）、busy 经 Ok 通道、拒绝方零 user 插入/零 token/零 spawn、异会话并发互不阻塞、断连后 streaming 清理闭包收尾。**诚实记录**：实现子代理死于磁盘满（ENOSPC）未跑成抽取前基线——主会话以 git 手术补跑（`git checkout aba1b8c` 还原 + 施加使能补丁 + 运行 + 安全 diff 逐字节恢复迁移树，`cmp` 证明恢复零损）。
- **平移等价性审计**：`git show aba1b8c:壳路径 | diff - engine路径` 逐文件核对——agent_engine 481 行差异全部落在四类允许改动（接缝参数 bus/registry/agent_config/sidecar/secret/project_dir、emit 机械改写 to_value+常量、use 路径、include_str! 4→2 级）；delegate_bridge 97 行差异同类（Option\<AppHandle>→Option\<Arc\<dyn EngineEvents>> + agent_config/skills_root/secret 构造注入、tauri::async_runtime::spawn→tokio::spawn、2 emit 改写）。测试数量守恒：agent_engine 114→114、delegate_bridge 13→13；全仓对账：壳 236−127=109 ✓、engine 610+127+3=740 ✓。
- **磁盘事件（根因与处置）**：实现子代理中途死于根盘 97% 满（ENOSPC，cargo 无法写产物）。处置：删 `src-tauri/target/debug/incremental`（6.7G 可再生缓存）；后续测试复跑使增量缓存再涨（2.8G），release 构建前再清（6.4G→9.1G 可用）。子代理已完成的工作经全量 diff 审计后保留（质量合格，仅 app.rs 常量化与 tests HashMap 导入两处由主会话补齐）。
- **警告分析**：engine 构建有 6 条既有警告（dead_code 等）——`git show aba1b8c` 对照证实 `stream_payload_from_sse`/`build_tool_process_event_candidate` 等在基线即无非测试调用方（调用点全在 `#[cfg(test)]`），警告随文件平移而非新增；15.3 触碰的全部文件零新警告。
- **test_companion 偶发失败归因**：首跑 test:all 时 `try_enqueue_single_drops_when_full_without_disconnect` 失败（WS ResetWithoutClosingHandshake + 入队超时）——当时主会话正并行编译 `cargo check --features perf-test`（CPU 争用）。隔离复跑通过、干净全量重跑 37/37 通过。归因：并发编译争用导致的时序抖动，非 15.3 回归（该测试属 WS 发送队列逻辑，不涉 chat 域）。
- **启动冒烟记录（裁决 A 口径，15.2 同程序）**：`npm run tauri build` release 编译成功（8m34s，binary 41MB 级）；AppImage linuxdeploy 失败（环境级、与 15-1/15-2 同源，deb/rpm 打包成功不影响二进制）。冒烟命令：`timeout 40 xvfb-run -a ./target/release/egosync`。结果：进程存活满 40s 被 timeout 终止（≥30s ✓）；`grep -c "there is no reactor running\|panicked"` = 0；正向证据：`egosync_engine::db::pool` 双池迁移执行完成、**迁移后的 `egosync_engine::services::delegate_bridge: delegate bridge listening port=43707`**（平移件运行时正常）、`egosync_engine::services::scheduler: 周五大石头检查跳过`（调度循环正常）；两处环境级降级（opencode sidecar 缺 PATH、xvfb 下 keyring DBus 不可用）与 15-1/15-2 基线完全一致。
- **e2e 归因记录（2026-09-18，15.2 裁决 A 口径）**：`cd egosync-app/tests/e2e && xvfb-run --auto-servernum npm test`（tauri-driver 在 PATH）。全量结果：**2/9 通过**——`llm-streaming`（3/3，**本故事迁移域关键正向证据**：chat_send_message 经迁移后 engine 的流式 UI 契约——输入框保持可用、发送按钮复位不卡死，恰是 15.3 平移的 busy/流式语义面）+ `task-management`（5/5，任务 CRUD 穿行 engine task 域）；与 15-2 基线通过组合**完全相同**。7 失败逐项归因（证据链）：
  1. **DB 污染（既有缺陷，15-1 在案，本次再证）**：wdio.conf.ts:60 擦 `~/.config/com.egosync.desktop`（错目录），真实库 `~/.local/share/com.egosync.desktop` 持久残留——实测 `onboarding_completed='true'` + 12 个历史角色 → 期望全新应用态的 spec（cold-start-onboarding 等）首步即败。
  2. **陈旧选择器（既有缺陷，15-2 在案，本次再证）**：role-crud 断言 `button=保存更改`（生产 SettingsTab 不渲染该按钮——其测试文件 SettingsTab.test.tsx:1085 自证「不在文档中」）与 `button=重新启用`（仅 ButlerSettingsContent:1265 渲染，非 spec 驱动的归档恢复位置）——spec 与应用文案漂移，非本故事改动（15.3 前端零改动，git diff 无 src/ 文件）。
  3. **driver 抖动（环境级，15-1/15-2 在案，本次再证）**：单跑复验时残留 tauri-driver 占 4444 口致会话建立超时，`pkill -x tauri-driver` 清理后 llm-streaming 立即 3/3 通过。
  4. 其余失败 spec（accessibility/briefing-review/butler-conversation/conflict-arbitration/performance）均属上述三类（新鲜应用状态依赖 / 陈旧选择器 / driver 抖动），无一件可归因 15.3 改动。
  正向执行级证据（冒烟记录同源）：应用运行期双池迁移全绿、迁移后 delegate_bridge 正常监听、scheduler 循环运行、零 reactor panic。**结论**：现行阻断全部为既有非本故事缺陷且逐项有证；busy/stop 语义由特征测试在 Rust 层钉死（OQ2 裁决 A 原文）；e2e 平台修复（擦目录错位、陈旧选择器、driver 泄漏）仍应作为独立工作项跟踪。
- **收口验证**：`npm run test:all` 全绿（vitest 43 文件 440 测试 + 壳 cargo 150——lib 109/characteristic 2/companion 37/packaging 1/app 1——+ engine 740）；`npm run build` tsc 零类型错误（前端零改动守门）；`grep -rn 'tauri::' crates/egosync-engine/src/` 零命中（CI 同口径）；`cargo check --features perf-test` 零错（app.rs 常量化验证）。

**评审补丁（step-04 分诊后由主会话应用，2026-09-18，实现子代理已终结故按规程自行应用）：**

- **发射侧执行级验证（VG-1+盲 12）**：`test_chat_busy_mutex.rs` 新增第三测 `chat_events_reach_listeners_and_busy_branch_emits_nothing`——MockRuntime 事件环上监听端真实收到 message:saved（恰一次=接受方 user 消息；busy 分支零发射即协议静默性执行级证据）、conversation:created、conversation:deleted（title-updated 与前三者共用 emit_chat_event 助手，generate_title 需真实 LLM 响应不在覆盖内，诚实记录）；engine `agent_engine.rs` 新增 RecordingBus 测试替身 + `create_role_tool_emits_role_proposed_via_bus`（role:proposed 事件名 + conversationId/name/icon/color/goal camelCase 六字段——顺带钉实前端 camelCase 契约）。
- **测试使能豁免扩展（OQ1-A 同类，供人工复核）**：chat_new_conversation / chat_delete_conversation 两命令及私有 generate_title 按 chat_send_message 同一机械模式泛型化 `<R: tauri::Runtime>`（+State\<TauriEventBus\<R\>\>）——仅为发射测试以 MockRuntime 驱动会话命令对；tauri 命令宏 per-runtime 实例化，发布物 Wry 路径零影响。
- **伪造 Registry 消除（盲 5）**：spawn_memory_extraction_after_idle 改持真实共享 Arc\<ChatSessionRegistry\>（schedule_memory_extraction 签名 &Arc，单一生产调用点零改动）——六组状态同身份，方法演化不再有静默错实例风险。
- **测试卫生（盲 3/4/11）**：删从未 await 的死屏障及误导注释（并发语义由 join! + 锁竞争承担，注释改述）；删壳侧 token 族两份重复测试（engine registry.rs 为迁移归宿）；两测试尾部追加 wait_cancel_tokens_empty（清理生命周期后半）。
- **簿记（盲 7/10）**：deferred-work.md——TauriEventBus 双实例条目标注 15.3 已解决（半条，KeyringSecretStore 半仍开放）+ TestSecretStore 增长记录 + 本轮五条新 defer 入册；sprint-status.yaml 15-3 → review。

## Spec Change Log

## Review Triage Log

**step-04 三层评审（盲扫 13 / 边界 2 / 验证缺口 3，2026-09-18）——无 intent_gap、无 bad_spec（无环回）；4 组补丁由主会话应用（实现子代理已终结于 ENOSPC），5 项 defer 入册，4 项驳回：**

| # | 层 | 发现（简） | 裁定 | 证据与处置 |
|---|---|----------|------|----------|
| VG-1 | 验证缺口 | 9 处 emit 改 bus+常量后仅订阅侧有钉，发射侧零执行级验证（删任一 emit 全绿） | **medium（预验证信任）** | **patch 已应用**：test_chat_busy_mutex 新增 `chat_events_reach_listeners_and_busy_branch_emits_nothing`（MockRuntime 监听端断言 message:saved 恰一次 / conversation:created / conversation:deleted 到达）；engine agent_engine 新增 RecordingBus 替身 + `create_role_tool_emits_role_proposed_via_bus`（钉 role:proposed 事件名 + camelCase payload 六字段）。**测试使能豁免扩展（记录供人工复核）**：OQ1-A 的 `<R: Runtime>` 泛型化按同类机械模式扩展至 chat_new_conversation / chat_delete_conversation / generate_title（私有 helper）——仅测试消费 MockRuntime，发布物 Wry 实例化零影响 |
| VG-2 | 验证缺口 | 默认 project_dir 解析迁壳后无测试观察取值/副作用 | **medium（预验证信任）** | **defer**（按层内处置入册）：可观察边界需活 sidecar；基线期同零覆盖（原函数持 AppHandle 不可测），非本故事缩减 |
| VG-3 | 验证缺口 | 测试死屏障（:183/:188 `let _ = barrier`） | **low** | **patch 已应用**（与盲扫 #3 同项）：删屏障与误导注释 |
| 盲 1 | 盲扫 | role:proposed/delegated 无 contains 钉（发射件迁出扫描树） | **low** | **defer**：COMMAND_SOURCES 仅六命令文件（companion_snapshot:894-900 亲证），agent_engine 自基线即不在内——盲区先于本故事；发射侧已由 VG-1 补丁钉 role:proposed |
| 盲 2 | 盲扫 | task:tool-action 双发射习语（task_decomposition.rs:25/:36 裸字面量） | **low** | **defer**：冻结 Never 明裁壳侧非本域发射面留 15.5；该字面量正是 source-scan 对此事件的锚点（清单注释 :751-752 明示） |
| 盲 3 | 盲扫 | 死屏障误导 | **low** | **patch 已应用**（=VG-3） |
| 盲 4 | 盲扫 | token 族测试跨箱重复（registry.rs:101/:114 与 chat.rs:845/:856 同名同断言） | **low** | **patch 已应用**：删壳侧两份（engine 份为迁移归宿；防锁语义双拷贝漂移） |
| 盲 5 | 盲扫 | spawn_memory_extraction_after_idle 伪造局部 Registry（仅 memory_extraction 真字段） | **low** | **patch 已应用**：改持真实共享 Arc（六状态同身份；方法将来触及其他字段不再静默错实例） |
| 盲 6 | 盲扫 | opencode-workspace 路径壳内三份解析 | **low** | **defer**：计数未因本故事增加（engine 份等价迁为 chat.rs 份，lib.rs/skill.rs 份先在）；合并属顺手重构（冻结 Never） |
| 盲 7 | 盲扫 | deferred-work 未随故事更新（TauriEventBus 双实例条目已解决未标注；TestSecretStore 2→5 未记） | **low** | **patch 已应用**：TauriEventBus 半条标注 15.3 已解决（scheduler 复用单实例）；TestSecretStore 增长与新增五条 defer 入册 |
| 盲 8 | 盲扫 | extract_event_names 提取器增强（deferred-work 记 15.3/15.5 顺带）未随带 | **false** | 冻结 Never 明选钉子方案（「仅按 15.2 补丁 1 先例追加 contains 钉子」），spec 裁决优先于旧 defer 建议；提取器增强仍为有效后续工作（条目在册） |
| 盲 9 | 盲扫 | llm:stream 双重序列化（to_value + emit 再序列化） | **low** | **defer**：to_value 为冻结 Always 明定的机械改写类，接缝签名由 15.2 冻结；单 token 量级开销可忽略；接缝演化归 15.4/15.5 |
| 盲 10 | 盲扫 | 故事状态滞后（spec in-progress / yaml in-progress） | **low** | **patch 已应用**：spec 前置翻转 in-review 在评审启动前完成（diff 快照早于翻转故盲扫层看到旧值）；sprint-status 15-3 → review |
| 盲 11 | 盲扫 | 清理生命周期半断言（token 不归零未验证） | **low** | **patch 已应用**：两测试尾部追加 wait_cancel_tokens_empty |
| 盲 12 | 盲扫 | busy 协议事件静默（busy 不发 message:saved）无执行级钉 | **medium** | **patch 已应用**（并入 VG-1 发射测试）：监听端断言恰一条 message:saved——恰一条而非两条即 busy 静默性的执行级证据 |
| 盲 13 | 盲扫 | engine 独立图无 preserve_order，逐字节一致依赖桌面图传递激活 | **false** | cargo 特性统一化：engine 编入桌面二进制时与壳共用同一 serde_json 实例（preserve_order 经桌面图激活），发布物键序保持；引擎独立测试无字节级断言（形状测试断言解析后 JSON）；15.4 场景已由在册 preserve_order 条目覆盖 |
| 边 A | 边界 | spec 任务 0 称「in-memory sqlite」而实测为 tempdir 文件库 | **low** | 机制差异不实质（同等隔离、无跨测态）；修法=编辑本 spec → 按规程驳回（记录于此） |
| 边 B | 边界 | 死屏障（=VG-3/盲 3） | **low** | **patch 已应用**（同项） |

**分组路由汇总**：①发射侧执行级验证缺口（VG-1+盲 12+盲 3/11 测试卫生同文件）→ patch（已应用）；②伪造 Registry（盲 5）→ patch；③跨箱测试重复（盲 4）→ patch；④簿记（盲 7+盲 10）→ patch；⑤既有盲区/平台债（VG-2+盲 1+盲 2+盲 6+盲 9）→ defer（五条入册 deferred-work.md）；驳回：盲 8、盲 13、边 A（证据见各行）。

## Design Notes

- **memory_pipeline/task_decomposition 归属裁决（冲突显式化）**：15.2 Never「memory_pipeline / task_decomposition / energy_calculator 留 15.3」与 epics 15.3 AC 未点名冲突。以 epics.md（权威故事源）为准：留壳。依据：依赖单向（memory_pipeline→agent_engine、task_decomposition→engine services，壳引 engine 合法正向引用）；energy_calculator 已被 15.2 闭包裁决实际迁入（先例不构成迁移义务）；两者引擎化的真实触发点是 15.4 server 是否需要，届时随闭包裁决。
- **Registry 形态**：保留六个 newtype 包装（类型定义逐字节搬移，消费者 `.0` 字段访问模式保持），Registry 以 pub 字段组合——避免方法化重写（锁粒度守恒的可审计性优先；「薄调用」指命令层经 Registry 取字段/方法，不要求 API 重设计）。chat.rs:58-92 辅助函数迁为 Registry 方法（cancel/replace token 族）。
- **任务顺序的绿性设计**：registry 类型先落（纯搬移 + pub use 回引，消费者零改动）→ 接线一次性切换（agent_engine 留壳期两触点最小补丁保行为：try_state::<ChatSessionRegistry> 取 mcp_scope_lock——若不补，try_state::<旧类型> 返回 None 导致锁静默缺失）→ 两文件整体平移。每步 cargo test + 特征测试绿。
- **特征测试 hold 机制**：AgentBridge reqwest 无超时（agent_bridge.rs:20-26 亲证）→ 挂起 TcpListener（accept 后不响应）使 run_stream 停在 create_session await，busy 窗口确定性开启；不依赖 axum 完整协议模拟。断点后清理：drop listener → 连接断 → run_stream Err → 兜底 done 帧 → 清理闭包移除 streaming 标记。
- **泛型化 spike**：命令宏源码核验（tauri-macros-2.6.2 wrapper.rs:325 原样再发射 `#function` + macro_rules 包装 per-runtime 实例化）支持 `<R: Runtime>` 命令；风险残留为实证性质，spike 放任务 0 首步，失败即回 OQ1 重裁决（不硬闯）。

## Verification

**Commands:**
- `cd egosync-app && npm run test:all` -- vitest + 壳 cargo test + engine cargo test 全绿（EXIT=0）
- `cd egosync-app && npm run build` -- tsc 零类型错误（前端零改动守门）
- `grep -rn 'tauri::' crates/egosync-engine/src/` -- 零输出（与 CI 断言同口径，含注释）
- 特征测试抽取前后各跑一次 -- 断言逐项一致（锁粒度守恒的执行级证据）
- 启动冒烟：release 构建 + `xvfb-run` ≥30s，进程存活、输出无 "there is no reactor running"/panic（记录入 Implementation Notes）
- `cd egosync-app/tests/e2e && npm test` -- 15.2 裁决 A 口径：执行一次并逐项归因（现行失败须全部归因既有缺陷且有证据链；busy/stop 语义由特征测试钉死）

**Manual checks (if no CLI):**
