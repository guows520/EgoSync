# 评审报告：云端托管版增量章节（FR-44～FR-48）——版本与事实核实

- **评审对象**：`_bmad-output/planning-artifacts/architecture.md` 第 2391–2974 行（2026-09-17 追加的云端托管版增量章节）
- **评审镜头**：版本与事实核实（逐条判定"经过验证" vs "凭训练数据断言"）
- **评审日期**：2026-09-17
- **总体判定**：**PASS-WITH-FIXES**
- **核实方法**：仓库断言用 grep/read 对照源码逐条复核；平台断言用官方一手来源（docs.rs / MDN / Caddy / SQLite / OWASP / caniuse / tauri 与 chrono 源码）核实；一致性对照 PRD §4.15（prd-egosync.md 663–716 行）与伴侣章节（architecture.md 1926–2390 行）。

---

## 一、总体结论

本章的可验证断言**绝大多数精确属实**，且多处达到"数字级精确"（16 个 service 文件、agent_engine 22 处、chat.rs 恰 1092 行、恰 120 个注册 command、busy 文案逐字一致、去重 map 变量名点名到位、六个依赖版本全部对上 Cargo.toml）——这说明耦合点清单确实做过代码验证，不是凭训练数据编的。

但有 **1 处关键论据不成立**（决策 #3 的 spawn 替换安全性，见 F1）、**1 处 trait 设计与代码事实冲突**（EngineEvents 缺事件监听面，见 F2），以及若干数字偏差与平台断言过强（F3–F8）。F1 直接威胁本章自我设下的"桌面零回归"硬边界，必须在实施前修正措辞与设计。

---

## 二、仓库内断言核实表

| # | 章节断言（行号） | 代码证据 | 判定 |
|---|---|---|---|
| 1 | "16 个 service 文件引用 tauri::"（2428、2526） | `grep -l 'tauri::' src/services/*.rs` 精确命中 16 个文件 | ✅ 属实（精确） |
| 2 | "agent_engine.rs 耦合最深，20 余处"（2526） | agent_engine.rs 含 22 处 `tauri::` | ✅ 属实 |
| 3 | "commands/chat.rs（1092 行）持有五组托管状态"（2428、2528） | chat.rs 恰 1092 行；StreamingState(`Arc<Mutex<HashSet>>`)/CancelTokens/OpencodeSessions/OnboardingConversations/MemoryExtractionState 均定义于 chat.rs:19–39 并在 lib.rs manage | ✅ 属实，**但漏第六组**：chat.rs:16 `OpencodeMcpScopeLock`（lib.rs:68 manage）——见 F5 |
| 4 | "scheduler.rs 已在用裸 tokio::spawn（代码验证）"（2504、2883） | scheduler.rs:406/456/520/559/590 共 5 处裸 `tokio::spawn` | ✅ 属实——**但以此为"替换安全"论据不成立**，见 F1 |
| 5 | "约 8 处 tauri::async_runtime::spawn"（2527） | service 层 9 处/8 文件（清单漏列 companion_dispatch 2 处，"等"字带过）；**全代码库 19 处/13 文件**（commands 5 + lib.rs 5） | ⚠️ 口径偏小：若按决策原文"全量替换"，实际 19 处 |
| 6 | "lib.rs app.manage() 15 处"（2528） | lib.rs 实有 **16** 处 `.manage(` | ⚠️ 差 1 处 |
| 7 | "4 个 rfd 原生对话框 command"（2507、2529） | chat_pick_working_directory（chat.rs:518）、skill_pick_custom_directory（skill.rs:48）、data_export 内嵌导出目录选取（data.rs:43）、pick_import_file（data.rs:90） | ✅ 属实——注意第 4 个嵌在 `data_export` 本体内，牵连其 capability 归属（见 F8） |
| 8 | "LlmConfig 出参只有 api_key_ref；key 仅在 Create/Update 入参与 test_connection 用户主动输入"（2589） | models/settings.rs:47–59 LlmConfig 无 api_key 字段；api_key 仅见于 CreateLlmConfigInput(:68)/UpdateLlmConfigInput(:79) 及 commands/llm_config.rs:98（list_models_by_params 用户输入） | ✅ 属实（结构性论断成立） |
| 9 | "skill-scope-updated 是唯一前端→前端事件，Rust 无监听"（2490、2578） | 全前端仅 skillService.ts:10 一处 `emit('skill-scope-updated')`；Rust 侧 grep 零命中 | ✅ 属实（"唯一"也核实） |
| 10 | 同会话流式 busy："我还在想上一个问题，请稍等片刻…"（2667） | chat.rs:295–309：streaming 含 conv_id 时插入该 busy 消息并返回（companion_dispatch.rs:518 同文案） | ✅ 属实（逐字一致） |
| 11 | 调度去重是内存态、未持久化（2508） | scheduler.rs:329 `last_triggered_map`、:332 `last_briefing_trigger_date`、:338 `last_review_trigger_week` 均为循环内局部变量 | ✅ 属实（三处）——**但第四处不成立**：bigrock 保护已是 DB 持久化（bigrock_protection.rs:149 `is_reminded_today(last_reminded_at)`，字段在表内），见 F5 |
| 12 | "120 个注册 command"（2489、2775） | generate_handler 内 `commands::` 条目恰 **120** | ✅ 属实（精确） |
| 13 | 版本：axum 0.8 / tokio 1 / sqlx 0.8 / keyring 3 / reqwest 0.12 / chrono 0.4（2466、2472、2558） | src-tauri Cargo.toml: tokio="1"(:22)、keyring="3"(:28)、sqlx="0.8"(:30)、reqwest="0.12"(:33)、chrono="0.4"(:35)、axum="0.8"(:51)；relay-server Cargo.toml axum="0.8"+tokio="1"，且确有 Dockerfile/docker-compose.yml | ✅ 全部属实 |
| 14 | "无耦合部分：db/、models/、llm/、error.rs、agent_bridge（纯 reqwest）、sidecar（裸 tokio::process::Command）、data_export 可直接平移"（2530） | 上述文件/目录 `tauri::` 引用均为 0；sidecar.rs:5 `use tokio::process::{Child, Command}`（非 Tauri sidecar API，路径解析依赖 tauri.conf.json resources——章节已用"路径注入"接缝覆盖） | ✅ 属实 |
| 15 | 伴侣章节引用（禁 workspace 反模式、TransportStatus、1s→30s 退避+抖动、"重连即快照"、FR-42 降级、SNAPSHOT/COMMAND/STATE_DELTA 帧类型、companion-proto path 先例） | architecture.md 2150/2196、2043、2172、2074、2038/2101–2106、2071、2148–2150 逐一存在 | ✅ 属实 |
| 16 | "约 10 个测试文件级机械替换"（2909，Gap 清单） | `vi.mock('@tauri-apps/api')` 命中 **14** 个测试文件 | ⚠️ 低估约 40% |
| 17 | 运行时探测 `window.__TAURI_INTERNALS__`（2512、2574） | @tauri-apps/api/core.js:202 即 `window.__TAURI_INTERNALS__.invoke` | ✅ 属实 |

## 三、Web 平台断言核实表

| # | 章节断言（行号） | 一手来源 | 判定 |
|---|---|---|---|
| 1 | axum 0.8 SSE 内建、无需 feature flag（2466、2559） | docs.rs axum 0.8.9：`axum::response::sse`（Event/KeepAlive/Sse）位于核心；futures-util 为非可选依赖，feature 列表无 SSE 项 | ✅ 属实 |
| 2 | 浏览器 EventSource 不能带 Authorization 头（2432、2506） | MDN EventSource：构造器仅接受 `(url, { withCredentials })`，无 headers 选项——凭证只能走 Cookie 或查询串 | ✅ 属实（关键约束成立） |
| 3 | Caddy 反代 SSE "需 flush_interval -1 防缓冲截流"（2562、2812） | Caddy 官方 reverse_proxy 文档：flush_interval 负值=低延迟模式；**但对 `Content-Type: text/event-stream` 的响应该选项被忽略、自动立即刷新**。axum SSE 正是 text/event-stream | ❌ **断言过强/机制错误**（见 F3）——方向对（要防缓冲），但"需要 -1"不成立 |
| 4 | chrono::Local 在 Linux 读 TZ env（2509） | chrono 0.4.45 源码 tz_info/timezone.rs:29–34：`TimeZone::local(env_tz)` → 有 TZ 走 POSIX TZ 解析（命名时区查 /usr/share/zoneinfo 等 4 目录），无 TZ 读 /etc/localtime | ✅ 属实；"镜像装 tzdata"与源码行为互为印证（缺 tzdata 时命名时区解析失败） |
| 5 | tauri::async_runtime 本质是 tokio 全局运行时封装（2504） | tauri 2.11.5 官方文档："The singleton async runtime used by Tauri… Tauri uses tokio Runtime" | ✅ 属实——**但其推论"替换无行为变更"不成立**（见 F1） |
| 6 | Argon2id 适合令牌哈希、常时比较（2506、2635） | OWASP Password Storage Cheat Sheet：Argon2id 为首选（最低 m=19MiB/t=2/p=1）；常时比较为标准实践 | ✅ 基本属实。备注：若令牌为高熵随机值，Argon2id 属过度（无害）；因令牌可由用户自选，保守选择正确；章节未落 OWASP 参数下限 |
| 7 | iOS Safari 支持 EventSource/SSE（FR-45 要求含 iOS Safari） | caniuse：Safari on iOS 4.0+ 全支持，全球覆盖率 97.03% | ✅ 属实 |
| 8 | SQLite WAL 模式下运行中直接复制卷有一致性风险（2604） | SQLite 官方 howtocorrupt §1.2："事务进行中备份…可能含部分旧部分新内容而损坏"；§1.4："复制数据库文件而不复制其 journal（-wal）"列为致损坏路径 | ✅ 属实；章节"停容器或走逻辑级导出"与官方建议（停机复制 / VACUUM INTO / backup API）一致 |
| 9 | "浏览器 Notification API 仅前台可用"（2652） | MDN/平台事实：桌面浏览器**后台标签页**仍可发通知（文档存活期间）；iOS Safari 普通浏览**根本没有** Notification API（iOS 16.4+ 仅限加到主屏的 PWA） | ⚠️ 论据不精确（见 F6）——结论（V1 应用内通知）恰好仍然正确，且 iOS 侧实际比章节写的更受限 |
| 10 | （隐含）多浏览器多标签并发 SSE 无连接数风险（裁决 A"多浏览器=多窗口"） | MDN 警告：非 HTTP/2 下 SSE 受**每浏览器每域名 6 连接**上限（Chrome/Firefox 均 Won't fix） | ⚠️ 章节未提及该平台约束（见 F6）——Caddy 默认 h2 场景可解，但"用户既有反代为 HTTP/1.1"或本地直连开发模式会触顶 |

## 四、关键发现（按严重度排序）

### F1（Critical）：决策 #3 "tokio::spawn 全量替换是无行为变更的同义改写"在 setup 路径会 panic

**位置**：行 2504（Critical Decisions #3）、行 2535（抽取顺序第 3 步"机械替换（现状已混用，语义相同）"）、行 2883（Coherence 表"tokio::spawn 替换安全性：通过"）。

**事实链**：
1. tauri 2.11.5 源码（app.rs `make_run_event_loop_callback`）：`RuntimeRunEvent::Ready => { if let Err(e) = setup(&mut self) { panic!(…) } }`——**setup 闭包在主线程事件循环回调中同步执行，没有任何 runtime 上下文包裹**（无 block_on、无 Handle::enter）。
2. EgoSync lib.rs:42 起 setup 闭包直接调用 4 个内含 `tauri::async_runtime::spawn` 的**同步**入口：
   - `scheduler::spawn_scheduler`（lib.rs:369 → scheduler.rs:322 `pub fn` → :323 spawn）
   - `task_deadline_watch::spawn_hourly_watch`（lib.rs:295 → :86 `pub fn` → :87 spawn）
   - `task_protection_watch::spawn_hourly_watch`（lib.rs:299 → :75 `pub fn` → :76 spawn）
   - `companion_dispatch::register_stream_mirror`（lib.rs:345 → :694 `pub fn` → :700/702 两处 spawn）
3. `tauri::async_runtime::spawn` 经全局单例 RUNTIME 句柄可从**任意线程**调用；裸 `tokio::spawn` 在 runtime 上下文外调用会 panic（tokio 契约："must be called from the context of a Tokio 1.x runtime"）。
4. 章节引以为据的"scheduler.rs 已在用裸 tokio::spawn"恰恰只证明**已运行任务内部的嵌套 spawn**安全——scheduler.rs:406–590 那 5 处全部位于 :323 派生出的任务体内；入口 spawn（:323）本身就在同步上下文。
5. server 侧因 `#[tokio::main]` 的 block_on 建立了线程级 runtime 上下文而无恙——**问题只在桌面宿主，且恰好打在"桌面零回归"这条自称硬边界上**。

**建议修正**：
- 决策 #3 措辞从"无行为变更的同义改写/机械替换"改为"需注入运行时句柄的等价替换"；
- "宿主差异只允许存在于三处 trait 接缝"（2494）应改为**四处**：新增"任务派生接缝"——engine 的同步入口（spawn_scheduler/spawn_hourly_watch/register_stream_mirror 一类）接收宿主注入的 `tokio::runtime::Handle`（桌面壳传 `tauri::async_runtime::handle()`，server 传当前 handle）；
- Coherence 表"tokio::spawn 替换安全性：通过"需降级为"有条件通过（需 Handle 注入接缝）"；
- 抽取顺序第 3 步补充验收：桌面启动路径（setup → 四个同步入口）不 panic 的 e2e 断言。

### F2（Major）：耦合分类遗漏第四类"事件监听（AppHandle.listen）"，EngineEvents trait 无法承载

**位置**：行 2437（三类收敛：事件发射/任务派生/托管状态）、行 2503（决策 #2：trait 只有 `emit(event, payload)`）、行 2490（WS 否决论据"既有引擎事件全部单向（Rust→UI）"）。

**事实**：companion_dispatch.rs:707 `app_handle.listen("llm:stream", …)`、companion_snapshot.rs:796 与 :803 两处 `app_handle.listen`——Rust 侧存在对引擎事件的**消费**，且这两个文件都在要迁入 engine 的 16 个耦合 service 名单内。emit-only 的 EngineEvents trait 没有 subscribe 面，这两处迁移无处落缝；"既有引擎事件全部单向（Rust→UI）"也不精确（存在 Rust→Rust 的事件镜像，供手机伴侣流式转发）。

**建议修正**：二选一并显式裁决——(a) EngineEvents 增加 `subscribe(event, callback) -> ListenerId` 面（SseEventBus 的 broadcast 天然可复用为内部订阅源）；(b) 裁决 companion_dispatch/companion_snapshot 留桌面壳（云端 paired_devices 闲置与此自洽），并在 16 个迁移 service 名单中剔除、登记理由。同时修正"全部单向（Rust→UI）"的措辞。

### F3（Minor）：Caddy "需 flush_interval -1" 断言过强、机制不实

**位置**：行 2562（②技术要点）、行 2812（Caddyfile 结构注释）、行 2908（Gap #2"实测"）。

**事实**：Caddy 官方文档明示：响应带 `Content-Type: text/event-stream` 时 flush_interval **被忽略、响应自动逐写刷新**；axum 的 `Sse` 响应正是该 Content-Type。因此 SSE 反代**不需要**显式 `flush_interval -1`（配置了也无害，属冗余保险）。Gap 清单"起来后实测"实际上已有对冲，但正文三处把"需要"当硬约束陈述，属于"未经实测就写死平台细节"的典型。

**建议修正**：三处措辞改为"默认即不缓冲（text/event-stream 自动逐写刷新）；flush_interval -1 作为冗余保险可保留"；Gap #2 从"验证 -1 是否必要"改为"验证 SSE 经反代不断流（回归性验证）"。

### F4（Minor-Major）：云端 SecretStore env 方案的示例键名与真实 api_key_ref 格式不符

**位置**：行 2587（④ env→文件查找链）、行 2617（compose 示例 `EGOSYNC_SECRET_llm_main=...`）、行 2741（Format Patterns"EGOSYNC_ 前缀大写"）。

**事实**：llm_config.rs:119–120：`id = uuid::Uuid::new_v4()`，`api_key_ref = format!("llm_{}_api_key", id)`——引用键是**运行时生成的 UUID 命名**。`llm_main` 不匹配任何真实 api_key_ref 模式；用户不可能预知 UUID 来预置 env 变量。云端用户经 UI 创建 LLM 配置必然落到 `/data/secrets.json`（save 路径），env 只对**固定名** secret（如未来的 EGOSYNC_TOKEN 以外的引导性密钥）有意义。另 `EGOSYNC_SECRET_llm_main` 与自身"前缀大写"命名规则自相矛盾。

**建议修正**：④ 中 env 适配器定位收窄为"固定名 secret 的引导通道"，明示用户创建的 LLM Key 只经 secrets.json；compose 示例删除该行或改用真实格式说明；补一句 api_key_ref 命名规则（`llm_{uuid}_api_key`）作为 server 适配器键匹配的依据。

### F5（Minor）：数字与清单偏差集合

1. **chat.rs 托管状态实为六组**：`OpencodeMcpScopeLock`（chat.rs:16，lib.rs:68 manage）被"五组"清单漏掉——ChatSessionRegistry（registry.rs"五组会话状态"）设计口径少算一组，实施时会发现多一个无处安放的状态。建议清单改为六组或显式裁决该锁的去处。
2. **async_runtime::spawn 数量**：决策写"全量替换"，耦合清单写"约 8 处"；service 层 9 处/8 文件（清单漏 companion_dispatch 2 处），全代码库 19 处/13 文件。若"全量"按字面执行（含 commands/lib.rs），工作量被低估约 2.4 倍；若范围仅限 service 层，应写明"commands 与 lib.rs 的 spawn 留在桌面壳不替换"。
3. **app.manage 15 处 → 实为 16 处**；**mock 测试文件"约 10 个" → 实为 14 个**（Gap 清单）。
4. **决策 #7 论据覆盖面**：`last_triggered_map`/`last_briefing_trigger_date`/`last_review_trigger_week` 三处确为内存态；但"大石头保护"现状**已是 DB 持久化**（bigrock_protection.rs:149 读表中 `last_reminded_at`）。Process Patterns"统一走 scheduler_triggers、不允许多套去重机制并存"（行 2754）实际是要**替换一个已持久化的机制**（含数据迁移），不是全新增——工作量与风险口径应如实登记。

### F6（Minor）：SSE/通知的两处平台事实遗漏或失准

1. **HTTP/1.1 下 SSE 每浏览器每域名 6 连接上限**（MDN 明示，Chrome/Firefox 均 Won't fix）：裁决 A 支持"多浏览器=多窗口"，经 Caddy h2 无碍，但章节既允许"文档化对接用户既有反代"（2562），就应登记该约束（反代为 HTTP/1.1 时多标签 + XHR 会触顶），并在文档中给出"反代需支持 HTTP/2 或限制并发标签"的提示。
2. **"浏览器 Notification API 仅前台可用"（2652）方向写反**：桌面后台标签页（文档存活时）仍可发通知；真正的限制是 iOS Safari 普通浏览无 Notification API（16.4+ 仅限加主屏 PWA）。结论（V1 应用内通知、Service Worker 化延后）不受影响，但论据应改写为"页面关闭即无通知（无 SW 推送）+ iOS Safari 基本不可用"。

### F7（Minor）：PRD FR-46 的 "令牌/passkey" 中 passkey 被静默丢弃

PRD 690/693 行写"单用户访问凭据（令牌/passkey）"（或然措辞），章节决策 #5 只裁决令牌路径，未显式登记 passkey 的取舍。可接受（PRD 是 either-or），但按本章"显式暴露冲突"的自设标准应补一行裁决记录（例：passkey/WebAuthn 延后至 V2，理由：单用户 + 自托管下令牌足够，WebAuthn 对 iOS Safari 反而是加分项后置）。

### F8（Minor）：能力门控 "web-ok：其余" 不完备

- 7 个 `companion_*` command（配对/NSD/二维码等）按"web-ok：其余"会被 server 路由，但云端实例不连伴侣中继、paired_devices 闲置——这些 command 在云端必然失败或无意义，应列入 desktop-only 或显式裁决（与 F2 的 companion 服务归属联动）。
- `data_export` 本体内嵌 rfd 目录选取（data.rs:43）——按"4 个 rfd 对话框类 command"口径，**data_export 本身是 desktop-only**；而 ② 又把 `/api/export` 设计为云端下载流（2554）。两者并不矛盾（云端走 HTTP 流落点），但注册表里 data_export 的 capability 归属需要显式裁决为"同名双实现"或"参数化落点"，否则"从注册表源头排除 desktop-only"（2507）会把云端导出一并排掉。

### F9（备忘）：伴侣章节一处旧断言未列入登记清单

伴侣章节 1962 行："工作循环仅桌面运行时执行 | FR-10 | **不存在云端代替桌面跑循环**"。该句在决策 #19（云脑单源、FR-47 云端常驻）语境下语义已过时。裁决 C 只登记了 Deferred"云端数据同步"与"托管"一词的语义演化；Validation Issues Addressed（2917–2919）声称"无静默矛盾"，但这一处属于未登记的语义漂移。建议追加到裁决 C 的旧章节登记清单（仍遵守"只追加不回改"原则）。

---

## 五、内部一致性检查结果

- 章内自洽性总体良好：限流口径（2561 vs 2635）、TLS 否决与 FR-46"内置或反代终结"（2488 vs PRD 695）、裁决 B 与 FR-48"不触发任何数据合并"（2684 vs PRD 715）、⑨ 停机诚实代价与 PRD FR-47 ASSUMPTION——均一致。
- 已发现的章内矛盾：F4（env 示例键名 vs 自身命名规则）；F1（决策 #3 vs 抽取顺序第 3 步与 Coherence "通过"）；F2（三类耦合枚举 vs 16 文件迁移名单含 listen 耦合的文件）。
- 与 PRD §4.15 的冲突：仅 F7（passkey 静默丢弃）与 F8（FR-45"核心体验功能集"下 capability 门控口径不完备）；其余 FR-44/45/47/48 验收条款均被覆盖且语义一致。
- 与伴侣章节的冲突：仅 F9（1962 行未登记）。

## 六、建议修正清单（按优先级）

| 优先级 | 修正项 | 落点 |
|---|---|---|
| P0 | 决策 #3 改为"Handle 注入接缝"方案；"三处 trait 接缝"改四处；Coherence 表降级为有条件通过 | 2504、2494、2883、2535 |
| P1 | EngineEvents 补 subscribe 面，或裁决 companion_dispatch/companion_snapshot 留桌面壳；修正"全部单向"措辞 | 2503、2490、2437 |
| P1 | chat.rs 状态清单改六组（补 OpencodeMcpScopeLock）或裁决其去向 | 2428、2528、2796 |
| P2 | Caddy flush_interval 三处措辞降级为"冗余保险"；Gap #2 改回归性验证 | 2562、2812、2908 |
| P2 | SecretStore env 定位收窄、compose 示例键名对齐 `llm_{uuid}_api_key` 实际格式 | 2587、2617 |
| P2 | companion_*/data_export 的 capability 归属显式裁决 | 2507 |
| P3 | 数字勘误：manage 16 处、mock 14 个、spawn 全量口径写明范围；bigrock 去重"替换而非新增"登记 | 2527、2528、2754、2909 |
| P3 | SSE 6 连接约束（HTTP/1.1 反代场景）入约束表；通知论据改写；passkey 取舍登记；1962 行入旧章节登记清单 | 2426–2433、2652、2506、2695 |

## 七、结语

本章作为增量架构章节的事实密度显著高于常见水准——耦合点、状态清单、command 数量、依赖版本几乎全部经得起复核，平台选型（SSE + Cookie + TZ + WAL 停机备份）的方向性判断也全部与一手来源吻合。唯二动摇结论的是 **F1（spawn 替换的 setup 路径 panic 风险，直接威胁桌面零回归）** 与 **F2（EngineEvents trait 缺监听面）**——两者都可在文档层一次修正，不动摇"同一引擎、双宿主"的总体范式。修正 F1/F2 后本章节可进入实施。

---

### 附：证据来源索引

- 仓库源码：`egosync-app/src-tauri/src/{lib.rs, commands/chat.rs, commands/data.rs, commands/skill.rs, services/scheduler.rs, services/agent_engine.rs, services/companion_dispatch.rs, services/companion_snapshot.rs, services/task_deadline_watch.rs, services/task_protection_watch.rs, services/llm_config.rs, services/bigrock_protection.rs, models/settings.rs}`、`egosync-app/src-tauri/Cargo.toml`、`relay-server/Cargo.toml`、`egosync-app/src/services/skillService.ts`、`egosync-app/node_modules/@tauri-apps/api/core.js`
- 一手文档：[axum 0.8.9 SSE](https://docs.rs/axum/latest/axum/response/sse/index.html) · [MDN EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource) · [Caddy reverse_proxy（flush_interval）](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) · [chrono 0.4.45 tz_info/timezone.rs 源码](https://docs.rs/chrono/latest/src/chrono/offset/local/tz_info/timezone.rs.html) · [tauri 2.11.5 async_runtime](https://docs.rs/tauri/latest/tauri/async_runtime/index.html) · [tauri v2.11.5 app.rs 源码](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-v2.11.5/crates/tauri/src/app.rs) · [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html) · [caniuse: EventSource](https://caniuse.com/eventsource) · [SQLite How To Corrupt](https://www.sqlite.org/howtocorrupt.html)
- 项目文档：`_bmad-output/planning-artifacts/prd-egosync.md` §4.15（663–716 行）；`_bmad-output/planning-artifacts/architecture.md` 伴侣章节（1926–2390 行）
