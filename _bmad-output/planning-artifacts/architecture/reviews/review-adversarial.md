# 对抗性架构评审 — 云端托管版增量章节（FR-44～FR-48）

- **评审对象**：`_bmad-output/planning-artifacts/architecture.md` L2391–2974（2026-09-17 追加）
- **镜头**：构造"两组各自严格遵守章节字面规则、却产出互不兼容实现"的分叉场景（A 组=桌面壳+引擎抽取 Epic 15；B 组=server binary+SSE+WEB Epic 16/17）
- **方法**：通读全章 + 对照代码验证（`src-tauri/src/{error.rs, lib.rs, commands/chat.rs, services/agent_engine.rs, services/llm_config.rs, services/secret_store.rs, services/companion_dispatch.rs, db/settings.rs, db/pool.rs}`，事件名/命令注册/时间源/密钥引用均以实际代码为据）
- **总体判定**：**PASS-WITH-FIXES** —— 五条硬边界与"同一引擎、双宿主"范式对"业务逻辑层"的分叉免疫是真实的；但章节对"契约形状层"（参数 schema、错误形状、事件清单绑定、secret 键映射、重连补齐协议）多处留白，其中 F1–F3 会在 Epic 15→16 交接面产生必然的集成分叉，必须收紧后才可立项。

---

## F1（致命）command 注册表只冻结"名字集合"，参数形状与 handler 语义无机制保证

**代码事实**：lib.rs `generate_handler!` 注册 126 个 command；Tauri command 签名混用 `State<'_, T>` 注入参数与客户端参数（如 `chat_send_message(streaming_state: State, request: ChatRequest)`）；章节裁决"桌面注册与 server 路由同源生成，对等测试断言两侧集合一致"。

**场景**：A 组从 generate_handler 抽出共享清单（capabilities.rs：名字 + desktop-only 标记，仅此而已）。B 组在 server 侧为每个 command **手写** axum handler：哪些参数来自注入、哪些来自请求体、invoke 的参数包装形态（`{"request": {...}}` 键名包装 vs 直接 struct body）、`#[serde(default)]` 语义——全部靠 B 组对 126 个签名的逐个人工阅读理解。"同源生成"只生成路由**名字**，生成不了 handler **身体**。

**分叉产物**：名字集合对等测试全绿；某个 command 在 web 端因参数包装形态不同而 422/deser 失败，桌面端正常。15.4 的"双通道黄金用例"只覆盖抽样 service 函数，未覆盖的 command 无防线。**更根本的洞**：对等测试代码规定放 `egosync-app/src/transport/`（vitest），但它必须同时看到 Rust 侧清单（engine capabilities.rs + server 路由 + 桌面 generate_handler）才能断言集合一致——vitest 如何看到 Rust 清单（构建期导出 JSON？debug command？）完全未裁决，两组各自手抄一份清单做断言也算"遵守"了章节字面。

**建议条款**：注册表条目必须携带每个 command 的参数 schema（由 Tauri command 签名经过程宏导出 JSON Schema，HTTP handler 据此驱动反序列化）；对等测试用例必须由注册表**全量生成**（禁止抽样），且注册表必须以构建期生成的 JSON 工件形式同时可被 vitest 与 server 测试消费——"断言两侧集合一致"的测试代码必须物理上能看到两侧。

## F2（致命）事件契约三处自相矛盾：逐字节等价 vs Value 签名改写；事件名常量与双侧字面量无绑定；测试无法枚举事件全集

**代码事实**：全部 emit 现传**强类型 payload**（`app_handle.emit("llm:stream", StreamPayload { ... })`，Tauri 在 emit 时序列化；agent_engine.rs 单文件 20+ 处）；事件名在 Rust 侧为散落字符串字面量（≥9 个唯一名），在 TS 侧为独立字面量（≥8 个，`useTauriEvent` 调用点），两份清单无任何共享源。

**场景**：章节同时要求 (a) "模块平移保持文件内容逐字节等价（import 路径除外）"与 (b) `EngineEvents::emit(event: &str, payload: serde_json::Value)`。把强类型 emit 改写为 Value 签名**必然改写每个发射点**——A 组必须违反 (a) 或 (b) 之一，二选一都合规却产物不同：选"保留字面量绕过常量"则 events.rs 的"事件名常量"成为与真实发射点漂移的死清单；选"改写发射点"则 20+ 处重写中一个 typo（`llm:stream` → 相邻变体）即可引入分叉。B 组 SSE 侧纯转发不受影响，但 15.4 "事件形状一致"测试需要事件全集清单来枚举——清单从哪来未定义，只能手挑黄金用例。

**分叉产物**：改写期事件名或 payload 字段笔误，若该事件不在桌面 e2e 覆盖内、不在黄金用例集内、TS 字面量未同步——三道防线全漏，且因 payload 为自由 `Value`，字段名笔误（`conversationId` vs `conversation_id`）不产生任何编译期错误。

**建议条款**：显式豁免规则三一次——事件发射点的"强类型→Value + 字面量→events.rs 常量"是唯一允许的机械改写，events.rs 常量是唯一发射源；TS 侧事件名与 payload 类型必须由构建期从 events.rs 生成（或对等测试断言：Rust 发射事件名集合 == TS 订阅集合 == 测试枚举集合，三者由同一导出清单驱动，禁止手抄）。

## F3（高）SecretStore 的 ref→存储键映射未冻结，章节自身三处互相矛盾；env 优先级使云端重录 key 被"合规地"遮蔽

**代码事实**：`api_key_ref = format!("llm_{}_api_key", id)`（llm_config.rs:120），keyring 键为 `(com.egosync.app, ref)`；章节 compose 示例写 `EGOSYNC_SECRET_llm_main`（与真实 ref 格式 `llm_{uuid}_api_key` 不符），Format Patterns 又写"环境变量 EGOSYNC_ 前缀大写"——{KEY} 段是否大写、原样拼接还是转换，三处文档互相矛盾。且 server 适配器为"env→文件查找链"（env 优先），而云端重录 key 走 `llm_config_update` → 运行时写 SecretStore → 只能落 secrets.json——env 中的旧值**永远遮蔽**文件新值。

**场景**：B 组按"前缀大写"实现 `EGOSYNC_SECRET_{ref.to_uppercase()}`；运维按 compose 示例配小写 env；桌面导出→云端导入的配置 ref 为 `llm_9f8e7d_api_key` → 查找 `EGOSYNC_SECRET_LLM_9F8E7D_API_KEY` 不存在 → 每次对话报 `KeyringError("未找到配置…的 API Key，请重新保存")`（task_classifier.rs:309 / mission_inferrer.rs:436 / llm_config.rs:218 三处各自拼错误）。用户"重录"后写入 secrets.json，仍被 env 遮蔽或因大小写规则查不到——重录表现为无效，且**无任何测试能发现**（旧值仍是合法 key，只是永远读不到新值）。

**分叉产物**：桌面→云端迁移用户全链路不可用；三种合规实现（原样拼接/大写/仅前缀大写）在同一章节下都成立。

**建议条款**：冻结映射规则为 `EGOSYNC_SECRET_{api_key_ref}` 原样区分大小写拼接（并修正 compose 示例为真实 ref 形态）；裁决写入语义——运行时 `save_secret` 更新 secrets.json 且**文档明示"配置了 env 的 ref 其 UI 重录不会生效"**（或规定 env 仅在文件无该键时兜底，写入永远落文件、读取文件优先）；导入完成后对每个 `api_key_ref` 探测 SecretStore 可达性并给出指明重录路径的错误。

## F4（高）AppError 契约未冻结：401/5xx 与 200 的分类线未画，对等测试清单不含错误路径

**代码事实**：`AppError` 的 serde 形状为单键 map `{"DbError": "..."}`（error.rs 手写 Serialize，14 个 variant）；章节只说"业务错误=200+AppError JSON（与 invoke 错误形状同构）——前端 transport 统一解包"。

**场景**：B 组实现 `IntoResponse for AppError` 时面临未裁决的分类题：`DbError` 算业务（200）还是 5xx？"HTTP 状态码承载传输层错误（401/429/5xx）"没有给出判定规则。axum 直觉（DbError→500、SidecarError→503）与章节字面（一切 AppError→200）都读得通。A 组的 Tauri 通道根本不存在 401/5xx 分支——"两侧错误路径同构"在 Tauri 侧无从对照，15.4 测试清单（command 集合/事件形状/无 key 字段）**不含错误形状断言**。

**分叉产物**：web 端数据库故障返回 500（transport 抛"网络错误"），桌面端同一故障返回 `{"DbError": "..."}`（UI 显示具体中文信息）；前端解包逻辑随 B 组对 14 个 variant 的逐个分类漂移，黄金用例只测成功路径测不出。

**建议条款**：冻结分类规则——全部 AppError variant 一律 200 + 原样 serde JSON，非 200 仅限四类：认证失败 401、限流 429、路由不存在 404、进程级故障 5xx；对等测试增加错误路径黄金用例（每个 variant 断言双通道解包所得错误对象逐字节同构）。

## F5（高）SSE 重连"全量补齐"的责任切分三问全空：哪个 command、谁触发、哪些可重放

**场景**：章节裁决"broadcast 滞后即断开 → EventSource 自动重连 + UI 经 command 全量补齐状态"，但没有单一状态补齐端点——前端须重放 N 个查询 command；EventSource 自动重连对应用层是**静默**的（onopen 再次触发），`useEngineEvent`/Transport 接口未定义 reconnect 信号，而 Tauri 分支永远不需要它；哪些 command 幂等可安全重放（误重放 `notification_ack` 类命令有副作用）未裁决。B 组（HttpTransport）认为 UI 层自己轮询 healthz 补齐；Epic 16 UI 组认为 transport 会广播重连事件——两边各写一半。

**分叉产物**：慢客户端被断开后 web 端界面停留在陈旧状态，无任何提示，直到用户手动刷新；或 UI 组与 transport 重复建设两套连接状态机（章节 ③ 已规定 healthz 探测属 transport 职责）。

**建议条款**：Transport 接口必须定义 `onConnectionStateChange`（HttpTransport 在 EventSource onopen-after-error 时触发重连信号）并规定补齐协议=重放冻结的只读 query command 白名单；补齐触发方为 transport 层而非 UI 层，写入双传输对等契约。

## F6（中）"桌面全绿"防线对 ChatSessionRegistry 并发语义是空集：busy 互斥与隐式协议无既有测试

**代码事实**：busy 语义 = 单把 `Mutex<HashSet>` 下 check-then-insert，busy 提示作为 assistant 消息**落库并以 Ok 返回**（chat.rs:301–309）；companion_dispatch.rs:514–521 通过 `msg.role != "user"` 这一**隐式协议**探测 busy（第二处 busy 路径文案还是另一个字符串）；grep 全仓无任何 busy 语义测试。

**场景**：A 组抽取 Registry 时把 check 与 insert 拆为两把锁（"细化锁粒度"）或把 busy 改为返回 `Err`（"更干净的错误语义"）——`npm run test:all` 与 e2e 全绿（无覆盖），companion 路径的 role 探测静默失效：busy 时伴侣收到提示消息被当作正常回复处理。

**分叉产物**：多标签并发流式时双重 LLM 调用（锁拆分后 check-then-insert 不再原子）或 companion 会话协议错乱——全部测试绿，纯生产期暴露。

**建议条款**：ChatSessionRegistry 抽取故事必须**先落特征测试再动代码**：并发双发仅一次 LLM 调用 + 恰一条 busy 落库消息 + Ok 返回值形状 + companion 的 role 探测协议；抽取后 busy 语义（落库消息经 Ok 通道返回）为冻结契约，禁止改为 Err 通道。

## F7（中）时间源清单未冻结：settings 时间戳（手写 UTC）与 scheduler（容器 Local）不同源，浏览器渲染时区未裁决

**代码事实**：`chrono_now_pub` 为 epoch 手算的 UTC RFC3339 串（`Z` 后缀，db/settings.rs:144–164）——settings 表/消息时间戳全是 UTC 字符串；scheduler 触发用 `Local::now()`（容器 TZ）。决策 #8 只覆盖 scheduler 平移，对"持久化时间戳=UTC"这一现状只字未提。

**场景**：桌面端前后端同机同时区，从未暴露；云端浏览器 TZ ≠ 容器 TZ（自托管者在海外 VPS + 国内访问是常态）——前端按浏览器本地渲染 UTC 串全部偏移；简报文本内日期为容器 Local 拼好的字符串，与前端格式化层的格式并存。另一合规分叉：A 组认为"容器有 TZ 了，chrono_now_pub 顺手改 Local 更一致"——无任何测试断言时间戳格式带 `Z`，改了全绿。`scheduler_triggers` 的 date+hhmm 键在用户修改 TZ 后跨时区错乱（同一天双触发或漏触发）。

**建议条款**：冻结时间源三分表——持久化时间戳一律 `chrono_now_pub` 的 UTC RFC3339（禁止改 Local）、调度判定一律容器 Local、前端渲染一律按浏览器 TZ 解析 UTC 串；`scheduler_triggers` 键含 TZ 偏移或文档明示"变更 TZ 后首个周期可能重复/跳过一次"。

## F8（中）EGOSYNC_TOKEN 与 /api/setup 并存的优先级、变更与失效语义未裁决

**场景**：决策 #5"env 预置**或**首访 setup"，未裁决：两者同时存在时 login 校验哪个（任一通过即可=fail-open，还是 env 独占）；"仅在无凭据时挂载"中 env 算不算凭据；用户改 env 值/删 env 重启后，已 setup 的哈希与 cookie 的失效关系。B 组的两种合规实现行为完全不同：宽容版（任一匹配即发 cookie）与严格版（env 存在 ⇒ setup 404 + login 只认 env）都符合章节字面。

**分叉产物**：运维从 compose 移除 env 后登录行为随实现漂移；安全审计面上"env 旧值仍可登录"的宽容实现是隐患。

**建议条款**：冻结优先级表——env 存在 ⇒ `/api/setup` 返回 404 且 login 仅常时比对 env、库内哈希忽略；env 不存在 ⇒ 以库内 Argon2id 哈希为准；两态切换须重启进程并文档明示 cookie 不随其失效。

## F9（低-中）前端 capabilities.ts 与 engine capabilities.rs 双清单无同步机制

**场景**：章节规定 UI 按 `transport.capabilities`（"来自共享清单"）门控，但 Rust 清单→TS 消费的物理通道（构建期生成/运行时 command 查询/手抄）未裁决。A 组给新 command 标 desktop-only；前端组手维护的 capabilities.ts 未同步 → web 端入口可见但 `POST /api/cmd/x` 404（与未知命令同响应，用户看到的是"网络错误"）。对等测试只断言 server 物理排除 desktop-only，不断言两份清单一致。

**建议条款**：capabilities.ts 必须由构建期从 capabilities.rs 生成（或 Transport 启动时经 command 拉取），禁止手写；对等测试增加"TS 能力清单 == 引擎注册表"断言。

## F10（低-中）secrets.json"加密文件"措辞与实现方案未定义

**场景**：④写"数据卷**加密**文件（/data/secrets.json，0600）"——0600 是权限不是加密；加密密钥来源（用户 passphrase？独立 env？硬编码？）零字。B 组要么明文存（违背"加密"字样）要么自造密钥派生方案（安全面失控）。

**建议条款**：二选一裁决并落死——(a) 删"加密"措辞，定义为"0600 权限保护的明文，安全边界与 env 同级"；或 (b) 冻结加密方案与密钥来源（如 `EGOSYNC_SECRET_MASTER_KEY`）。

## F11（低）长请求超时与请求体上限的双通道不对等

**场景**：`chat_send_message` POST 挂起至完成（LLM 分钟级）；axum `DefaultBodyLimit` 默认 2MB、反代默认写超时各异、Tauri invoke 无此限制。B 组不主动配置，则 web 端大 base64 附件 413、长回复被 Caddy 掐断，桌面端一切正常——功能级分叉且极难排查。

**建议条款**：冻结 server body 上限（如 50MB，写进注册表契约）、业务端点禁用响应超时（仅保留 idle 超时）；Caddyfile 样例须含 `request_body max_size` 与无 write_timeout 配置（与 flush_interval -1 并列）。

---

## 已裁决、不重复报告（验证过确已堵住）

- migrate! 宏相对路径（Gap #4 已登记；补充：`db/conversations.rs:514` 的 `include_str!("../../migrations/...")` 是同类断点，建议并入该故事）
- opencode 三件并列 vs 并入 server 镜像（⑤已显式裁决）
- 伴侣"云端数据同步"Non-Goal 与"托管"语义演化（裁决 C 已登记）
- `skill-scope-updated` 前端→前端事件（已裁决浏览器进程内消化，不进传输契约）
- CORS/CSP/Cookie 属性/浏览器不落盘/限流/多浏览器广播语义/慢客户端断开策略（⑧与裁决 A 已覆盖）
- Caddy SSE flush_interval（Gap #2 已登记）

## 附注（不构成分叉，建议一句话登记）

- 决策 #7 的去重持久化是**引擎级**变更，桌面端同样获得持久去重——"桌面 FR-10 行为不变"的表述应加注"重启窗口内不再重复触发"这一边缘行为变化，避免 A 组收口时误判为回归。

## 汇总

| # | 严重度 | 一句话收紧条款 |
|---|--------|----------------|
| F1 | 致命 | 注册表携带参数 schema，对等测试全量生成且物理可见两侧 |
| F2 | 致命 | events.rs 常量为唯一发射源，TS 事件名构建期生成，测试枚举同源 |
| F3 | 高 | 冻结 `EGOSYNC_SECRET_{api_key_ref}` 原样映射 + 写入/遮蔽语义 |
| F4 | 高 | 冻结 AppError 全 variant→200 + 非白名单不得非 200 + 错误路径黄金用例 |
| F5 | 高 | Transport 定义重连信号 + 只读补齐 command 白名单 |
| F6 | 中 | Registry 抽取先落 busy 互斥特征测试，返回通道为冻结契约 |
| F7 | 中 | 时间源三分表：持久化 UTC / 调度 Local / 渲染浏览器 TZ |
| F8 | 中 | env 与 setup 优先级表落死，切换须重启 |
| F9 | 低-中 | capabilities.ts 构建期生成，测试断言双清单一致 |
| F10 | 低-中 | secrets.json 加密方案二选一裁决 |
| F11 | 低 | 冻结 body 上限与禁用业务端点响应超时 |
