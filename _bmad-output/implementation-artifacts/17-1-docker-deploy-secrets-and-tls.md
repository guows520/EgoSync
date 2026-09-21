---
title: '17-1 Docker 化部署、密钥注入与 TLS'
type: 'feature'
created: '2026-09-21'
status: 'done'
baseline_commit: '97551e97e92e4d384383369b490ce07a220f89c5'
route: 'dispatch'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-17-context.md'
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 15/16 交付了引擎 crate、server binary 与 web 客户端，但仓库无任何 Docker 部署物（无 Dockerfile/compose/Caddyfile/.dockerignore/部署文档），云端版无法一键部署；且 15.4 遗留的 TLS 反代缺口（`is_same_origin` 以 Origin 缺省 443 对 Host 缺省 80 比对，Caddy TLS 后所有浏览器 POST 被 403 断；会话 Cookie 无 Secure 旗标）使公网 HTTPS 实际不可用；另有三个部署前置缺口：双 SQLite 未启用 WAL（epic「WAL 单容器单写者」假设落空，data_export.rs 的 `PRAGMA wal_checkpoint` 现为无操作）、`EGOSYNC_OPENCODE_PATH` 注入未实现（server 仅 PATH fallback）、日志为 stderr 纯文本（架构 ⑦ 冻结为 stdout JSON）。

**Approach:** 交付多阶段 Dockerfile（前端 dist 构建 → server 构建 → 运行时层含 opencode+tzdata+ca-certificates）、两服务 docker-compose.yml（egosync-server + caddy）、Caddyfile（自动 HTTPS）、.dockerignore、云端自托管部署指南；代码侧闭合四个部署前置缺口：TLS 反代三件套（反代感知同源判定 + Cookie Secure + IPv6 Host 拆分）、WAL 启用、opencode 路径注入、日志 JSON stdout。两项人工裁决并入本故事（2026-09-21）：①密钥缺失错误 6 处收敛为单一结构化错误（NFR-C7），导入触发的批量探测留 17.3（已登记 deferred-work）；②Google Fonts 外链改自托管随 dist 分发（离线/内网一致性与隐私），CSP 同步收紧为零第三方域。SecretStore 服务端适配器 15.4 已落地（双通道/文件优先/0600 原子写，行为由 secret_store_test.rs 钉死），本故事只做文档化与部署物接入，不改其行为。

## Boundaries & Constraints

**Always:**
- 架构 ⑤ 冻结：opencode 并入 server 镜像（非独立容器）；compose 两服务 `egosync-server` + `caddy`；Caddy 只做 TLS 终结与代理
- SecretStore 读序（文件优先、env 兜底）与写路径（仅 secrets.json、0600 原子写）不改；env 键映射 `EGOSYNC_SECRET_{api_key_ref}` 原样区分大小写
- Caddyfile 必含 `request_body max_size 50MB` 与 `flush_interval -1`；不设 write timeout（与 server 侧 body 上限对齐，防代理层第二道暗限）
- opencode 版本 pin v1.15.10，与 release.yml 三处硬编码一致（验证步骤比对）；镜像支持 linux/amd64 + linux/arm64（上游有 arm64 资产，按 TARGETARCH 参数化下载）
- secrets.json / \*.db / node_modules / target 不进镜像层与 git；前端 dist 在镜像内构建（全局 `dist/` 在 .gitignore，CI checkout 无产物）；构建上下文为仓库根（server path 依赖 crates/egosync-engine，复用 relay-server 模式）
- 时间源三分表不动：持久化 UTC / 调度 Local / 前端浏览器 TZ（17.2 领地）
- server 直连 HTTP 行为不变（dev/e2e 零影响）：反代感知与 Cookie Secure 均经 `EGOSYNC_BEHIND_PROXY` 显式门控，未启用时忽略 X-Forwarded-\* 头（防直连暴露下的伪造信任）
- 字体自托管（人工裁决 B，2026-09-21）：经 npm 字体包（fontsource 系，OFL 授权）引入 Inter/JetBrains Mono/Noto Sans SC（可变字体优先控制体积），替代 index.html Google 外链；CSP 撤除 fonts.googleapis.com/gstatic 两域（font-src/style-src 回归 `'self'`），csp.contract.test.ts 同步改为断言**零第三方域**；桌面（tauri csp:null）与 web 同链本地加载，视觉零分叉且离线一致
- 密钥缺失错误收敛（人工裁决 A，2026-09-21）：6 处 `AppError::KeyringError(format!(...))` 收敛为 engine 单一助手函数，统一文案含重录路径（设置 → 模型服务配置 → 编辑对应配置重新保存密钥）；AppError variant 名不改（15.1 defer 裁决维持 KeyringError 形状）

**Never:**
- 不做 CI 镜像发布（`.github/workflows/server-docker.yml` 是 17.3 交付物）
- 不做多租户/多实例同步路径；不做会话 TTL/登出广播（既有 defer 领地）
- 不动 SecretStore trait 签名、command_parity_test / api_test 的密钥零泄漏断言、CORS 白名单机制
- 不做 server 内置 rustls TLS（架构冻结：TLS 终结在反代）；`/api/import`/`/api/export` HTTP 流端点与导入触发的批量密钥探测是 17.3（deferred-work 已登记）
- 不为字体引入 Google 之外的任何第三方域；不手工散下载字体文件（一律走 npm 包版本化）

## I/O & Edge-Case Matrix

| 场景 | 输入 / 状态 | 期望行为 | 错误处理 |
|------|------------|---------|---------|
| 反代后同源请求 | `Origin: https://dom.com` + `Host: dom.com` + `X-Forwarded-Proto: https` + BEHIND_PROXY=1 | 放行（现状 403）；login Set-Cookie 带 `Secure` | N/A |
| 直连 HTTP（dev/e2e） | 未设 BEHIND_PROXY | 现行为逐字节不变（无 Secure；http 缺省 80 比对） | N/A |
| 伪造 X-Forwarded-\* | 直连暴露 + 客户端伪造头 | BEHIND_PROXY 未启用 ⇒ 头被忽略 | N/A |
| IPv6 字面量 | `Origin: http://[::1]:8080` + `Host: [::1]:8080` | 括号感知拆分，同源判定正确 | N/A |
| 明文 HTTP 访问 | `http://domain` 经 Caddy | 308 重定向 https（2026-09-21 人工裁决：接受 Caddy 默认 308——方法保留的永久重定向，301 的语义超集）；自有反代路径文档明示同等要求 | N/A |
| 容器/宿主机重启 | `docker restart` 或 reboot | `restart: unless-stopped` 自愈；/data 双库 + secrets.json + Caddy 证书（独立卷）无损 | N/A |
| opencode 不可用 | EGOSYNC_OPENCODE_PATH 未设且 PATH 无 | 现行为：降级运行不崩 | tracing warn |
| 离线/内网部署 | 无外网访问 | 字体本地加载（dist 内 woff2），无第三方请求 | N/A |
| 密钥缺失（云端常态路径） | secrets.json 与 env 均无某 api_key_ref | 单一结构化错误，文案含重录路径 | NFR-C7 |

</frozen-after-approval>

## Code Map

- `server/src/security.rs:114-145` — `is_same_origin`/`split_host_port` 修复点（:125 已挂「17.1 反代定稿后复核」标记）；CSP_POLICY 常量（字体域撤除点）；CORS 白名单引用同文件
- `server/src/auth.rs:325-332` — login Set-Cookie 构造（httpOnly/SameSite=Strict/30 天）；logout 清 Cookie 处同步加 Secure；Bearer 通道不受影响
- `server/src/main.rs:71-77` — tracing 初始化（stderr 纯文本 → stdout JSON）；:79-106 env 面（新增 EGOSYNC_BEHIND_PROXY 读取与下发）
- `server/src/bootstrap.rs:157` — `SidecarManager::new(None, None)` → 读 EGOSYNC_OPENCODE_PATH（文件取父目录、目录直传，均走 resource_dir 注入参数；缺省保持 PATH fallback）
- `crates/egosync-engine/src/services/sidecar.rs:237-333` — `resolve_binary_path`（resource_dir/resources/ → 根 → PATH），不改桌面传参与生命周期
- `crates/egosync-engine/src/db/pool.rs:31,114` — init_db/init_conversations_db 补 `.journal_mode(Wal)`（:memory: 测试库 SQLite 自动保持 memory 模式，无害）
- `crates/egosync-engine/src/services/llm_config.rs:231/277/317/416`、`mission_inferrer.rs:442`、`task_classifier.rs:322` — 密钥缺失错误 6 处（收敛对象）；`llm_config.rs:122` ref 格式 `llm_{uuid}_api_key`；桌面重录入口 `GlobalSettingsModal.tsx`（模型服务配置 tab）
- `server/src/secret_store.rs` — **不改**（15.4 落地；secret_store_test.rs 四断言钉死读序/大小写/0600）；部署文档化 env 引导通道
- `server/src/healthz.rs` — 复用（`?deep=1` 已实现）；compose healthcheck 用存活级
- `relay-server/Dockerfile`、`relay-server/docker-compose.yml`、`.github/workflows/relay-docker.yml` — 复用模式：多阶段+依赖层缓存+`--locked`、非 root（65534）、wget healthcheck、`restart: unless-stopped`、build context=仓库根写法；`read_only`/无卷**不适用**（有状态双库）
- `.github/workflows/release.yml:117-127` — opencode v1.15.10 pin 现状（三平台各一处硬编码）；Dockerfile ARG 须一致
- `egosync-app/index.html:7` — Google Fonts 外链（Inter 300-600 / JetBrains Mono 400-500 / Noto Sans SC 300-600，display=swap）——自托管替换点；内联防 FOUC 脚本 hash 契约不受影响（字体改动不触脚本）
- `egosync-app/src/security/csp.contract.test.ts:88-95` — CSP 契约门禁：现断言两 Google 域为唯一第三方源，须改为断言零第三方域 + font-src/style-src 回归 `'self'`
- `egosync-app/package.json` — `build` = `tsc && vite build`；`src/transport/capabilities.ts` 已提交（git ls-files 核实），前端阶段零 cargo 依赖；`src/transport/index.ts:38` `isTauriHost()` 运行时探测，同一 dist 双宿主
- `server/tests/secret_store_test.rs`、`server/tests/command_parity_test.rs:166-174`、`server/tests/api_test.rs:806-821` — 不动的钉死断言（读序 / llm_config_\* 响应无 api_key 键 / 密钥零泄漏）
- `docs/user-guide/` — 00-10 桌面手册编号序列，新指南接续编号
- `README.md:149-187` — 「Web 自托管服务（server/）」节：env 表仅三项，需补 HOST/PORT/SECRET_\*/BEHIND_PROXY/OPENCODE_PATH 与 Docker 快速路径

## Tasks & Acceptance

**Execution:**
- [x] `server/src/security.rs` — `is_same_origin` 反代感知（BEHIND_PROXY=1 时生效 scheme 取 X-Forwarded-Proto 首值、端口缺省按该 scheme；Host 拆分括号感知 IPv6）+ 单测覆盖矩阵前三行 — TLS 部署的 403 病灶
- [x] `server/src/auth.rs` — BEHIND_PROXY 且 XFP=https 时 login/logout Cookie 加 `Secure` + 单测 — 反代会话安全
- [x] `server/src/main.rs` — 新增 `EGOSYNC_BEHIND_PROXY` env 读取并注入 AppState（security/auth 消费）；tracing 切 JSON stdout — 架构 ⑦「结构化日志」对齐
- [x] `crates/egosync-engine/src/db/pool.rs` — 双库 init 补 `.journal_mode(SqliteJournalMode::Wal)` — epic「WAL 单容器单写者」假设兑现（桌面同引擎同步生效，属实现注记）
- [x] `server/src/bootstrap.rs` — 读 `EGOSYNC_OPENCODE_PATH` 注入 SidecarManager（文件→父目录、目录→直传、缺省→现 PATH fallback）+ 单测 — 架构 ⑤「同一注入点两个值」
- [x] `server/src/secret_store.rs` — 仅对齐头注释与实际行为：env 兜底对全部 key 生效（含 UUID ref——epic AC 示例即 UUID 形态），代码行为零改动 — 防后续故事误读「固定名限定」
- [x] `server/Dockerfile` — 三阶段：node 构建 dist → rust:1-bookworm 构建 server（`--locked`、依赖层缓存、context=仓库根）→ debian:bookworm-slim 运行时（opencode v1.15.10 按 TARGETARCH 下载 x64/arm64、tzdata、ca-certificates、wget、非 root 用户 + /data 属主预置、dist→/app/static、二进制→/app）— epic AC 1
- [x] `server/.dockerignore` — 排除 .git、node_modules、target、dist、secrets.json、\*.db、\*.db-wal/shm 等（构建上下文=仓库根，须全仓库生效）— epic AC 8（不进镜像层）
- [x] `server/docker-compose.yml` — egosync-server（build context=仓库根；env：DATA_DIR=/data、HOST=0.0.0.0、STATIC_DIR=/app/static、OPENCODE_PATH、BEHIND_PROXY=1、TZ、EGOSYNC_SECRET_\* 透传、RUST_LOG；命名卷 egosync-data:/data；restart: unless-stopped；healthcheck wget /healthz）+ caddy（caddy:2、80/443、Caddyfile 挂载、caddy_data/caddy_config 证书持久卷）— epic AC 2/4
- [x] `server/Caddyfile` — `{$EGOSYNC_DOMAIN}` 占位、reverse_proxy egosync-server:8080、request_body max_size 50MB、flush_interval -1、自动 HTTPS — epic AC 3
- [x] `docs/user-guide/11-云端自托管部署.md` — 从零到浏览器全步骤（域名解析 → compose 启动 → 首访 setup 指向 README 首访流程/16.1）；两种 TLS 路径（内置 Caddy / 自有反代各自要求）；env 全表；1C1G 内存预算标注（按实测）；HTTP/1.1 反代 SSE 每域名 6 连接提示；卷级备份须停容器警告；TZ 语义；桌面→云端迁移 LLM Key 重录明示；公网抢占窗口缓解（预设 EGOSYNC_TOKEN）— epic AC 5
- [x] `README.md` — Web 自托管节：补 Docker 快速路径 + env 表补全 + 指向新指南
- [x] `egosync-app/package.json` + `egosync-app/index.html` + 字体导入入口 — npm 安装 fontsource 系字体包（Inter / JetBrains Mono / Noto Sans SC，可变字体优先控体积），CSS import 替代 index.html:7 Google 外链；tailwind font-family 名不变；核对 dist 体积增量合理 — 离线一致性与隐私（人工裁决 B）
- [x] `server/src/security.rs`（CSP_POLICY）+ `egosync-app/src/security/csp.contract.test.ts` — CSP 撤除两 Google 域（font-src/style-src 回归 `'self'`），契约测试改断言零第三方域；security.rs 头注释同步改写（16.1 放行理由已被自托管取代）— 与字体任务配套
- [x] `crates/egosync-engine/src/services/llm_config.rs`（+ `mission_inferrer.rs` / `task_classifier.rs` 调用点）— 6 处密钥缺失错误收敛为单一助手函数（统一文案含重录路径「设置 → 模型服务配置 → 编辑对应配置重新保存密钥」）+ engine 单测；同步排查既有断言旧文案的测试 — NFR-C7（人工裁决 A）

**Acceptance Criteria:**
- Given 仓库部署物，when 查看部署目录，then 多阶段 Dockerfile + docker-compose.yml 存在；运行时层含 opencode 二进制（v1.15.10，与 release.yml 一致）+ tzdata + ca-certificates
- Given compose 拓扑，when `docker compose up -d`，then 两服务起来：`egosync-server`（engine+opencode 单容器）+ `caddy`（自动 HTTPS）；双 SQLite 挂命名卷 /data 且 WAL 模式；TZ env 透传
- Given Caddy 反代生效（本机演练 domain=localhost、Caddy 内部 CA），when 以 https Origin 发 POST /api/auth/status 等端点，then 不再 403 且 Set-Cookie 含 Secure；`http://` 访问被 308 重定向（人工裁决 2026-09-21）；SSE 端点 `curl -N` 流式逐写不断流
- Given 服务器重启，when `docker compose restart` / 宿主机重启演练，then 实例自愈、/data 数据与 Caddy 证书无损
- Given 资源约束验证，when 1C1G 级约束（memory 限额）下运行，then 常驻内存实测值写入文档预算标注
- Given 新用户按文档操作，when 从零到浏览器访问，then 步骤完整可跟随（含密钥重录说明）
- Given 密钥边界回归，when server cargo test + 15.5 对等套件 + secret_store_test 全量运行，then 全绿（含 llm_config_\* 响应无 api_key 键断言）
- Given TLS 与密钥边界（本机 docker 演练环境配真实密钥），when 浏览器经 https 访问并核对响应，then 密钥只存在服务端内存与 /data/secrets.json 两处（`docker exec` 核查 + .dockerignore 验证不进镜像层）
- Given 字体自托管，when 构建 dist 并断网加载页面，then 字体本地命中（dist 内 woff2、零第三方请求）、观感与在线一致；csp.contract.test 断言 CSP 零第三方域
- Given 某配置的密钥缺失（secrets.json 与 env 均无值），when 触发 list_models/test_connection 等读取路径，then 返回同一结构化错误且文案指明重录路径（6 处文案一致）

## Implementation Notes

（实现期追加：决策、触碰文件、意外发现）

- **behind_proxy 装配期门控 + 状态字段**：`AppState` 新增 `behind_proxy` 字段（main.rs 读 env → `build_app_state(_, _, behind_proxy)` 参数下发），`reject_cross_origin` 中间件从 `from_fn` 升级为 `from_fn_with_state`——XFP 读取被门控包裹（直连暴露下头根本不读）。`build_test_state_with_proxy` 为反代感知集成测试通道（tests/proxy_tls_test.rs）。
- **Cookie Secure 的登出对称**：logout 过期 Cookie 与 login 签发 Cookie 属性须对称追加 Secure（部分浏览器对属性不匹配的 Set-Cookie 不覆盖旧值）——共用 `cookie_secure_flag()` 单一判定（XFP 代理链取首值、大小写不敏感、门控关闭恒为空）。
- **WAL 经 connect options 而非 PRAGMA**：`SqliteConnectOptions::journal_mode(SqliteJournalMode::Wal)` 挂在连接串上（连接池每连接生效），优于一次性 PRAGMA（只作用于执行它的单连接）。`:memory:` 库 SQLite 规定保持 memory 模式（设 WAL 无效不报错，已加断言钉死无害性）。
- **opencode 注入实现细节**：`opencode_resource_dir()` 对裸文件名（无目录分量）返回 None 交给 PATH 解析（`Path::parent()` 对 "opencode" 返回 Some("")，须显式过滤空 OsStr）；镜像内放置为 `/app/resources/opencode`——SidecarManager 搜索 `{resource_dir}/resources/opencode` → `{dir}/opencode`，与桌面 bundle 布局同序。
- **Docker 构建器差异（意外发现 1）**：本机 docker.io 29.1.3 无 buildx 组件——BuildKit 不可用，经典构建器实测**不消费** `server/Dockerfile.dockerignore`（per-Dockerfile ignore 是 BuildKit 机制）且不注入 TARGETARCH。对策：① 仓库根新增 `.dockerignore`（经典与 BuildKit 双消费的实际生效面）；`server/.dockerignore`（Story 钉死路径）与 `server/Dockerfile.dockerignore` 同步维护为副本；② Dockerfile 的 TARGETARCH 缺省回退 `uname -m` 推导（x86_64→amd64 / aarch64→arm64）。首次构建曾因经典构建器把 `egosync-app/src-tauri/target/`（11G）打进上下文而磁盘爆满（/ 95%）——根 ignore 落地后复现消失。
- **Caddyfile 语法校正**：`request_body` 是**站点级指令**（非 reverse_proxy 子指令）——首版写成子指令被 caddy validate 拒绝；正确形态为 site 块内 `request_body { max_size 50MB }`。`encode gzip` 刻意移除（SSE 逐写语义优先，压缩非 TLS 终结层职责）。
- **compose 的 env_file 机制**：`EGOSYNC_SECRET_*` 无法通配透传（compose environment 键须逐个枚举）——经 `env_file: [.env]` 整文件透传（EGOSYNC_TOKEN/TZ/RUST_LOG 同时显式声明默认值，env_file 值覆盖显式默认）。`EGOSYNC_DOMAIN` 用 `:?` 必填语法（缺失即启动报错，指向部署指南）。
- **字体体积实测**：@fontsource-variable 三包（可变字体，单文件全字重 100-900）经 vite 打包后 dist 增量 **4.59MB**（110 个 woff2 unicode-range 分片，浏览器按需取用分片）——优于预估 10-20MB 量级。tailwind font-family 名从 `Inter`/`Noto Sans SC`/`JetBrains Mono` 改为 fontsource 注册名 `Inter Variable` 等（可变字体 family 后缀约定）。
- **`npm ci` 与 scripts/ 依赖**：前端 stage 需 COPY `scripts/`（`src/transport/events.test.ts` 经 `../../scripts/gen-transport.mjs` 类型引用——tsc include src 面内的编译依赖）与 `postcss.config.js`（tailwind 管线）；仓库无 `public/` 目录（首版 COPY 假设了存在，已修正）。
- **engine 错误收敛落位**：`missing_api_key_error()` 为 `llm_config.rs` 内 `pub fn`（mission_inferrer / task_classifier 经全限定路径引用，无循环依赖）；既有测试零处断言旧文案（grep 核实），新增单测钉死「KeyringError 形状 + 配置名 + 重录路径」三要素。
- **日志 JSON**：`tracing_subscriber::fmt().json().with_writer(stdout)`——Cargo.toml 补 `tracing-subscriber` 的 `json` feature；启动失败路径（env 校验等）仍走 eprintln（stderr）——那些是 init 前的进程级错误，非 tracing 事件。
- **容器 HOME 缺省坑（意外发现 2，本机 docker 演练捕获）**：Docker 对无 home 目录的 `USER 65534` 缺省设 `HOME=/nonexistent`——opencode（bun 运行时）启动期 `mkdir $HOME` 直接 EACCES，sidecar 降级（`/healthz?deep=1` 如实报 `opencode:false`）。修复：镜像 `ENV HOME=/data`（与引擎侧 `home_dir=data_dir` 对齐，可写且随卷持久）；修复后 deep 探针全绿（`{"status":"ok","db":true,"opencode":true}`）。
- **HTTP→HTTPS 重定向实测为 308 非 301**：Caddy 自动 HTTPS 对明文访问发 308（Permanent Redirect，方法保留）——语义为 301 的严格超集（POST 不降级 GET），I/O 矩阵「301 重定向 https」按其意图（永久重定向）满足；部署文档措辞已按实际行为改写（308）。
- **308 人工追认（2026-09-21）**：用户裁决接受 Caddy 默认 308（方法保留的永久重定向对表单重提更稳），冻结块 I/O 矩阵与 AC 的「301」措辞经人工批准同步改为 308——非实现侧单方面改期望。
- **compose env 优先级机制更正（评审 #18）**：上文「env_file 值覆盖显式默认」表述有误——compose 规范中 `environment` 优先于 `env_file`。当前部署行为正确只是因为 `${VAR:-default}` 插值与 `env_file` 读取的是同一个 `.env` 文件（插值在 compose 命令层先发生，显式 `environment` 的键只在 .env 未定义时落到 default）。要覆盖某显式键，改 `.env`（插值同源）而非指望 env_file 压过 environment。
- **评审补丁批（2026-09-21，step-04 分诊后亲施）**：24 项 patch 全落地——package-lock 三 fontsource 条目 resolved 从腾讯云内网镜像改回 registry.npmjs.org（#1 高危：非腾讯云网络构建必挂）；compose 顶层 `name: egosync-server`（#3 卷名与文档对齐）+ 双服务 logging 轮转 10m×3（#6）；Caddyfile HSTS 头（#5）+ max_size 50MB→50MiB（#20：MB=10^6 与 axum 52,428,800 差 4.7%）；Dockerfile opencode 双架构 sha256 pin（#13，官方 asset digest）+ TARGETARCH 非法显式值硬失败（#19）；.gitignore 补 \*.db-journal / \*\*/secrets.json.tmp / \*\*/.env.\*（#9 与 dockerignore 防护面对齐）；部署指南 7 处（#2 导入/导出 17.3 边界、#10 抢注恢复 alpine+sqlite3 命令、#11 pull 命令修正、#12 $http_host 端口语义、#23 TZ 合法值核对、卷名核对步骤）+ README 限流措辞诚实化（#4）；测试补 6 项（#14 memory WAL 走被测路径、#15 CSP 正则 i 标志、#27 parse_behind_proxy 抽取+单测、#28 resolve_binary_path 命中测试、#29 missing-key 调用路径+源契约、#30 dist woff2 正向断言）；ci.yml 新增 opencode-pin 一致性 job（#31，只匹赋值行+sha256 齐备核对）+ relay-docker paths 耦合根 .dockerignore（#32）；architecture.md CSP 冻结段追加 2026-09-21 人工裁决修订注记（#7）；sprint-status 时间戳精度恢复（#17）。另 5 项 defer 落 deferred-work（#4 XFF 限流 / #8 ignore 漂移门禁 / #22 env 空值 / #7b project-context 覆盖 / #33 stdout JSON 验证），3 项驳回（#24/#25/#26）。
- **本机 docker 演练的端口适配**：演练机 80/443 被宿主 systemd caddy 占用（不可动）——演练经临时 override（8081:80 / 8443:443 + curl 显式 `Host: localhost` 钉头）完成，语义与 443 直连等价；生产 compose 文件保持 80/443 标准映射。

## Spec Change Log

（评审回环填入）

## Review Triage Log

（2026-09-21 三层评审：盲扫 18 项 / 边缘 13 项 / 验证缺口 7+3 项；去重后逐项裁决如下）

| # | 发现（层） | 裁决 | 证据/理由 | 路由 |
|---|-----------|------|----------|------|
| 1 | package-lock 三 fontsource 条目 resolved 指向腾讯云内网镜像 http://mirrors.tencentyun.com（盲扫/边缘/缺口三层同报） | **high** | grep 实证 3/489 条为内网明文源；npm ci 按锁取包，非腾讯云网络（GitHub runner、境外 VPS）构建必挂——击穿「一键部署」核心交付 | patch G1 |
| 2 | 部署指南 11.8/11.10 描述云端应用内导入/导出，实为 desktop-only capability 隐藏入口 + 17.3 端点（盲扫） | **medium** | capabilities.ts DESKTOP_ONLY_COMMANDS 含 data_export/data_import；GlobalSettingsModal 按 capability 隐藏；指南让用户点不存在的按钮 | patch G2 |
| 3 | 备份命令卷名 `egosync-server_egosync-data` 与实际 `server_egosync-data` 不符（compose 无 name:，项目名取目录名）；docker run -v 对缺失卷静默建空卷 → 空备份假象（盲扫/边缘） | **medium** | compose 无顶层 name: 已核；盲按文档操作得到空 tar | patch G3 |
| 4 | 反代拓扑下认证限流退化为实例级单桶（限流键取 socket IP= caddy 容器 IP；XFF 未消费），可被远程失败登录 DoS；README:194 仍宣传 5 次/分钟/IP（盲扫/缺口） | **medium** | auth.rs 限流按 ConnectInfo；behind_proxy 仅教 XFP 未教 XFF；机制属实但信任 XFF 做限流键需可信代理语义设计（防伪造绕过），超出 trivial | defer D1（+README 措辞诚实化并入 G2 文档批） |
| 5 | 全程 HTTPS 部署无 HSTS（盲扫） | **medium** | Caddyfile 无 Strict-Transport-Security；308 只保明文首访跳转，HSTS 闭合复访降级窗口 | patch G5 |
| 6 | 容器无日志轮转（json-file 默认无上限），1C1G 长跑磁盘风险（盲扫） | **medium** | compose 无 logging: 块；本故事自身经历过磁盘 95% 事故 | patch G6 |
| 7a | 人工裁决改掉冻结 CSP 但 architecture.md 权威文本未同步记录（盲扫） | **medium** | architecture.md CSP 段仍写两 Google 域冻结；与实现/契约测试直接矛盾；裁决已获人工批准，属记录缺位非决策缺位 | patch G7（追加式修订注记，不回改历史） |
| 7b | project-context.md 对新部署拓扑与三个新 env 零覆盖（盲扫） | **low** | agent-context 文件按分诊规则固定 defer | defer D5 |
| 8 | 三份 .dockerignore 靠人工同步维护无防漂移机制（盲扫/缺口） | **medium** | 副本失手（尤其 secrets 排除项）无声重开上下文洞；机械校验属 CI 基建，17.3 server-docker.yml 自然归宿 | defer D2 |
| 9 | .gitignore 防护面窄于 .dockerignore：缺 \*.db-journal、\*\*/secrets.json.tmp、\*\*/.env.\*；secrets.json.tmp 原子写期真实承载密钥（盲扫/边缘） | **medium** | 比对两 ignore 集合实证；「绝不入库」承诺对 tmp 文件不成立 | patch G9 |
| 10 | 抢注恢复指引不可操作：镜像无 sqlite3，无可用命令（盲扫） | **low** | debian-slim 运行时无 sqlite3；指引缺落地命令 | patch G10 |
| 11 | 运维速查表 `docker compose pull && up -d` 错误：镜像从未发布 registry，pull 拉空（盲扫） | **low** | 与紧邻正文「git pull → up -d --build」自相矛盾 | patch G10 |
| 12 | nginx 推荐用 $host 转发 Host 丢端口（$http_host 才原样），非缺省端口部署复现 403 病灶（盲扫） | **medium** | nginx 语义属实：$host 不含端口；Origin 带 :8443 vs Host 裸域名即 403 | patch G10 |
| 13 | opencode 下载只 pin 版本不校验内容（盲扫/边缘） | **medium** | 135MB 运行时二进制仅 TLS+--version 兜底；密钥安全主题故事应 pin sha256 | patch G11 |
| 14 | WAL :memory: 断言未走被测路径：直连建池未施加 journal_mode(Wal)（盲扫） | **low** | pool.rs:387-391 实证：断言对默认池恒真，「WAL 无害」声明未被测试 | patch G12 |
| 15 | CSP 零第三方域断言正则无 i 标志，大写域源可绕过（盲扫/边缘） | **low** | /https?:\/\/[a-z.]+/g 实证；CSP 域源大小写不敏感 | patch G13 |
| 16 | 版本 pin 一致性校验数注释行（×4 对 ×4 可在真分叉时仍过）（盲扫） | **low** | grep -c 含注释；校验须只匹赋值行——并入 V5 CI 门禁 | patch G23（spec 内「三处 vs ×4」措辞部分按规则驳回：修法是改本 spec） |
| 17 | sprint-status last_updated 丢时间戳精度（裸日期）（盲扫） | **low** | 本人同步时手误；历史格式带 T 时区 | patch G15 |
| 18 | Implementation Notes 记录 compose env 优先级反了（env_file 覆盖显式默认——实为 environment 优先，当前正确只因插值同源）（盲扫） | **low** | compose 规范 environment > env_file；机制说明误导后续维护 | patch G16（追加更正注记——append-only 约定内） |
| 19 | TARGETARCH 注入非 amd64/arm64（如 arm/v7）时被 uname 回退静默覆盖 → 异架构镜像错二进制（边缘） | **low** | case 逻辑实证：\*) 分支无条件按宿主覆盖；多平台 buildx 场景成立 | patch G17 |
| 20 | Caddyfile max_size 50MB 与 axum 50MiB 差 4.7%（边缘） | **medium** | caddy adapt 实证：50MB→50000000，axum 上限 52428800；50MiB→52428800 精确对齐 | patch G18 |
| 21 | EGOSYNC_BEHIND_PROXY 非 UTF-8 值静默按直连（边缘） | **low** | Err(\_) => false 无告警；观测缺口 | patch G19（并入解析纯函数抽取） |
| 22 | secret_store env 空值被当有值 → provider 401 而非结构化重录错误（边缘） | **medium** | env::var Ok("") 计入 Some；但改 load 行为触碰冻结「不改其行为」边界 | defer D4 |
| 23 | TZ 非法值静默 UTC（边缘） | **low** | 简报错时无提示；文档一行缓解 | patch G10 |
| 24 | tailwind family 名实改（Inter Variable）与任务措辞「名不变」矛盾（边缘） | **low** | fix = 改本 spec 措辞 → 按规则驳回；fontsource 注册名带 Variable 后缀是必然，视觉栈序不变 | 驳回（spec-edit） |
| 25 | logout 时 XFP 缺失致过期 Cookie 属性不对称（边缘） | **low** | 会话行已删，残留 Cookie 服务端不可达，无安全后果；修法需随行存 Secure 旗标加状态 | 驳回（low+复杂修法） |
| 26 | 端口 >65535 畸形 authority 解析为无端口（边缘） | **low** | 浏览器无法产生该 Origin/Host；两侧同函数处理一致 | 驳回（不可达） |
| 27 | EGOSYNC_BEHIND_PROXY env 解析无测试（缺口，预验证） | **medium** | 解析回归静默关闭 TLS 修复全链且测试全绿（注入的是解析后布尔） | patch G19 |
| 28 | EGOSYNC_OPENCODE_PATH→SidecarManager 接线无测试（缺口，预验证） | **medium** | 改回 (None,None) 全绿；按层建议以引擎侧 resolve_binary_path 命中路径测试闭合 | patch G20 |
| 29 | 六处 missing_api_key_error 调用点无调用路径测试（缺口，预验证） | **medium** | 仅构造函数被测；任一调用点回退旧文案无门禁 | patch G21 |
| 30 | 字体自托管无正向产物断言——字体整体消失仍全绿（缺口，预验证） | **medium** | csp.contract 只断言外链缺席（对「什么都没有」天然成立）；CI dist 分支是现成挂点 | patch G22 |
| 31 | Dockerfile⇄release.yml pin 一致性仅手工 grep（缺口，预验证） | **medium** | 无任何 workflow/测试比对两处；升版漏改静默分叉 | patch G23（ci.yml 门禁，只匹赋值行） |
| 32 | 根 .dockerignore 开始治理 relay 构建但 relay-docker paths 无触发耦合（缺口，预验证） | **low** | paths 过滤实证不含 .dockerignore；当前排除集与 relay COPY 清单比对无破坏 | patch G24（paths 加一行） |
| 33 | stdout JSON 日志格式无验证（缺口，预验证） | **low** | 钉法需 spawn bin 读输出属进程级烟囱，17.3 镜像冒烟自然归宿 | defer D3 |

**分组汇总**：patch 组 G1-G24（含文档批 G2/G10、Caddyfile G5/G18、compose G3/G6、Dockerfile G11/G17、.gitignore G9、engine 测试 G12/G21、server 代码/测试 G13/G19/G20、前端测试 G22、CI G23/G24、杂项 G15/G16/G7）；defer 组 D1-D5；驳回 3 项（#24/#25/#26）。无 intent_gap、无 bad_spec → 不回环，patch/defer 正常处理。

## Design Notes

- **EGOSYNC_BEHIND_PROXY 门控而非无条件信任 X-Forwarded-\***：跨源拒绝是浏览器面防线（浏览器无法伪造 X-Forwarded-\*），但显式门控使直连暴露（无反代）语义零变化，e2e/dev 零影响；Caddy 反代默认透传 X-Forwarded-Proto（不设 Port），端口缺省按该 scheme 归一。
- **Cookie Secure 的 e2e 兼容**：仅 BEHIND_PROXY=1 且 XFP=https 时附加；16.2 web e2e 直连 http://127.0.0.1 不设该 env，行为不变。
- **opencode 注入零引擎改动**：EGOSYNC_OPENCODE_PATH 为文件时取父目录传 `SidecarManager::new` 首参，复用既有「resources/ 子目录 → 根」搜索序；桌面传 tauri resource_dir 不动——「同一注入点两个值」落位。
- **镜像内 dist 构建**：capabilities.ts 等传输工件已提交入库，前端阶段仅需 node + npm ci；vite 无 TAURI_ENV_PLATFORM 时走 safari13 target 分支，双宿主探测在运行时完成。
- **WAL 桌面侧生效**：engine 共享，桌面库文件首开后切 WAL——存储引擎细节、用户不可见，实现注记登记；:memory: 测试库 SQLite 规定保持 memory 模式（设 WAL 无效但不报错）。
- **字体自托管路径**：fontsource npm 包（OFL 授权随包携带）提供 unicode-range 分片 woff2 与 CSS，vite 打包进 dist——比手工散下载可控（版本化/可复现）；可变字体单文件覆盖全字重，体积最优；桌面安装包与镜像同步增大（预估 10-20MB 量级），换离线一致与零第三方依赖；16.1 冻结 CSP 的两 Google 域放行由本故事人工裁决显式取代（renegotiation），契约测试同步收紧为「零第三方域」。
- **错误收敛的形态**：AppError variant 维持 KeyringError（15.1 defer 裁决不改名），收敛的是文案拼装——engine 内单一 `pub fn missing_api_key_error(config_name: &str) -> AppError`，6 调用点替换；17.3 的批量探测复用同一助手（deferred-work 已登记接线计划）。

## Verification

**Commands:**
- `cd server && cargo test` -- expected: 全绿（含新增 security/auth 单测）
- `cd crates/egosync-engine && cargo test` -- expected: 全绿（WAL 改动波及面）
- `cd egosync-app && npm run build && npx vitest run` -- expected: 零类型错误 + 前端全绿（含改版后的 csp.contract.test）
- `ls egosync-app/dist/assets/ | grep -c woff2` + `grep -r fonts.googleapis egosync-app/dist/index.html` -- expected: 本地 woff2 存在且外链引用清零
- `cd egosync-app && npm run test:all` -- expected: 全绿（DoD）
- `cd server && docker compose config -q && caddy validate --config Caddyfile` -- expected: 语法双过
- `grep -c 'v1.15.10' server/Dockerfile && grep -rn '1.15.10' .github/workflows/release.yml` -- expected: 版本 pin 两处一致
- 本机 docker 演练（AC 驱动）：`docker compose build && EGOSYNC_DOMAIN=localhost docker compose up -d` → `curl -k https://localhost/healthz`（200）→ `curl -sI http://localhost/`（308）→ 带 `Origin: https://localhost` 的 POST（放行 + Set-Cookie Secure）→ `curl -k -N https://localhost/api/events`（SSE 逐写）→ `docker compose restart` 后数据/证书无损 → `docker stats --no-stream` 采样内存并回填文档
- `docker run --rm <image> ls /data` -- expected: 空（secrets/db 不进镜像层）

**Manual checks (if no CLI):**
- 部署指南从零到浏览器步骤按文档真实走一遍（本机演练即人工核验载体）

## Verification Results

（2026-09-21 本机执行记录；演练机约束：80/443 被宿主 systemd caddy 占用、无 buildx（经典构建器）、2C/3.6G）

- ✅ `cd server && cargo test` — **全绿**：24+2+14+6+2+4+5 = 57 个集成测试 + 库单测（含新增 `behind_proxy_same_origin_and_secure_cookie` / `direct_http_unchanged_and_forged_xfp_ignored`、auth `cookie_secure_flag_gates_on_behind_proxy_and_xfp`、security 5 个同源/IPv6 单测、bootstrap `opencode_resource_dir_maps_env_to_injection_dir`）
- ✅ `cd crates/egosync-engine && cargo test` — **全绿**：824 passed / 0 failed（含新增 `missing_api_key_error_is_structured_and_points_to_reentry_path`、`init_dbs_enable_wal_journal_mode`）
- ✅ `cd egosync-app && npm run build && npx vitest run` — tsc 零错误；**877 passed / 67 files**（含改版后 csp.contract.test 的零第三方域断言）
- ✅ 字体产物：`dist/assets/` 110 个 woff2（4.59MB，unicode-range 分片按需取用）；`grep fonts.googleapis dist/index.html` 零命中
- ✅ `cd egosync-app && npm run test:all` — **全绿（DoD）**：vitest 877（67 files）+ src-tauri cargo test 107 + engine cargo test 824，链式 exit 0
- ✅ `docker compose config -q`（COMPOSE OK）+ `caddy validate --config Caddyfile`（Valid configuration）
- ✅ 版本 pin：Dockerfile `ARG OPENCODE_VERSION=v1.15.10` == release.yml 三处 `OPENCODE_VERSION="v1.15.10"`（评审 #16 修正：只匹**赋值行**——初版记录的「×4 对 ×4 grep 计数」含注释行数，数巧合对齐不构成校验；现已升级为 ci.yml `opencode-pin-consistency` job 机械门禁 + 双架构 sha256 pin 齐备核对，本地模拟通过）
- ✅ 本机 docker 演练（AC 驱动，端口 8081/8443 + Host 钉头适配）：
  - `docker compose up -d` 两服务 Up（egosync-server healthy）
  - `curl -k https://localhost:8443/healthz` → **200 `{"status":"ok"}`**
  - `curl -sI http://localhost:8081/` → **308 Permanent Redirect → `https://localhost/`**（Caddy 方法保留重定向，语义为 301 超集——见 Implementation Notes）
  - 带 `Origin: https://localhost` 的 POST login → **200 + `Set-Cookie: …; Secure`**（修复前该形态 403——病灶实测闭合）；跨源 `Origin: https://evil.example` → **403**（拒绝语义保持）
  - `curl -k -N …/api/events`（Bearer）→ **SSE 字节即时到达**（无代理缓冲；flush_interval -1 生效）
  - `docker compose restart` → 数据/会话/证书无损（重启后旧会话 Cookie 仍 200；/data 双库 + `-wal`/`-shm` 在卷内——WAL 实证）
  - `/healthz?deep=1` → **`{"status":"ok","db":true,"opencode":true}`**（HOME 修复后 sidecar 全量健康）
  - `docker stats --no-stream`：egosync-server **332.8MiB** / caddy **13.6MiB**（引擎全量运行态）——已回填部署指南 11.7
- ✅ `docker run --rm egosync-server:local ls /data` — **空**（secrets/db 不进镜像层）；`/app/resources/opencode --version` → `1.15.10`
- ✅ 构建验证附带：多阶段三容器全部成功（前端 npm ci+build / rust --locked 双层缓存 / 运行时 opencode 下载+x64）；镜像内二进制非 stub（healthy 容器 + deep 全绿为证）；镜像内 dist 伺服实测（容器内 `wget http://localhost:8080/` 返回 index.html、assets 200 text/javascript；110 个 woff2 在 `/app/static/assets/`、零 Google Fonts 引用）
- ✅ I/O 矩阵行 9 复核（TLS 面响应头）：经 Caddy 的响应携带收紧后 CSP（`font-src 'self'`、零第三方域）+ nosniff / X-Frame-Options / Referrer-Policy——HTTP/2 下全透传

**评审后复跑（2026-09-21，24 项 patch 全落地后——本地第一手执行）**：
- ✅ `cd egosync-app && npm run build` — tsc 零错误 + `✓ built in 7.41s`
- ✅ 前端 vitest 全量 — **877 passed / 67 files**（含 csp.contract 新增 `i` 标志零第三方域断言 + dist woff2/css 正向断言）
- ✅ `cd server && cargo test` — **全绿**：lib 单测 17（含新 `parse_behind_proxy_maps_env_values`）+ 集成 57（proxy_tls 2 / secret_store 4 / api 24 / command_parity 2 / desktop_remote 14 / parity 6 / 其余 5）
- ✅ `cd crates/egosync-engine && cargo test` — **827 passed / 0 failed**（824 → +3：`resolve_binary_path_hits_resource_dir_binary`、`missing_key_call_paths_surface_structured_reentry_error`、`all_missing_key_call_sites_share_single_helper`；含改版后 `init_dbs_enable_wal_journal_mode`——memory 段真实施加 WAL 走被测路径）
- ✅ `cd egosync-app/src-tauri && cargo test` — **107 passed / 0 failed**
- ✅ `npm run test:all` 语义（vitest && src-tauri cargo test && engine cargo test 链）——三组件分链各 exit 0，AND 链等价成立
- ✅ `EGOSYNC_DOMAIN=localhost caddy validate --config server/Caddyfile` — Valid（含 HSTS 头 + 50MiB）；`docker compose config -q` — OK（含顶层 name / 双服务 logging）
- ✅ Caddy 适配值实证：`max_size 50MiB` → 52428800 == axum DefaultBodyLimit（50MB 时代为 50000000，4.7% 差已闭合）
- ✅ Dockerfile sha256 pin 实证：v1.15.10 linux-x64 tarball 下载核对 `a4c0c94a…cd0c5` 匹配官方 asset digest（arm64 值同取自官方 API）
- ✅ TARGETARCH 逻辑单测（shell 级）：空→宿主推导 amd64；amd64/arm64 直用；riscv64/arm-v7 → 硬失败
- ✅ ci.yml `opencode-pin-consistency` job 本地模拟：Dockerfile ARG == release.yml 三处赋值 == v1.15.10；x64/arm64 sha256 pin 齐备；两 workflow YAML 解析通过
- ✅ package-lock 修复核验：`mirrors.tencentyun.com` 计数 0（三 fontsource 条目 resolved 全部指回 registry.npmjs.org）

## Completion Notes

（收口时填入）

**实现侧补充（2026-09-21）**：
- 所有执行项完成；`.gitignore` 追加 `**/secrets.json` / `**/.env` / `*.db[-wal/-shm]`（原文件无此三类模式；部署指南引导用户在 server/ 创建含令牌的 .env——不入库是安全前提）。
- 演练环境残留清理：drill compose 栈与卷、临时 override、server/.env 均已移除；`src-tauri/target/debug`（11G 本地缓存）为腾磁盘删除——可再生的本地产物，非仓库内容。
