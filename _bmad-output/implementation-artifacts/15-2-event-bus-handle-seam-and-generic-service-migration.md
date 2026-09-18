---
title: '事件总线、Handle 接缝与泛域服务迁移（Story 15.2）'
type: 'refactor'
created: '2026-09-18'
status: 'done' # 2026-09-18 人工批准：两处冻结块偏差——delegate_bridge 留壳随 15.3 chat 域收编、energy_calculator 随闭包迁入（scheduler:319 硬依赖）；评审三层全过、test:all 全绿、启动冒烟零 panic
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '7ed52b7d530cb2c589a34c8c80c3d4320417223c'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-15-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 15.1 后泛域服务仍留桌面壳：AC 点名的 12 个 service 中 7 个直接持有 AppHandle 发射事件、7 处 `tauri::async_runtime::spawn` 混用，三个同步启动入口（spawn_scheduler / 两个 spawn_hourly_watch）在 setup 同步上下文裸 tokio::spawn 会 panic（v0.1.6-alpha.1 历史事故）；事件名字面量散落各处（常量甚至定义在 commands 层），无单一事实源。云端赛道（15.4 server）无法在 tauri 事件机制上复用这些服务。

**Approach:** 落地四接缝中的两条——engine 定义 `EngineEvents` trait（对象安全 `emit(&self, event: &str, payload: serde_json::Value)`）+ 壳侧 `TauriEventBus` 转发 `app_handle.emit`；新建 engine `events.rs` 收编 9 个事件名常量为唯一发射源；14 个泛域 service（AC 12 件 + 依赖闭包追加 notification_service / suggestion_generator）迁入 engine——AppHandle 换 EventBus 注入、密钥经既有 SecretStore trait、skills_root 换路径入参；三个同步启动入口接收注入的 `tokio::runtime::Handle`。桌面行为零变化（事件名与 payload JSON 逐字节一致）。

## Boundaries & Constraints

**Always:** 平移逐字节等价，允许的改动仅四类——①import 路径调整；②接缝调用替换（EventBus / SecretStore / skills_root / Handle）；③AC 豁免的发射点机械改写（强类型 payload → 常量名 + `serde_json::to_value`）；④常量与函数搬移换引用不换值。emit 错误语义按站点保持（bigrock_protection 两处 warn、scheduler/delegate_bridge 忽略）。三个入口 Handle 必经 `tauri::async_runtime::handle()` 注入；register_stream_mirror 随 companion_dispatch 留壳且 spawn 不动。agent_engine 提取件（resolve_default_provider、build_role_task_summary、build_role_memory_summary 及 4 个共享 helper）以「搬入 engine + agent_engine import 回引」落地，agent_engine 其余函数体零改动。收口 = `npm run test:all` 全绿；e2e 按用户裁决（2026-09-18，选项 A）：**证据豁免**——沿用 15-1 基线对照模式（环境级缺陷先于本故事，平台修复为独立工作项），启动路径 AC 以 release 二进制 xvfb 启动冒烟作替代执行级证据（进程存活 ≥30s、输出无 "there is no reactor running"/panic 文本），豁免依据与冒烟记录入 Implementation Notes。

**Never:** 不迁 agent_engine 本体与 chat 域（15.3）；memory_pipeline / task_decomposition / energy_calculator 留 15.3；companion_* 四件套留壳；不建 server binary（15.4）；不做 TS 侧事件名/payload 构建期生成（15.5）；不收编纯壳侧事件名常量（role:\* / task:created 等，留 15.3/15.5）；不改 companion_snapshot 的 WRITE_SIGNAL_EVENTS 25 名硬编码清单（非 AC 范围，漂移风险已知在案）；不引入第五接缝（ProviderFactory 类抽象违反四接缝范式）；KeyringError 改名决策留 15.4 错误形状冻结时。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 桌面启动（reactor 安全） | setup 同步调用三个入口 | 经注入 Handle 派生任务，无 "there is no reactor running" panic | 入口内部错误仅 warn 不阻塞 setup（现状） |
| 事件发射等价 | A 组 11 发射点改经 TauriEventBus | 事件名与 payload JSON 形状与现状逐字节一致 | bigrock 两处 warn、scheduler/delegate 忽略——各站点语义保持 |
| 密钥经既有接缝 | task_classifier / mission_inferrer / suggestion_generator / briefing / review 的 provider 构造 | 经 `&dyn SecretStore` 完成，keyring 不可用时按既有语义降级 | KeyringError 语义不变 |
| skills_root 注入 | 壳侧传 Some(app_data_dir 预计算路径) | 创建 Skill 流程行为不变 | None → runtime_unavailable 响应语义保持 |
| 无 opencode 环境 | sidecar 不可用 | event_router run_pump 条件 spawn 保持壳侧原样 | 降级语义不变 |
| 既有 dev 库 | 迁移前已存在的 egosync.db | 无新 migration、零 schema 变化 | N/A |

</frozen-after-approval>

## Code Map

**调查结论（三路子代理 + 人工复核，2026-09-18）：**

- **事件全景** — Rust 侧共 30 个事件名；A 组（迁移服务发射）8 名 11 点：`bigrock:protection`（bigrock_protection.rs:251,399）、`bigrock:reminder`（bigrock_reminder.rs:157）、`briefing:generated`（briefing_generator.rs:139）、`review:generated`（review_generator.rs:180）、`q2:reminder`（q2_protection_reminder.rs:218）、`notification:new`（scheduler.rs:265）、`task:classified`（delegate_bridge.rs:516）、`skill-registry-updated`（delegate_bridge.rs:410）。加壳监听侧 `llm:stream` 共 9 名入常量源。payload 形式：7 处强类型 struct（5 个 Payload struct 内嵌于迁移文件随迁；NotificationNewPayload 已在 engine models）、2 处 `json!`/域对象。
- **常量现状** — BIGROCK_PROTECTION_EVENT 等 5 个常量散落在各 service 文件内（随文件平移后改引 events.rs）；`TASK_CLASSIFIED_EVENT` 定义在 commands/task.rs:11（被 review.rs:91 跨模块引用）——搬入 engine events.rs，壳侧两处改引。
- **12 个 service 的 tauri 用法分层** — 纯净直迁 5 件：event_router（零 tauri import）、mission_inferrer、task_classifier、task_deadline_watch、task_protection_watch（后四件仅各 1 处 `async_runtime::spawn`）；AppHandle 注入 7 件：bigrock_protection、bigrock_reminder、briefing_generator、q2_protection_reminder、review_generator（均 `Option<&AppHandle>`，None=测试路径）、scheduler（:322 owned + :156 Option）、delegate_bridge（:39 字段 `Option<AppHandle>` + :17 `use tauri::{AppHandle,Emitter,Manager}` + :289 `app_handle.path().app_data_dir()`）。
- **依赖闭包（14 件迁移集的由来）** — scheduler/bigrock×2/q2 → notification_service（纯净：pool 签名）→ suggestion_generator（:172/:184/:232 用 agent_engine 三函数）→ agent_engine（15.3 留壳）。engine 不能反向引用壳、第五接缝违约 → notification_service 与 suggestion_generator 随闭包迁移（15-1「传递闭包解读」裁决先例）；agent_engine 三函数 `resolve_default_provider`(:2357，db+secret+llm 构造，纯净)、`build_role_task_summary`(:1377)、`build_role_memory_summary`(:1874) 及共享 helper（BUTLER_MEMORY_PER_ROLE:1207、format_task_context_line:1338、select_task_context_items:1362、format_memory_reference_line:1463——被 agent_engine 其余函数 :1416/:1430/:1488 共用）提取入 engine，agent_engine 顶部 `use` 回引后其余函数体零改动。
- **spawn 清单（7 处 async_runtime::spawn）** — 三入口：scheduler.rs:323、task_deadline_watch.rs:87、task_protection_watch.rs:76 → `handle.spawn`（注入 Handle）；任务体 4 处：task_classifier.rs:277、mission_inferrer.rs:407、delegate_bridge.rs:506 → `tokio::spawn`（已有 runtime 上下文）；briefing:310/review:475 已是 tokio::spawn 不动。
- **密钥注入点** — briefing:84 / review:108,605 经 resolve_default_provider；task_classifier:307 与 mission_inferrer:434 各有私有 build_default_provider（:435 起调壳侧 load_secret）——签名加 `secret: &dyn SecretStore`，trait 方法为 save_secret/load_secret/delete_secret（engine services/secret_store.rs:13-21）。
- **scheduler 的 commands 依赖** — :613/:625 引 `crate::commands::settings` 8 个常量（KEY/DEFAULT × REVIEW_DAY/TIME、BIGROCK_REMINDER_DAY/TIME）——下沉 engine（scheduler.rs 内定义），commands/settings.rs 改 use 回引；briefing 相关 2 常量为私有且仅壳用，不动。
- **壳侧接线** — lib.rs:12-15 `pub use egosync_engine::{db,error,models}`（加 events）；:66-69 manage 区（KeyringSecretStore 先例，:69）加 `manage(TauriEventBus)`；:81-86 DelegateBridge::new 构造（app_data_dir 变量在 :90 区已有）；:263-269 event_router manage+条件 spawn（路径经 pub use 不变）；:301/:305/:375 三入口调用点；:351 register_stream_mirror 不动。services/mod.rs:4-7 现有 11 项 pub use 回引（追加 16 项：14 service + event_bus trait + role_context）。
- **commands 调用面** — briefing.rs:19、review.rs:30/:83/:103、mission.rs:39/:48、task.rs:47/:161 需加 `State<'_, TauriEventBus>` / `State<'_, KeyringSecretStore>`（15.1 commands/llm_config.rs 先例）；scheduler.rs / notification.rs / chat.rs:366,370 仅路径引用，零改动。
- **测试基建** — 迁移集内联测试全部可平移：纯函数 + in-memory sqlite；delegate_bridge 用 tempfile 文件型 sqlite（engine dev-deps 已备）。无环境变量/外部资源依赖。
- **CI** — ci.yml:238-272 engine job 断言 step（grep Cargo.toml 声明面 + Cargo.lock 传递闭包）追加第三断言：`grep -rn 'tauri::' crates/egosync-engine/src/` 零命中（现状已零命中，断言当日即绿）。engine 依赖清单预计零新增（迁移集用量已被 15.1 清单覆盖，实现期以实际 grep 复核）。

## Tasks & Acceptance

**Execution:**

- [x] `crates/egosync-engine/src/events.rs` + `src/lib.rs` -- 新建 9 常量（8 个 A 组事件名 + LLM_STREAM_EVENT，值与现状字面量逐一相同）+ `pub mod events;` -- 唯一发射源
- [x] `crates/egosync-engine/src/services/event_bus.rs` -- 新建 `pub trait EngineEvents: Send + Sync { fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String>; }`（对象安全、同步方法）-- 接缝二
- [x] `crates/egosync-engine/src/services/role_context.rs` -- 自 agent_engine 提取 build_role_task_summary / build_role_memory_summary + 4 个共享 helper（内容逐字节等价）；agent_engine 顶部 use 回引使 :1416/:1430/:1488 等既有调用零改动 -- 闭包解锁
- [x] `crates/egosync-engine/src/services/llm_config.rs` -- 追加 resolve_default_provider（自 agent_engine:2357 提取；签名加 `secret: &dyn SecretStore`，load_secret 改 trait 调用）；agent_engine :3576/:5087 两处调用改 engine 版并传 `KeyringSecretStore::new()`（unit struct 零成本构造）-- provider 构造归位
- [x] `crates/egosync-engine/src/services/` -- 平移 5 件纯净 service：event_router、task_classifier（:307 起 secret 参数）、task_deadline_watch、task_protection_watch、mission_inferrer（:434 起 secret 参数）；:277/:407/:87/:76/:75 spawn 按清单替换 -- 纯逻辑归位（第一批）
- [x] `crates/egosync-engine/src/services/{notification_service,suggestion_generator}.rs` -- 平移；suggestion_generator :172/:184 改引 role_context、:232 改 engine 版 resolve_default_provider（generate_suggestions 加 secret 参数）-- 闭包补全
- [x] `crates/egosync-engine/src/services/{bigrock_protection,bigrock_reminder,briefing_generator,q2_protection_reminder,review_generator,scheduler,delegate_bridge}.rs` -- 平移 7 件 AppHandle service：AppHandle 参数 → `Option<&dyn EngineEvents>`（scheduler :322 与 delegate_bridge 字段用 `Arc<dyn EngineEvents>`）；11 发射点机械改写（常量名 + to_value，warn/ignore 各站点保持）；briefing/review 改 engine 版 provider + secret 透传；scheduler 8 个 settings 常量下沉本文件；delegate_bridge :289 改 skills_root 入参（`Option<PathBuf>`）、:506 spawn→tokio -- 泛域归位（第二批）**【偏差】delegate_bridge 经冲突 2 裁决留壳（仅落地常量改引 :410/:516 与 classify secret :507 两点；spawn 保持 `tauri::async_runtime::spawn` 原样随壳不动——清单中「:506→tokio::spawn」一项随留壳作废，见 Implementation Notes）；energy_calculator 经冲突 1 裁决随闭包迁入（scheduler:319 硬依赖）**
- [x] `crates/egosync-engine/src/{lib.rs,services/mod.rs}` -- 16 个新模块声明 + lib.rs doc 注释补接缝二/四 -- 声明收口
- [x] `egosync-app/src-tauri/src/services/tauri_event_bus.rs` -- 新建 TauriEventBus 实现 EngineEvents（emit 转发 `app_handle.emit`，`tauri::Error → String`）-- 接缝二桌面侧
- [x] `egosync-app/src-tauri/src/{lib.rs,services/mod.rs}` -- lib.rs：manage(TauriEventBus)、:81-86 DelegateBridge::new 传 bus+skills_root、:301/:305/:375 三入口传 `tauri::async_runtime::handle()`、pub use 加 events；mod.rs：删 14 个 pub mod、pub use 追加回引、新增 tauri_event_bus -- 壳侧接线 **【偏差】DelegateBridge::new 不动（随冲突 2 留壳）；Handle 注入实为 `handle().inner().clone()`（tauri 2.11.2 RuntimeHandle 是枚举，见 Implementation Notes）**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 删除三提取件及 helper（搬移非删除语义），顶部 use 回引 engine 版；:3576/:5087 传 KeyringSecretStore::new()；其余函数体零改动 -- 提取源收口 **【补充】连带 :4552 委派建任务路径 classify 补 secret；删除三个失效 import**
- [x] `egosync-app/src-tauri/src/commands/{task,briefing,review,mission,settings}.rs` -- task.rs:11 常量删除改引 engine events、:47/:161 调用点传 secret/bus；briefing:19、review:30/:83/:103、mission:39/:48 加 State 注入传参；settings.rs 8 常量改 use 回引 -- 调用方适配 **【补充】task_check_q2_reminders 移除失效的注入参数 app_handle（注入参数前端不可见，零影响）**
- [x] `egosync-app/src-tauri/src/services/{companion_dispatch,companion_snapshot}.rs` -- :707/:803 `listen("llm:stream")` 改引 LLM_STREAM_EVENT（行为不变）-- AC 点名 **【补充】companion_snapshot:936 测试断言同改 crate::events:: 路径；companion_dispatch:594 生产 executor task_create 补 KeyringSecretStore state**
- [x] `.github/workflows/ci.yml` -- engine job 断言 step 追加 `grep -rn 'tauri::' crates/egosync-engine/src/` 零命中断言 -- 物理封禁兜底
- [x] 启动冒烟 + 基线对照（裁决 A 口径执行并记录）-- reactor 安全执行级证据与 e2e 豁免依据

**Acceptance Criteria:**

- Given EngineEvents 接缝，when trait 定义完成，then 方法签名为 `emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String>`（对象安全、非泛型）；engine 内全部事件发射经 trait
- Given events.rs 常量源，when 收编完成，then 9 个事件名全部以常量定义、engine 内发射点零字面量；TauriEventBus 转发后事件名与 payload JSON 形状与现状逐字节一致；companion_dispatch:707 与 companion_snapshot:803 监听引常量
- Given 发射点机械改写，when 强类型 payload 改写为「常量名 + Value」，then 除四类允许改动外零额外改写；warn/ignore 错误语义逐站点保持
- Given 14 个泛域 service（AC 12 件 + 闭包 2 件），when 迁入 engine，then 各自 AppHandle 换 EventBus 注入、任务体内 spawn 按 7 处清单替换、密钥经 SecretStore trait、skills_root 经入参；内联测试随文件迁移且全部通过
- Given 三个同步启动入口，when 签名改造，then 接收 `tokio::runtime::Handle`；lib.rs setup 传 `tauri::async_runtime::handle()`；register_stream_mirror 随 companion_dispatch 留壳、其 spawn 不变
- Given 桌面启动路径，when release 二进制 xvfb 启动冒烟（裁决 A 替代证据），then setup → 三个入口正常派生任务、无 "there is no reactor running" panic（进程存活 ≥30s、输出无 panic 文本）
- Given 桌面回归收口，when `npm run test:all` + e2e（证据豁免口径，基线对照与冒烟记录入 Implementation Notes），then 全绿/豁免成立

## Implementation Notes

- **A 组 11 发射点逐点审查（Verification 手工项）**：以 `git show 7ed52b7d:<file> | diff - crates/egosync-engine/src/services/<file>` 逐文件核对——notification_service 与 energy_calculator 零差异；event_router 仅 1 行注释改写（规避 CI grep 命中 `tauri::`）；其余 9 件新增行全部落在四类允许改动内（import 调整 / 接缝参数 / emit 机械改写：`serde_json::to_value(&payload).map_err(|e| e.to_string()).and_then(|payload| bus.emit(CONST, payload))` / scheduler 8 常量下沉——值与基线逐一相同）。warn/ignore 分支结构未变（bigrock×2/q2/briefing/review warn、scheduler 忽略）；payload 构造表达式仅经 to_value 包装。delegate_bridge 两点留壳但常量改引（:410/:516）。

**执行记录（2026-09-18）：**

- **迁移落地**：engine 新增 events.rs（9 常量）、services/event_bus.rs（EngineEvents trait）、services/role_context.rs（agent_engine 三提取件 + 4 共享 helper，函数体逐字节等价）、tauri_event_bus.rs 壳侧实现（emit 转发 `app_handle.emit`，`tauri::Error → String`）；15 个 service 文件迁入 engine（14 计划件中 delegate_bridge 换 energy_calculator，见下）；agent_engine 删除 5 常量 + 7 函数 + resolve_default_provider（共 163 行），顶部 use 回引；commands/{task,briefing,review,mission,settings}.rs 与 companion_dispatch/companion_snapshot 的 AC 点名改动全部落地；CI 第三断言已加。
- **冲突 1（energy_calculator，已决）**：frozen AC 要求 scheduler 迁移，而 scheduler.rs:319 调 `crate::services::energy_calculator::calculate_and_update_energy`，engine 不能反向引用壳 → energy_calculator 随闭包迁入（15-1「按依赖传递闭包解读」用户裁决先例）。它与 Never 条款「energy_calculator 留 15.3」冲突，按 frozen-AC 优先处理；该文件零 tauri、纯 pool 签名，迁移 diff 为零（与基线逐字节相同），风险极低。**遗留**：15.3 规划时注意该文件已在 engine，无需再迁。
- **冲突 2（delegate_bridge，已决）**：spec 计划其迁 engine（Arc<dyn EngineEvents> 字段 + skills_root 入参），但 :703/:729 调 `agent_engine::execute_delegate_to_role_with_source`（pub(crate)，深依赖 build_role_messages / 委派 provider 构造——chat 核心）与 `append_delegation_metadata`；迁文件必然拖动 agent_engine 本体（违反 Never「不迁 agent_engine 与 chat 域」）或引入 executor 注入闭包（= 第五接缝，违反 Never）。**裁决：留壳**。其 2 个发射点仍改引 engine events 常量（:410/:516）、classify 调用点补 secret（:507）、:506 `tauri::async_runtime::spawn` 按清单不动、DelegateBridge::new 无需 bus/skills_root 注入；15.3 随 agent_engine 本体迁移时一并收编。迁移计数：14 件 = 12 AC 件 − delegate_bridge + 闭包 2 件（notification_service/suggestion_generator）+ energy_calculator。
- **RuntimeHandle 适配（实现期发现）**：tauri 2.11.2 的 `tauri::async_runtime::handle()` 返回枚举 `RuntimeHandle`（非 `tokio::runtime::Handle` 本体）；实际注入式为 `tauri::async_runtime::handle().inner().clone()`（inner() 给出其 tokio Handle 引用）。三个入口均按此落地，语义 = 在 tauri 全局运行时上派生任务。
- **KeyringSecretStore 加 Clone derive**：命令层 spawn 闭包需 'static 移入（State 借用不可移）；unit struct Clone 零成本，`secrets.inner().clone()` 直取。agent_engine/task_decomposition 内部调用点按 spec 既有手法 `KeyringSecretStore::new()` 即席构造。
- **task_check_q2_reminders 移除注入参数 app_handle**：emit 改经注入 bus 后该参数不再使用（未用参数即编译警告）；注入类参数前端不可见，移除对外零影响。
- **spec 未枚举的连带调用点（实现期发现并修复）**：① agent_engine.rs:4552（委派建任务路径）classify_and_persist 补 secret；② task_decomposition.rs:25/:48（留壳）同补 secret——壳引 engine 服务为正向引用，合法；③ companion_snapshot.rs:936 测试断言引 TASK_CLASSIFIED_EVENT 改 `crate::events::`（常量搬家后 commands::task 路径失效）；④ companion_dispatch.rs:594 生产 executor 的 task_create 直调补第 4 参 `app.state::<KeyringSecretStore>()`；⑤ agent_engine 顶部删除 `use crate::services::secret_store;`（唯一使用者是已提取的 resolve_default_provider）与 AnthropicProvider/OpenAiProvider 两个 import（同因），测试专用 format_memory_reference_label 移入测试模块避免 lib-only 未用告警。**基线对照**：git worktree 上对 baseline 7ed52b7d 跑 cargo check，新旧警告集合完全一致——零新增警告。
- **启动冒烟记录（裁决 A 替代证据）**：`npm run tauri build` release 编译成功（4m52s，binary 41MB）；AppImage 打包步 linuxdeploy 失败（环境级，与 Origin 缺陷同类，先于本故事；不影响二进制）。冒烟命令：`xvfb-run -a ./egosync`（release 二进制，35s 存活检查）。结果：进程存活 ≥35s；`grep -c "there is no reactor running\|panicked"` = 0；日志含 `egosync_engine::services::scheduler: 周五大石头检查跳过`（证明注入 Handle 下调度器工作循环正常派生运行）、`delegate bridge listening`；两处环境级降级（opencode sidecar 缺失、xvfb 下 keyring DBus 不可用）与 15-1 基线一致。
- **e2e 基线对照记录（2026-09-18，含豁免依据更新）**：`cd egosync-app/tests/e2e && xvfb-run --auto-servernum npm test`（README 要求 xvfb；`tauri-driver` 在 `~/.cargo/bin`，须在 PATH）。全量结果：**2/9 通过**（`llm-streaming`、`task-management`——恰为最重度穿行迁移后端的两件：任务创建→engine task_classifier 经 KeyringSecretStore 接缝自动分类、流式会话），7 失败。**与 15-1 基线不同败**（当时 100% 止于 `Origin header is not a valid URL` 会话级缺陷）——今日 Origin 缺陷未复现（IPC 实测可用：仪表盘渲染出角色总览、任务分类真实落库），环境在两故事间演化。逐项归因（证据链）：
  1. **DB 污染（既有缺陷，15-1 已在案）**：e2e beforeSession 擦 `~/.config/com.egosync.app`（错目录），真实库 `~/.local/share/com.egosync.desktop` 持久残留（`onboarding_completed=true` + 5 个历史角色）→ 期望全新应用的 spec（cold-start-onboarding 等）首步即败。**清库对照实验**：移走真实数据目录后单跑 cold-start-onboarding，「首次启动应显示 Onboarding 视图」由败转**通过**（实验后已恢复原数据目录）。
  2. **陈旧选择器（既有缺陷）**：spec 断言 `button=配置 AI 模型`，而前端实际渲染「配置大模型服务」（OnboardingView.tsx:269）——15.2 零前端改动（git diff 无 src/ 文件），属 spec 与应用的既有文案漂移。
  3. **会话建立偶发超时（环境级）**：残留 tauri-driver 占 4444 口时新会话 POST 超时；清理后复跑正常建立。
  4. 其余 6 件失败均属上述三类（新鲜应用状态依赖 / 依赖前序步骤 / driver 抖动），无一件可归因 15.2 改动。
  正向执行级证据（取自 e2e 运行期应用日志）：全新库 31 迁移双池全绿；scheduler 工作循环在注入 Handle 下运行（周五大石头检查）；delegate bridge 正常监听；task_classifier 在无默认 LLM 时按既有语义降级 Q2 并落库。**结论**：裁决 A 豁免的字面口径（与 15-1 同败）已不成立（环境演化），但现行阻断全部为既有非本故事缺陷且逐项有证；启动路径 AC 证据以冒烟记录为准。e2e 平台修复（DB 隔离错目录、陈旧选择器、driver 抖动）仍应作为独立工作项跟踪。

**评审修正（step-04 评审轮，2026-09-18）：**
- 计数勘误：基线 A 组实为 **8 事件名 9 发射点**（上文与 I/O 矩阵的「11 点」为规划期误计——错把 delegate_bridge 2 点与 commands 层 task:classified 重复计入）；迁移后 **7 点经 `bus.emit`**（engine 内），delegate_bridge 2 点留壳但已改引常量。
- 逐文件审计勘误：14 件迁移 = 2 件零差异（notification_service、energy_calculator）+ 1 件注释改写（event_router）+ **11 件实质改动**（上文「其余 9 件」漏计 task_deadline_watch / task_protection_watch 的 Handle 签名与注释改动）。
- e2e 擦目录勘误：wdio.conf.ts:60 实擦 `~/.config/com.egosync.desktop`（上文引 `~/.config/com.egosync.app` 有误）；结论不变——config 目录 ≠ 真实数据目录 `~/.local/share/com.egosync.desktop`。
- e2e 豁免口径更新：豁免依据改为「逐项归因既有缺陷」（Origin 缺陷未复现、环境已演化），Verification 节已同步修正；「与基线同败」字面口径作废。

**评审补丁（step-04 裁决后由主会话应用，2026-09-18）：**
- 补丁 1（Triage #1/#16/#18）：engine events.rs 新增 `#[cfg(test)]` 值钉（NOTIFICATION_NEW / SKILL_REGISTRY_UPDATED / LLM_STREAM 三常量）；commands/notification.rs:56 字面量改引常量；companion_snapshot source-scan 测试追加 `WRITE_SIGNAL_EVENTS.contains(NOTIFICATION_NEW_EVENT)` 补偿断言（常量型发射对提取器失明）。
- 补丁 2（Triage #13）：CI 第三断言改显式 rc 守卫（rc=2 grep 自身失败须报错，不再按无命中放行）。
- 补丁 3（Triage #10）：q2 模块头「emit Tauri Event」改「emit 事件」；events.rs TASK_CLASSIFIED_EVENT 注释生产/消费方表述修正。
- 补丁后复验：engine 610 全绿（+1 钉子）、壳 275 全绿、`grep -rn 'tauri::' crates/egosync-engine/src/` 零命中。8 条延期项已登记 deferred-work.md。

## Spec Change Log

## Review Triage Log

<!-- 评审轮 1（step-04，2026-09-18）：盲扫 / 边缘 / 验证缺口三层并行；裁决依据含主会话独立复证（cargo tree 特性图、wdio.conf 实读、字面量 grep）。 -->
| # | 来源 | 发现 | 裁决 | 处置 |
|---|------|------|------|------|
| 1 | 盲扫+边缘 | `LLM_STREAM_EVENT` 半迁移：发射端字面量仍在（agent_engine×4、chat.rs:422、app.rs:177/197、companion_snapshot:807/:908） | medium：漂移面真实（重命名常量将静默断流），但发射端收编被 Never 显式排除（15.3/15.5 范围） | 以常量钉子测试守护（补丁 1） |
| 2 | 盲扫 | 「A 组 11 发射点」口径失真（基线实为 8 名 9 点；偏差后 7 点经 bus） | medium：核实为真——规划期误计 | Implementation Notes 勘误已补；frozen 矩阵计数留人工裁决 |
| 3 | 盲扫 | Implementation Notes 逐文件审计对不平账（2+1+9=12≠14） | medium：核实为真 | 勘误已补（实为 2+1+11） |
| 4 | 盲扫 | spawn 清单与代码矛盾（task 7 注解称「清单落地」，代码按留壳保持 tauri spawn） | medium：核实为真——注解笔误（代码行为正确） | task 7 注解已修正 |
| 5 | 盲扫 | Verification e2e 豁免判据失效未更新 | medium：核实为真 | Verification 口径已改「逐项归因」 |
| 6 | 盲扫 | e2e 平台缺陷无跟踪工作项 | medium：部分真实——DB 目录与 driver 抖动已被 15-1 既有 deferred 条目覆盖；陈旧选择器未登记 | defer 新条目（见 deferred-work 追加） |
| 7 | 盲扫 | e2e 证据记录引错目录（`.app` vs `.desktop`） | low：核实为真（结论不变） | 勘误已补 |
| 8 | 盲扫 | TestSecretStore 桩两处复制粘贴 | low：真实；去重属测试基建重构（15-1 #16 先例） | defer |
| 9 | 盲扫 | 引擎内三份默认 provider 构造并存 | medium：真实；合并违反本故事逐字节平移约束 | defer 登记 15.4 错误形状冻结 |
| 10 | 盲扫 | 迁移文件残留过时「Tauri」表述（q2 模块头）+ events.rs 注释生产/消费方颠倒 | low：核实为真 | 补丁 3（注释直改） |
| 11 | 盲扫 | 单一变更集混入无关记账（用户状态对齐 6 文件） | medium：真实 | 提交拆分纪律：故事提交仅暂存 15.2 文件（见呈现检查点） |
| 12 | 盲扫 | lib.rs 双 TauriEventBus 实例（manage + Arc::new 各一） | low：行为等价（无状态包装）；15.3 接线指引 | defer |
| 13 | 盲扫+边缘 | CI 第三断言 grep rc=2 时按无匹配放行 | medium：核实为真（set -e 不覆盖 if 条件分支） | 补丁 2（显式 rc 守卫） |
| 14 | 盲扫 | 事件常量无自动化锚点 | false：6 个钉子测试已存在（bigrock×2/briefing/review/q2 + companion TASK_CLASSIFIED ∈ WRITE_SIGNAL_EVENTS）；真实缺口为 3 名未钉（第 16 行） | 由第 16 行承接 |
| 15 | 盲扫 | 「含注释」grep 口径迫使注释改写 | low：真实；策略二选一（收窄 grep 或明示规约） | defer |
| 16 | 验证缺口 | 三个常量无值钉（NOTIFICATION_NEW / SKILL_REGISTRY_UPDATED / LLM_STREAM）——错拼入库即绿 | medium（预验证） | 补丁 1 |
| 17 | 验证缺口 | 启动路径无可重复验证（冒烟一次性、CI e2e 全禁） | medium（预验证） | defer（环境级，独立工作项） |
| 18 | 验证缺口 | commands/notification.rs:56 仍字面量发射 notification:new | medium（预验证） | 补丁 1（改引常量 + 配对钉子补偿 source-scan 盲区） |
| 19 | 验证缺口 | source-scan 测试对常量型发射失明 | medium（预验证） | defer（15.3/15.5 测试基建） |
| 20 | 验证缺口 | preserve_order 条件性：engine 独立图缺特性，非桌面宿主键序与桌面不同 | medium（预验证）：消费方序不敏感，无行为破坏；15.5 对等测试将受扰 | defer 登记 15.4/15.5 |
| 21 | 边缘 | 桌面 payload 键序经 BTreeMap 按字母序（字节不再一致） | false：cargo tree 实证壳图经 tauri-build→schemars 激活 serde_json/preserve_order，to_value 保持字段序、与基线字节一致（与第 20 行互补：该特性为传递激活，非显式声明） | 驳回（残留面入第 20 行 defer） |

## Design Notes

- **emit 返回 `Result<(), String>` 的理由**：bigrock_protection 两处 warn 站点需要错误信息（`error = %e`）；`tauri::Error` 不能进 engine 契约；不加 AppError variant（KeyringError 改名已 defer 至 15.4，不再搅动错误形状）。`to_value` 序列化错误与转发错误并入同一 `Result<(), String>` 链，共享各站点的 warn/ignore 语义。等价性论证：tauri `Emitter::emit` 内部本就经 serde_json 序列化，`to_value` 与其同源，JSON 字节形状不变。
- **闭包扩张（12 → 14）**：scheduler/bigrock×2/q2 → notification_service → suggestion_generator → agent_engine（3 函数）。第五接缝违反四接缝范式、engine 不能反向引用壳，迁移是满足 AC 的唯一路径；15-1 已有「按依赖传递闭包解读」用户裁决先例。
- **agent_engine 提取的回引技巧**：4 个共享 helper 名经顶部 `use` 引入作用域后，:1416/:1430/:1488/:1495 等既有调用点文本零改动——与 15.1 的 `pub use` 回引同款手法，把 36 万字节大文件的改动压到 import 区 + 2 个调用点。
- **KeyringSecretStore::new() 即席构造**：keyring v3 进程全局、实现为委托自由函数的 unit struct，在 agent_engine 调用点即席构造零成本，避免为 15.3 文件引入 State 管道。

## Verification

**Commands:**
- `cd egosync-app && npm run test:all` -- vitest + 壳 cargo test + engine cargo test 全绿（EXIT=0）
- `cd egosync-app && npm run build` -- tsc 零类型错误（前端无改动，守门）
- `grep -rn 'tauri::' crates/egosync-engine/src/` -- 零输出（物理封禁，与 CI 第三断言同口径）
- 启动冒烟（裁决 A 口径）：release 构建 + `xvfb-run` 启动 ≥30s，进程存活、输出无 "there is no reactor running" / panic 文本（命令与输出记录入 Implementation Notes）
- `cd egosync-app/tests/e2e && npm test` -- 证据豁免（裁决 A）：执行一次并逐项归因（现行失败须全部归因既有缺陷且有证据链；「与基线同败」口径已随环境演化作废——见 Implementation Notes e2e 记录），不作为收口门禁

**Manual checks (if no CLI):**
- A 组 9 发射点（基线口径）diff 逐点审查：常量值与原字面量相同、payload 构造表达式仅经 to_value 包装、warn/ignore 分支结构未变；其中 7 点经 bus（engine）、2 点留壳改引常量
