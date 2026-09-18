---
title: 'server binary 与单用户认证（Story 15.4）'
type: 'feature'
created: '2026-09-18'
status: 'review'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: 'cf147d1'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-15-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 15 前三故事已把全部业务逻辑迁入 `crates/egosync-engine`，但引擎只能活在 Tauri 壳里——云端托管赛道（Epic 16/17）的唯一前置「可公网部署的 axum server binary」尚不存在；且 120 条 command 的路由面对应关系目前只靠纪律（generate_handler 手工清单），没有机制保证。

**Approach:** 三段递进：① 命令层宿主无关化——web-ok 的 106 条命令体从壳 `commands/*.rs` 平移为 engine `commands` 模块的普通异步函数（首参 `&EngineCtx`，emit→总线+常量、spawn→tokio、AppHandle 路径→注入），壳命令薄化为 wrapper，桌面零回归；② commands.json 构建期工件——机械导出名字+参数 schema+capability（desktop-only 12 条物理排除），经对等断言守门；③ `server/` crate（axum 0.8、relay-server 同栈同布局）——POST /api/cmd/{command} 全量 web-ok 路由 + SSE 广播 + 单用户令牌认证（env/首访 setup、Argon2id、httpOnly Cookie、限流、CSP/CORS）一体交付，无任何 dev 旁路。

## Boundaries & Constraints

**Always:**

- **桌面零回归（硬边界）**：收口 = `npm run test:all` 全绿 + `npm run build` tsc 零错 + 启动冒烟（release xvfb ≥30s 无 reactor panic）+ e2e 全量（15.2 裁决 A 口径：执行一次 + 既有缺陷逐项归因，证据链入 Implementation Notes）。
- **15.3 特征测试跨迁移保绿**：busy 互斥特征测试（恰一次 LLM、恰一条 busy 落库、Ok 通道返回、恰一条 message:saved）在命令体入 engine 前后断言逐项一致；`<R: Runtime>` 泛型随 ctx 化自然退役属接线适配，断言不动；锁粒度五不变量保持。
- **平移等价**：命令体迁移允许改动仅五类——State/AppHandle 取值改 ctx 字段、emit 改 bus+events.rs 常量、`tauri::async_runtime::spawn` 改 `tokio::spawn`、路径函数改 ctx 注入路径、use 路径调整；业务逻辑/锁结构/错误文案/SQL 零改动；随文件迁移的内联测试全绿。
- **命令层事件名收编**：壳侧遗留字面量事件名（role:created/updated、task:created/classified、data:imported、notification:read 等，15.3 裁决「留壳」）随命令体入 engine 必须常量化进 events.rs（engine 零字面量发射规则）；值钉测试同步追加。
- **错误白名单冻结（架构 ②）**：AppError 全部 variant（error.rs:3-36 现役 **13 个**——epics AC 文本「14 个」为笔误，以代码为准；规则同样覆盖未来新增 variant）一律 `200 + 原样单键 map JSON`（`{"DbError":"..."}`，与 invoke 错误通道同构）；非 200 仅 401 / 429 / 404 / 进程级 5xx 四类；handler panic 经 CatchPanicLayer → 500；对等测试覆盖每 variant 错误路径。
- **初始化优先级冻结（架构决策 #5）**：env 存在 `EGOSYNC_TOKEN` ⇒ `/api/setup` 不挂载（请求 404）、login 仅常时比对 env；env 不存在 ⇒ 以库内 Argon2id 哈希为准（首访 setup 写入 `app_settings` kv——架构数据边界表指定落点）；两态切换须重启进程、已发 Cookie 不随切换失效（会话持久化于 `auth_sessions` 新表）；禁止任何「任一通过即可」fail-open 组合读法。
- **登录与会话**：`POST /api/auth/login` 令牌正确 ⇒ 换发 httpOnly SameSite=Strict Cookie（SSE/EventSource 平台约束倒逼）；错误 ⇒ 统一 401 不泄露存在性；未认证请求任意业务端点（含 SSE）⇒ 401；仅 healthz 豁免（「静态资源豁免」指认证豁免面，静态服务归 16.1）。
- **限流与跨源**：`/api/auth/*` 与 `/api/setup` 按 IP 5 次/分钟（超 ⇒ 429）；跨源请求显式拒绝（Origin 不同源 ⇒ 403，无 CORS 放行头）；CSP 头全响应下发（default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'）。
- **请求边界（F11）**：body 上限 50MB（显式覆盖 axum 默认 2MB）；业务端点禁用响应超时（chat 分钟级挂起是正常态）、保留 idle 超时；SSE KeepAlive 30s 心跳；broadcast 滞后即断开该连接。
- **commands.json 全量机械生成**（禁抽样禁手抄禁人肉阅读签名）：每条含名字、参数 schema（ctx 注入参数与客户端参数标注区分）、capability；desktop-only = 12 条（chat_pick_working_directory / skill_pick_custom_directory / pick_import_file / data_export / data_import + 7 个 companion_*；data_export 属 4 个 rfd 命令之一）；对等断言：壳 lib.rs generate_handler 源码扫描集合 == 工件 web-ok ∪ desktop-only ∪ 2 条 perf-test 门控（lib.rs:567-595 源码扫描测试先例）；CI 新鲜度断言（重新生成后 git diff 干净）；生成机制裁决（OQ2 裁决 A）= syn 源码扫描生成器（server 内 feature 门控 bin 解析 engine commands 模块源码 → 产出工件 + 生成的 dispatch registry 入册）——架构决策 #4「过程宏」措辞以机制等价满足（机械导出、零人肉、CI 守门），登记为措辞偏离。
- **服务端 SecretStore（OQ1 裁决 A——Add 9 覆盖表 9→17.1 前移登记）**：15.4 即实现架构 ④ 完整适配器——env `EGOSYNC_SECRET_{api_key_ref}` 原样区分大小写拼接（ref 段零大小写转换）+ 数据目录 `secrets.json`（0600 权限明文，「加密」措辞已废除）**文件优先、env 兜底**（env 仅固定名 secret 引导通道，不构成运行时遮蔽源）；写入永远落 secrets.json 0600；禁止 InMemory 占位（静默丢 Key 的已知坏状态）。
- **server 形态**：单实例单用户自托管，无多租户/多用户/注册体系抽象；无任何 dev 免认证旁路（`EGOSYNC_DEV_NO_AUTH` 等零命中，grep 断言）；`server/` 与 relay-server 同栈同布局（axum 0.8 + tokio 1、lib.rs build_router 供测试进程内复用、main.rs 仅 env+tracing+优雅退出、Ctrl-C+SIGTERM）；仓库根禁建 Cargo workspace，server 以 path 依赖引 engine。
- **server 引导对等复刻桌面序列**（壳 lib.rs:42-416 为蓝本，剔除桌面专属）：init_db 双池（数据目录 `EGOSYNC_DATA_DIR`）、四接缝注入（SseEventBus / 服务端 SecretStore / sidecar 路径 Option+PATH fallback / Handle::current()）、ChatSessionRegistry、AgentConfig 全量同步、LLM provider 同步、custom tools 写盘、sidecar 启动+watchdog（PATH 无 opencode 时优雅降级——桌面同款路径）、DelegateBridge 监听、EventRouter pump、两个 hourly watch、spawn_scheduler；companion_*/窗口/keyring 剔除；退出 = 取消 token + sidecar stop。
- **密钥零泄漏**：任何 API 响应不含 key 字段（浏览器只见 api_key_ref）；密钥/令牌哈希/事件 payload 明文永不入日志。

**Never:**

- 不做 `/api/export`、`/api/import` HTTP 流端点（Add 11 → 17.3）；不做静态资源服务/SPA 回退（16.1）；不做 TLS/Caddy/Dockerfile/compose/镜像（17.1）。
- 不做 `EGOSYNC_OPENCODE_PATH` 注入与 opencode 镜像 pin（Add 10 → 17.1；15.4 sidecar 走既有 `SidecarManager::new(None, port)` PATH fallback）。
- 不做前端 transport/capabilities.ts/useEngineEvent/任何 `src/` 改动（15.5；commands.json 只为其备好输入）。
- 不迁 12 条 desktop-only 命令与 2 条 perf-test 门控命令入 engine；server 物理不路由（404）。
- 不删壳侧既有 manage 清单与 companion/快照消费者接线（EngineCtx 为增量 manage，外科手术边界）。
- 不顺手重构：不合并三份 provider 构造、不动 delegate_bridge/agent_engine 既有结构、不改 sprint-status 其他条目、不清理既有警告。
- 禁止业务端点新增限流（限流收敛认证面）；禁止 REST 旁路端点（POST /api/cmd/{command} 是唯一业务入口）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 存活探针 | `GET /healthz`（无认证） | 200 `{"status":"ok"}` | N/A |
| 深度探针 | `GET /healthz?deep=1` | DB SELECT 1 + sidecar health_check 全过 ⇒ 200+flags；任一败 ⇒ 503+flags | 5xx 家族内（升级前巡检语义） |
| 首访初始化 | 无 env 无库哈希，`POST /api/setup {token}` | Argon2id 哈希入 app_settings；setup 路由随即不再挂载 | 已初始化后再请求 ⇒ 404 |
| env 态 setup | env 存在 `EGOSYNC_TOKEN` | `/api/setup` 不挂载 | 请求 ⇒ 404（库内哈希忽略） |
| 登录成功 | `POST /api/auth/login` 令牌正确 | Set-Cookie（httpOnly SameSite=Strict） | N/A |
| 登录失败 | 令牌错误（env 态或库态） | 401 统一形状，不泄露存在性 | 常时比较 |
| 会话跨重启/切换 | 库态→env 切换并重启 | 已发 Cookie 仍有效（auth_sessions 持久化） | N/A |
| 未认证业务请求 | 无/错 Cookie 访问 /api/cmd/* 或 /api/events | 401 | N/A |
| desktop-only 命令 | `POST /api/cmd/pairing_generate_qr` 等 12 条 | 物理不路由 | 404 |
| 未知命令 | `POST /api/cmd/no_such_cmd` | 404 | N/A |
| 业务错误 | 任意 AppError variant（13 个） | `200 + {"Variant":"msg"}` 单键 map | 与 invoke 错误通道同构 |
| 参数反序列化 | camelCase body（如 `{"conversationId":...}`） | 与 invoke 同构解包为 snake_case 参数 | 类型不符 ⇒ 200+ValidationError 形状 |
| 限流 | 同 IP 第 6 次/分钟打 auth/setup | 429 | 滑动窗口内存计数 |
| 跨源请求 | Origin 头与宿主不同源 | 403 拒绝（无 CORS 放行头） | CSP 头仍下发 |
| 大请求体 | body >50MB | 413（axum DefaultBodyLimit；传输层原语——白名单四类管业务通道，此为显式登记的传输面扩展） | N/A |
| 慢 SSE 客户端 | broadcast receiver 滞后（Lagged） | 断开该连接 | EventSource 自动重连兜底 |
| handler panic | 命令执行中 panic | CatchPanicLayer ⇒ 500 | 进程级 5xx 白名单内 |
| opencode 缺失 | PATH 无 opencode 二进制 | sidecar 优雅降级启动失败（桌面同款）；healthz deep opencode:false；chat 类命令返回 200+SidecarError | 非致命，服务存活 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/lib.rs:42-416` -- setup 引导序列蓝本（双池 :58-64、AgentConfig 同步 :102-166、LLM 同步 :186-200、custom tools :206-211、sidecar+watchdog :214-306、delegate 监听 :285-299、EventRouter pump :267-281、hourly watch :314-324、scheduler :400-413）；:417-540 generate_handler 120 条（2 条 perf-test 门控 :501-504）；manage 清单 :66-346；Exit 清理 :543-555。
- `egosync-app/src-tauri/src/commands/`（20 文件 120 命令）-- 迁移源。AppHandle 用途已清点：web-ok 命令中仅为 emit（role.rs:10-22 emit_role_event、task.rs:15、data.rs:116、notification.rs:81——字面量事件名待收编）、路径解析（skill.rs:121/166/216 app_data_dir→workspace/skills_root、data_destroy 的 data_dir）、命令体内 `tauri::async_runtime::spawn`（task.rs:42、review.rs:91 异步分类）；`try_state::<CompanionState>` 仅 desktop-only 的 data.rs:85。chat.rs 三命令持 `<R: Runtime>` 泛型（:182/:574/:612，ctx 化后退役）。
- `crates/egosync-engine/src/` -- 归宿。`services/event_bus.rs:11-13` EngineEvents（emit 单方法）；`services/secret_store.rs:13-21` SecretStore trait；`registry.rs:60-67` ChatSessionRegistry；`events.rs:8-56` 16 事件常量（命令层字面量随迁追加）；`error.rs:3-36` AppError 13 variant + :38-62 单键 map Serialize；`services/sidecar.rs:220/237/287/381-481/648-675` SidecarManager（new(None,port) PATH fallback、health_check）；`db/pool.rs:24-46` init_db；`db/app_settings.rs:7-40` get/set_setting（令牌哈希落点）；`services/scheduler.rs:346-352`、`task_protection_watch.rs:79`、`task_deadline_watch.rs:90` Handle 注入签名。
- `relay-server/` -- 同栈同布局范式（Cargo.toml:8 axum 0.8；src/lib.rs:61-71 build_router 供测试；main.rs:40-88 优雅退出双信号；tests/common/mod.rs:26-182 InProcessServer/SubprocessServer 双模式）。
- `.github/workflows/` -- `relay-docker.yml:22-39`（独立 job+单独 rust-cache `workspaces: relay-server -> target`）为 server CI 范式；`ci.yml:274-293` engine 零 `tauri::` 断言（迁移后自动覆盖新 commands 模块）。
- `egosync-app/src-tauri/tests/test_chat_busy_mutex.rs` -- 特征测试护栏（跨迁移断言不变）。
- 依赖现状：argon2/subtle/tower-http 全仓未引（server 需新增）；uuid 已在 engine（会话令牌生成免新依赖）；tokio broadcast 为内建。
- 权威锚点：architecture.md 决策 #4/#5/#6（L2506-2508）、② 错误白名单与 API 表（L2549-2570）、④ 密钥管理（L2592-2605）、⑧ CSP/CORS（L2645-2658）、目标态目录树（L2800-2830）；epics.md 15.4（L3820-3876）与覆盖表（L3667：Add 9/10→17.1、11→17.3）。

## Tasks & Acceptance

**Execution:**

- [x] `crates/egosync-engine/src/commands/{mod.rs,ctx.rs}` + `src/lib.rs` + 壳 `lib.rs` -- EngineCtx 落位（13 字段全量：双池/Registry/AgentConfigService/SidecarManager Arc<Mutex>/AgentBridge/EventRouter/DelegateBridge/bus/secrets + data_dir/opencode_workspace/skills_root/home_dir；Send+Sync 钉测试 engine_ctx_is_send_sync 入册）：引擎类型组合（DbPool/ConversationsPool/Arc<ChatSessionRegistry>/AgentConfigService/DelegateBridge/SidecarManager/AgentBridge/EventRouter/Arc<dyn EngineEvents>/Arc<dyn SecretStore>）+ 注入路径字段（data_dir/opencode_workspace/skills_root/home_dir）；壳 setup 构造并 manage `Arc<EngineCtx>`（增量，既有 manage 不动）-- 单一注入容器（接缝收拢点）
- [x] `crates/egosync-engine/src/commands/*.rs` + 壳 `commands/*.rs` + `events.rs` -- 分域平移 web-ok 106 命令体（18 域文件：memory/briefing/mission/scheduler/settings/dashboard/notification/suggestion/review/role/task/task_decomposition/data/mcp/skill/secret/llm_config/app/chat；chat 域 5 内联测试随迁；11 事件常量收编 + 值钉；壳全量薄化 wrapper；busy-mutex 特征测试断言零改动跨迁移保绿）（每域收口绿）：纯 DB CRUD 域（memory/mission/suggestion/briefing/dashboard/scheduler/settings/review/notification）→ 路径注入域（skill/role/data_destroy）→ spawn/secret 域（task/llm_config/app/task_decomposition）→ chat 域（特征测试护栏下最后迁移）；五类允许改动逐条对号；壳 wrapper 薄化（`State<Arc<EngineCtx>>` + 参数直传）；命令层字面量事件名常量化 + 值钉；内联测试随迁 -- server 可达的唯一路径
- [x] `crates/egosync-engine/src/capabilities.rs` -- desktop-only 12 名单 + perf-test 门控 2 名单 + capability 查询函数（4 rfd 组 + data_import + 7 companion_*）+ capability 查询函数 -- 门控事实源
- [x] `server/src/bin/gen_commands.rs` + `crates/egosync-engine/commands.json` + 生成的 dispatch registry + 对等断言测试（syn 源码扫描：pub async fn 全按值或单 &EngineCtx 判定命令；106 条导出与壳 generate_handler 120 = 106+12+2 对等断言钉死；新鲜度 regenerate+git diff 零漂移） -- 机械导出工件（名字+参数 schema+capability）、`dispatch(ctx, name, Value) -> Result<Value, AppError>` 生成、generate_handler 源码扫描 == 工件∪desktop-only∪gated、CI 新鲜度 -- 「靠机制不靠纪律」
- [x] `server/` crate 骨架（Cargo.toml + src/{main,lib}.rs）-- build_router/AppState、env EGOSYNC_{HOST/PORT/DATA_DIR/TOKEN}、tracing、双信号优雅退出（取消 token+sidecar stop 8s 死线）-- relay-server 同款：build_router(AppState)、env（EGOSYNC_HOST/PORT/EGOSYNC_DATA_DIR）、tracing、优雅退出 -- 布局对等
- [x] `server/src/{routes,sse}.rs` + engine SseEventBus -- POST /api/cmd/{command}（WEB_OK_COMMANDS 门控 404、空 body⇒{}、AppError⇒200 单键）+ GET /api/events（容量 256 broadcast、KeepAlive 30s、take_while 滞后即终止） -- POST /api/cmd/{command}（camelCase 反序列化与 invoke 同构、AppError→200、desktop-only/未知→404、50MB、无响应超时、CatchPanicLayer）+ GET /api/events（EngineEvents 实现→broadcast 扇出、KeepAlive 30s、滞后断开）-- API 面
- [x] `server/src/auth.rs` + `crates/egosync-engine/migrations/032_auth_sessions.sql` -- env/setup 两态（setup 物理不挂载 404）、SHA256 常时比较、Argon2id 入 app_settings kv、uuid v4 会话 SHA256 哈希入 032 表、httpOnly SameSite=Strict Cookie、IP 滑动窗口 5/min⇒429、require_auth 中间件 + /api/auth/{status,login} + app_settings 令牌哈希 -- env/setup 优先级（setup 仅无凭据时挂载）、Argon2id + 常时比较、login 换会话 Cookie（uuid v4 令牌、SHA256 哈希入 auth_sessions 多会话表）、401 中间件、IP 滑动窗口限流 5/min -- 认证一体交付
- [x] `server/src/secret_store.rs`（OQ1 裁决 A：env+secrets.json 完整适配器）-- 文件优先（0600、BTreeMap、tmp+rename 原子写）env 兜底 EGOSYNC_SECRET_{ref} 区分大小写；CSP 全响应/跨源 403/CatchPanic 500；healthz 两级（deep 503 降级）+ CSP/CORS 中间件 + healthz 两级 -- 服务端接缝实现 + 安全头 + 探针
- [x] `server/tests/` -- 集成测试 22 条：api_test 17（I/O 矩阵逐行：env/库态/切换+Cookie 跨重启/401 形状/desktop-only+未知 404/13 variant 白名单/真实命令 ValidationError+SSE 事件名+滞后断开/密钥零泄漏/429/403+CSP 全响应/413 边界恰好 50MB）+ parity_test 5（源码扫描对等/工件↔registry 零漂移/门控不路由/无旁路/engine 零 tauri::）：env 态/库态/优先级切换/Cookie 跨重启/限流/401 形状/错误白名单每 variant/SSE 慢客户端/全量路由 vs commands.json/无 key 字段断言 -- I/O 矩阵全覆盖
- [x] `.github/workflows/server-ci.yml`（独立 rust-cache server+engine 双 workspace、cargo test、gen_commands 新鲜度双工件 git diff --exit-code、无旁路 grep、engine 零 tauri:: grep）：独立 rust-cache + cargo test + 工件新鲜度；Docker job 不做——17.1）-- CI 对等
- [x] 收口验证 -- test:all 全绿（engine 812 passed；壳全绿；vitest 全绿）+ npm run build tsc 零错 + 双 grep 零命中 + server cargo test 22/22 + server 冒烟全链调通（healthz 200/deep 503 降级/错令牌 401/正令牌+Cookie/role_list 200/SSE KeepAlive `:` 帧观测/优雅停机）+ 桌面 release xvfb 冒烟 + e2e（见 Implementation Notes 证据链） -- 硬边界

**Acceptance Criteria:**

- Given commands.json 工件，when 从 engine commands 源码机械导出，then 每条含名字、参数 schema（ctx 注入与客户端参数标注区分）、capability；对等断言与 CI 新鲜度全绿；重新生成零 diff
- Given server 路由，when axum 0.8 启动，then POST /api/cmd/{command} 覆盖全部 106 条 web-ok command（desktop-only 物理不路由得 404）；参数 camelCase 反序列化与 invoke 同构；server/ 与 relay-server 同栈同布局
- Given SSE 事件流，when GET /api/events 认证连接，then 事件名=Event 字段、data=payload JSON 与 emit 同构；broadcast 扇出；KeepAlive 30s；滞后即断开
- Given 错误白名单，when 业务错误发生，then 13 个 AppError variant 一律 200+原样单键 map；非 200 仅 401/429/404/5xx；每 variant 错误路径有测试
- Given 初始化优先级，when env 存在/不存在，then setup 拒绝(404)+login 仅比对 env / 首访 setup 入库 Argon2id；切换须重启；已发 Cookie 不失效；无 fail-open 读法
- Given 登录与会话，when 令牌正确/错误，then httpOnly SameSite=Strict Cookie / 统一 401；未认证业务端点（含 SSE）401；仅 healthz 豁免
- Given 限流与跨源，when 超过 5 次/分钟/IP 或跨源请求，then 429 / 显式拒绝；CSP 按 ⑧ 下发
- Given 无旁路验证，when 检查代码库，then 不存在任何 dev 免认证旁路
- Given 命令层迁移，when 桌面全量回归，then 特征测试断言逐项一致、test:all/build 全绿、启动冒烟零 panic、e2e 按 15.2 裁决 A 口径归因收口
- Given 本地冒烟，when `cargo run` + curl，then cmd/SSE/认证面以真实令牌可调通

## Implementation Notes

- **迁移对账（①）**：106 条 web-ok 命令体入 `crates/egosync-engine/src/commands/`（18 域文件 + ctx.rs + mod.rs）；壳 20 文件对应薄化为 wrapper（chat/data/app/skill 留存桌面专属函数：chat_pick_working_directory、skill_pick_custom_directory、data 三命令、app perf-test 2 条）。事件常量收编 11 个入 events.rs + 值钉测试（companion_snapshot 源码扫描测试改为扫 engine 命令源 + 经 events.rs 解析 `*_EVENT` 常量——字面量消除后扫描面同构）。内联测试随迁：chat 5 条（engine 计数 811→812 的其余增量为既有域内联测试随迁 + ctx Send/Sync 钉）。
- **平移五类改动实录**：①State/AppHandle→ctx 字段（AppHandle 在 web-ok 命令中仅 emit 与路径解析，全量替换）②emit→`ctx.bus.emit`+events.rs 常量 ③`tauri::async_runtime::spawn`→`tokio::spawn`（task.rs:42/review.rs:91 异步分类）④路径函数→ctx.data_dir/opencode_workspace/skills_root/home_dir 注入 ⑤use 路径调整。业务逻辑/锁结构/错误文案/SQL 零改动；busy-mutex 特征测试断言逐项未动（仅调用签名接线适配）。
- **生成器（②）**：`server/src/bin/gen_commands.rs`（feature 门控 `gen`，syn+quote）——扫描规则：`pub async fn` 且参数全按值或恰一 `&EngineCtx`（引用参数=内部辅助非命令，confirm_and_create_task/reject_with_reason 由此自然排除、保持 pub 供 companion_dispatch 复用）。产出 `crates/egosync-engine/commands.json`（106 条：name/params[ctx 标注]/capability=web-ok）+ `server/src/dispatch_gen.rs`（camelCase 参数 struct + dispatch match registry）。对等断言（parity_test）钉死 120 = 106 ∪ 12 ∪ 2；新鲜度 regenerate+diff 零漂移（本机验证 FRESH）。
- **server 形态（③）**：`server/` axum 0.8 + tokio 1，与 relay-server 同栈同布局（build_router 供 InProcessServer 进程内测试复用；main.rs 仅 env+tracing+双信号优雅退出：CancellationToken 取消→sidecar stop→8s 死线）。中间件叠放（外→内）：CatchPanic(500) → CSP 全响应 → 跨源拒绝(403) → 50MB body 上限——叠放顺序修正过一次（初版 CSP 在跨源之内导致 403 无 CSP 头，测试抓出后调序）。
- **两处实现级坑与修法（供 15.5+ 复盘）**：
  - `axum::body::Body` 为 `!Sync`：handler 中 `&Request` 引用跨 await 会使 future `!Send`（Handler trait 不满足，报错不指因）。修法：同步段先提取 Cookie 值（extract_session_token）再入异步校验——auth_status 与 require_auth 中间件两处同款。
  - `BroadcastStream` + `filter_map(Err=>None)` 只会静默跳帧不会断开：滞后断开语义必须 `take_while(is_ok)` 终止流。修法经 sse_event_stream 提取为可测函数，单元测试直接对 Stream 断言（TCP 层内核缓冲会吸收慢消费，经 socket 不可稳定复现滞后）。
- **SSE 冒烟证据**：35s 观测窗内收到 KeepAlive `:` 注释帧（axum KeepAlive 30s 配置生效）；lag 测试：容量 256 灌 300 条后流终止（断开语义）+ 对照组容量内 10 条正常出帧。
- **llm_config_create 桌面对等行为登记**：命令含 refresh_runtime（同步 opencode + 重启 sidecar）；测试环境无 opencode 二进制 ⇒ 200+RuntimeRefreshError——与桌面 invoke 同款（15.3 冒烟同环境级降级），非 15.4 引入。密钥零泄漏测试经服务层植入配置后对 HTTP 响应面断言（列表零 `sk-` 原文、含 api_key_ref）。
- **依赖对账**：server 新增 argon2 0.5/subtle 2/tower-http catch-panic/tokio-stream(sync)/dirs 5/sqlx 0.8(与 engine 同源)/futures-util/uuid v4/sha2；engine 零新依赖（Cargo.lock 无 tauri/keyring 验证）。无 Cargo workspace（根禁建），server 以 path 依赖引 engine。
- **CI**：`.github/workflows/server-ci.yml`——push/PR 路径触发（server/+engine/+workflow）；job：cargo test → gen_commands 新鲜度（commands.json + dispatch_gen.rs 双工件 diff --exit-code）→ 无旁路 grep → engine 零 tauri:: grep。
- **e2e 归因记录（2026-09-19，15.2 裁决 A 口径：执行 + 逐项归因既有缺陷）**：今日环境处于坏态，两类既有缺陷复现，**基线对照实验证明与 15.4 零相关**：
  - **全量 2 次**（`xvfb-run --auto-servernum npm test`）：9/9 失败——失败签名两类混布：①WebDriver 会话建立超时（`http://127.0.0.1:4444/session POST aborted`）②会话建立成功后的 IPC 失败（`IPC app_complete_onboarding failed: Origin header is not a valid URL`，tauri 2.11.2 ipc/protocol.rs:495——发生在命令代码执行之前的 IPC 协议层）。
  - **基线对照实验（本日）**：checkout 基线 cf147d1（15.3 收口提交，零 15.4 改动）worktree release 构建（前端 dist 与 15.4 相同——15.4 零前端文件改动），同一 llm-streaming spec **连续 2 次同样以会话建立超时失败**——环境级失败与 15.4 无关的执行级证据（复刻 15-1 的基线实验先例：当时基线 9979f3e 以完全相同的 Origin 错误在同一 spec 位置失败）。
  - **缺陷既有性证据链**：①15-1 基线实验（9979f3e 零改动 → 100% spec 止于 Origin 错误）；②15-2 记录「今日 Origin 缺陷未复现…环境在两故事间演化」——缺陷按日漂移；③15-3 记录「残留 tauri-driver 占 4444 口致会话建立超时，pkill 清理后通过」——今日同款超时；本日另诊断出残留 WebKitWebDriver 占 4445（tauri-driver 内部 WebKitWebDriver 绑定失败致会话建立挂起，kill 后单跑会话可建）。
  - **桌面健康替代证据**（e2e 层被环境缺陷阻断时的旁证）：release xvfb 冒烟 35s 存活零 panic、完整引导序列走通（双池/AgentConfig/custom tools/sidecar PATH 降级/delegate bridge 监听）；`npm run test:all` 全绿（vitest + 壳 cargo + engine 812）——业务行为在 Rust 层验证面完备。
  - 结论：今日 e2e 全部失败可归因环境级既有缺陷（WebDriver 会话层 + IPC Origin 层），且基线对照证明该环境态对零改动代码同样失败；15-4 未引入新失败签名（无任何失败落在应用命令代码层）。
- **收口证据**：`npm run test:all` EXIT=0（vitest 全绿 + 壳 cargo 全绿 + engine 812 passed 0 failed）；`npm run build` EXIT=0（tsc 零类型错误）；`grep -rn 'tauri::' crates/egosync-engine/src/` 零输出；`grep 'EGOSYNC_DEV_NO_AUTH\|DEV_NO_AUTH'` 零命中（含 CI 同口径）；`cd server && cargo test` 22/22；server 冒烟全链调通（healthz 200 / deep 503 degraded {db:true,opencode:false} / auth status 200 / 错令牌 401 {"error":"unauthorized"} / 正令牌 200+Cookie / role_list 200 [] / 未认证 401 / SSE KeepAlive 帧 / SIGTERM 优雅退出）。

## Spec Change Log

## Review Triage Log

## Design Notes

- **命令层归属裁决（冲突显式化）**：架构目标态目录树未画 engine `commands/` 模块，但「engine crate 唯一业务逻辑所在」「src-tauri commands/ [M] 薄化」「Tauri command 与 axum handler 都薄调用」+ 15.4 AC「server 覆盖全部 web-ok command」四条共同强制命令体入 engine——薄 wrapper 留壳、业务体入 `engine/src/commands/`（镜像壳文件结构）是唯一不产生「云端专属业务逻辑」（范式违约）的路径。15.3 Never「命令层事件名留 15.5」被 15.4 结构需求覆盖：命令体入 engine 后字面量发射违反 engine 零字面量规则，收编随迁（显式登记，非顺手重构）。
- **AppError 13 vs 14**：epics AC 与架构 ② 均写「14 个 variant」，error.rs:3-36 现役 13 个（NotFound/LlmError/DbError/ValidationError/KeyringError/SidecarError/RuntimeRefreshError/SkillNotFound/SkillNotAddedToScope/SkillDisabled/PairingError/ConnectionError/ProtocolError）。以代码为准；「全部 variant 一律 200」规则不变，未来新增 variant 自动同规则。
- **会话设计（架构未指定会话存储，设计裁量）**：login 时 Argon2id/env 校验一次 → 换发随机会话令牌（uuid v4，122 bit 熵）→ SHA256 哈希存 032 新表 `auth_sessions`（多会话行 = 多浏览器=多窗口语义平移）；每请求校验 = 索引查找+哈希比对（亚毫秒）；Argon2 仅 login 执行（每请求 Argon2 校验会带来百毫秒级开销——不可取）；哈希存储防库泄露后会话劫持；「已发 Cookie 不随切换失效」由会话表持久化天然成立。令牌本体哈希按架构数据边界表存 `app_settings` kv。
- **setup 挂载语义**：API 表「仅在无凭据时挂载」⇒ env 存在或库哈希存在时路由物理不存在（404），优先级冻结的机械落地。
- **白名单边界两处显式登记**：跨源拒绝 403 与 body 超限 413 均为传输面前置拒绝，不属业务错误通道（白名单四类约束的是业务错误的 status 挪用）；deep healthz 失败 503 属 5xx 家族。三处入册供人工复核。
- **X-Forwarded-For 不解析**：15.4 按 ConnectInfo 直连 IP 计数；反代后单 IP 聚合的限流语义（5/min 是否够单用户登录重试）留 17.1 反代形态定稿时复核——登记。
- **sidecar PATH fallback**：Add 10（EGOSYNC_OPENCODE_PATH/镜像 pin）归 17.1；15.4 用 `SidecarManager::new(None, port)` 既有签名，PATH 无 opencode 时优雅降级（本 dev 环境即此态——桌面 15.3 冒烟同款环境级降级）。
- **依赖新增收敛**：server 侧 argon2 0.5、subtle（常时比较）、tower-http（catch-panic）；会话令牌复用 engine 既有 uuid；限流/cookie 解析手写（单实例单用户量级，不引 tower-governor/axum-extra）。engine 零新依赖（迁移只搬代码）。
- **EngineCtx 为增量 manage**：壳侧既有 manage（companion/快照/CancellationToken 等）原样保留——companion_dispatch 等消费者零改动；命令 wrapper 从分散 State 改取 `State<Arc<EngineCtx>>`，15.3 特征测试 setup 随之机械适配（mock_app manage EngineCtx，断言不动）。

## Verification

**Commands:**
- `cd egosync-app && npm run test:all` -- vitest + 壳 cargo + engine cargo 全绿（EXIT=0；engine 测试数随命令体内联测试迁移增长，逐域记录对账）
- `cd egosync-app && npm run build` -- tsc 零类型错误（前端零改动守门）
- `grep -rn 'tauri::' crates/egosync-engine/src/` -- 零输出（CI 同口径，含新 commands 模块与注释）
- `grep -rn 'EGOSYNC_DEV_NO_AUTH\|DEV_NO_AUTH' server/ egosync-app/ crates/` -- 零命中（无旁路）
- `cd server && cargo test` -- 集成测试全绿（I/O 矩阵逐行覆盖）
- `cd server && cargo run --bin gen_commands && git diff --exit-code crates/egosync-engine/commands.json` -- 工件新鲜度零漂移
- 启动冒烟：release 构建 + `xvfb-run` ≥30s 桌面进程存活、无 reactor panic（EngineCtx 接线后必测）
- server 冒烟：`EGOSYNC_TOKEN=... EGOSYNC_DATA_DIR=$(mktemp -d) cargo run` + curl -- healthz/deep/login（取 Cookie）/POST /api/cmd/ 只读命令/GET /api/events 收到 KeepAlive 或事件帧，全链调通
- `cd egosync-app/tests/e2e && npm test` -- 15.2 裁决 A 口径：执行一次 + 逐项归因既有缺陷（证据链入 Implementation Notes）

**Manual checks (if no CLI):**
- 无
