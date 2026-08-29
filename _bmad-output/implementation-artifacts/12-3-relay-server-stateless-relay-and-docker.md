---
baseline_commit: 0a932fd9f842ff76433fa17625c129ba9a57ec87
---

# Story 12.3: `relay-server` 无状态加密中继与 Docker 部署

Status: done

## Story

As a 出网在外的伴侣用户,
I want 一台自托管的云中继在我离开局域网时转发手机与桌面之间的端到端加密流量,
So that 我在户外也能安全使用手机伴侣，且中继对传输内容零知识、成本可控。

## Acceptance Criteria（AC）

> 完整 AC 以 `_bmad-output/planning-artifacts/epics.md` Story 12.3 段为唯一事实源，以下为逐条搬运，编号供任务引用。

1. **AC1 项目骨架与端点**：`relay-server/` 子项目基于 axum 0.8 + tokio + tokio-tungstenite，含 `src/{main.rs, registry.rs, forward.rs, auth.rs}`，与桌面共享 `crates/companion-proto`（path 依赖）；监听 `ws://0.0.0.0:7333`（端口可配），提供 `GET /healthz` 返回 200。
2. **AC2 注册鉴权与纯转发**：两个客户端（桌面与手机）以同一 `relay_id` 连接中继；注册阶段中继以挑战-应答要求连接方证明持有对应静态私钥，验证通过才登记 `{relay_id → 连接}`；应答错误被拒绝且不登记（防 ID 抢占）；双方就绪后，中继仅按 `relay_id` 做纯二进制密文帧转发，不解析、不修改帧内容。
3. **AC3 零持久化与日志纪律**：中继运行任意时长，注册表只存在于内存，零数据库、零磁盘写、断线即丢（不做离线投递）；集成测试断言：进程重启后注册表为空（"零持久化"验收进 `tests/`）；tracing 日志只含连接/转发事件元数据（relay_id、字节数），帧明文永不入日志。
4. **AC4 Docker 与 CI**：多阶段 Dockerfile 构建成功并启动服务，`docker compose up` 后 `/healthz` 探活通过；`.github/workflows/relay-docker.yml` 在推送时构建并发布镜像，CI 含 `cargo test`。
5. **AC5 压测验证**：单 VPS 部署（$5/月级）场景下数百并发长连接压测（Story 内以基准测试或文档化压测脚本验证）；服务无状态运行，内存占用与连接数线性、无持久化增长，转发延迟在可接受范围。

## Tasks / Subtasks

- [x] Task 1：`relay-server/` Cargo 骨架 + `main.rs`（AC1）
  - [x] `cargo new relay-server`（独立 bin crate，**不**加入仓库根 workspace——根 workspace 被架构明令禁止）；`Cargo.toml`：`axum = { version = "0.8", features = ["ws"] }`、`tokio`（full 或 macros+rt-multi-thread+signal）、`companion-proto = { path = "../crates/companion-proto" }`、`serde/serde_json`、`sha2 = "0.10"`、`tracing`、`tracing-subscriber`；`[dev-dependencies]` 增 `tokio-tungstenite`（测试客户端用，版本与 axum 0.8 内建的对齐——评审核实为 0.29，非早期情报所记 0.26）
  - [x] `main.rs`：tracing 初始化（env `RUST_LOG`，默认 info）→ axum Router：`GET /healthz`（返回 200，body 可为 `{"status":"ok"}`）+ `GET /relay`（或 `/`）WS upgrade 端点；监听地址 `RELAY_HOST`（默认 `0.0.0.0`）+ `RELAY_PORT`（默认 `7333`）env 可配；启动日志一行（含监听地址，不含任何密钥）
  - [x] 优雅退出：SIGTERM/SIGINT → server shutdown（Docker stop 场景；沿用 tokio signal 常规写法）
- [x] Task 2：`registry.rs` 内存注册表（AC2、AC3）
  - [x] `Registry`（`Arc<Mutex<HashMap<String, RelayEntry>>>` 或 dashmap——默认 std HashMap + tokio Mutex 即可，规则二）：`RelayEntry { desktop: Option<Slot>, phone: Option<Slot> }`，`Slot` 持有对端发送通道（`mpsc::UnboundedSender<Message>` 或等价句柄）+ 注册时刻 + 对端静态公钥
  - [x] 语义：单槽替换（同角色二次注册，验证通过后**替换旧槽并关闭旧连接**——镜像 12.2 评审 P6「新连接取代僵尸会话」教训）；任一端 WS 断开 → 清其槽位，并对端连接发送 close（让对端感知并走重连）；双槽均空 → 移除该 `relay_id` 条目（防 Map 无界增长）
  - [x] `#[cfg(test)]` 同文件单测：空构造、登记、单槽替换、断线清理、双槽空移除
- [x] Task 3：`auth.rs` 挑战-应答注册鉴权（AC2）
  - [x] **relay 控制协议**（relay 内部约定，写入 auth.rs 文档注释；**不进** companion-proto schema.json——8 帧 schema 是手机↔桌面 E2E 载荷，与本协议无关）：客户端连上 WS 后第一条消息必须是 **text JSON** `{"type":"register","relayId":"<16 hex>","role":"desktop"|"phone"}`（字段 camelCase，继承基线）；随后进入握手阶段：双方交换 **3 条 binary WS 消息**（每条一条 Noise 消息，无长度前缀——与 12.2 桌面 WS 承载约定完全一致）；握手完成后本连接进入转发态
  - [x] 挑战-应答实现（**零新增密码学**，纯复用 crate API）：中继对每个新连接 `companion_proto::crypto::generate_static_keypair()` 生成**一次性**密钥对 → `HandshakeSession::initiator(relay_priv)`；客户端以 `HandshakeSession::responder(own_static_priv)` 应答——XX 握手完成即证明应答方持有其所出示静态公钥对应的私钥（挑战=握手，应答=以正确私钥完成握手；错误私钥必然握手失败）。握手成功后中继 `remote_static_pubkey()` 取对端公钥做角色校验；transport 会话即弃（仅鉴权用途，中继不参与后续加密）
  - [x] 角色校验：`role=desktop` → `hex(SHA-256(pubkey))[..16] == relay_id`（**逐字镜像** 12.2 落地实现 `[Source: egosync-app/src-tauri/src/services/companion_pairing.rs:119]`，hex 编码与截断长度不得有差）；`role=phone` → 记录公钥入槽（证明"持有其出示公钥的私钥"即可；relay_id 归属桌面，见 Dev Notes 裁决 3）
  - [x] 拒绝路径：首消息非 register JSON / 未知 role / 握手失败 / desktop 槽 hash 不匹配 → 关闭连接、不登记（tracing warn 只记 relay_id + 失败原因类别）
  - [x] 鉴权阶段整体超时（10s，镜像 12.2 评审 P4 先例）+ 握手消息尺寸上限（32 字节下限沿用 12.2「短于临时公钥即非法」判据、上限防内存滥用）——未鉴权连接不得长期占用
- [x] Task 4：`forward.rs` 纯转发（AC2）
  - [x] 双槽就绪即启动双向转发：任一端收到的 **binary** WS 消息原样（不解码、不解析、不修改字节）发往对端；转发统计只记字节数（tracing debug 级）
  - [x] text 消息在转发态收到 → 忽略并 warn（控制协议只在注册首消息出现一次）；WS 层 Ping/Pong 由 tungstenite 协议栈自处理
  - [x] 单条消息尺寸上限沿用 tungstenite 默认（64MiB）不额外收紧——快照口径上限 10MB，转发层无理由拦截（帧尺寸校验是两端 crate `decode_frame` 的职责，中继不解析）
- [x] Task 5：集成测试 `relay-server/tests/relay.rs`（AC2、AC3）
  - [x] 测试基建：以子进程方式拉起 relay 二进制（`std::process::Command` + cargo 构建产物，`RELAY_PORT` 指向随机空闲端口），或 `#[tokio::test]` 内直接构建 Router + `axum::serve` 于随机端口（二选一，优先后者——零持久化断言需要真子进程重启的除外）；测试客户端用 `tokio-tungstenite` 直连 + `companion_proto` 完成 responder 侧握手
  - [x] `healthz` 返回 200（AC1）
  - [x] 注册成功路径：模拟桌面（desktop 角色，公钥 hash==relay_id）+ 模拟手机（phone 角色）注册 → 双向二进制帧转发往返逐字节一致（AC2）
  - [x] 防 ID 抢占：desktop 槽位错误私钥（公钥 hash ≠ relay_id）→ 拒绝且不登记——后续正确客户端仍可注册成功；握手消息损坏（沿用 12.2 判据：<32 字节）→ 拒绝（AC2）
  - [x] **零持久化断言**（AC3）：同一 relay_id 在进程 A 注册并转发成功 → kill A → 同端口拉起进程 B → 断言 B 对该 relay_id 一无所知（同 relay_id+正确密钥可全新注册成功、无旧对端残留转发）——注册表只活在进程内存
  - [x] **零磁盘写断言**（AC3）：子进程以空临时目录为工作目录运行，注册+转发+断开全流程后扫描该目录无任何新增文件
  - [x] 断线即丢：一端断开 → 对端收到 close；单槽替换：同角色二次注册 → 旧连接被关闭（AC2/AC3）
  - [x] 日志纪律自查（NFR-M7）：relay 全部 tracing 调用只含 relay_id/角色/字节数/事件类别，无帧字节、无任何密钥材料（AC3）
- [x] Task 6：压测验证（AC5）
  - [x] `relay-server/tests/relay_load.rs`：CI 友好规模（默认 ~20 对并发连接 × 若干轮转发）断言全部成功 + 记录耗时；规模经 env（如 `RELAY_LOAD_PAIRS`）可调，供本地/VPS 大压测复用同一测试
  - [x] `relay-server/LOADTEST.md`：文档化压测脚本——在 VPS 上以 `RELAY_LOAD_PAIRS=300` 运行上述测试的步骤、观测指标（RSS 内存 vs 连接数线性、p50/p99 转发延迟）、验收口径（数百并发长连接稳定、内存无随时间增长趋势）（AC5——AC 明示"基准测试**或**文档化压测脚本"二选一即可，此处取"可复用测试 + 文档化流程"）
- [x] Task 7：Docker + CI（AC4）
  - [x] `relay-server/Dockerfile` 多阶段：stage1 `rust:1`（bookworm）`cargo build --release`（依赖层与源码层分开缓存：先拷 Cargo.toml + 虚拟 src 建 deps 层，再拷源码）；stage2 `debian:bookworm-slim`（或 `gcr.io/distroless/cc-debian12`）只拷二进制；`EXPOSE 7333`；`HEALTHCHECK` 用无 curl 环境可用的探活（distroless 无 shell——若用 distrocheck 则选 debian-slim + wget/curl，或 compose 层 healthcheck，见 Dev Notes 裁决 5）
  - [x] `relay-server/docker-compose.yml`：服务 relay、端口映射 `7333:7333`（env 可配）、healthcheck、`restart: unless-stopped`；本地 `docker compose up` 后 `/healthz` 探活通过（本地无 Docker 环境——release 二进制直接验证 /healthz 200 + SIGTERM 优雅退出，容器内探活留待 CI/部署验证）
  - [x] `.github/workflows/relay-docker.yml`：风格对齐既有 ci.yml（checkout@v4、dtolnay/rust-toolchain@stable、Swatinem/rust-cache@v2）；触发 `push`（paths: `relay-server/**`、`crates/companion-proto/**`）+ PR；job1 `cargo test`（working-directory: relay-server）；job2 构建 Docker 镜像，main 分支推送时发布到 `ghcr.io/<owner>/egosync-relay`（GITHUB_TOKEN 即可，无需新密钥）；PR 仅构建不推送
- [x] Task 8：验证收尾
  - [x] `cd relay-server && cargo test` 全绿（单测 + 集成 + 负载冒烟）
  - [x] `cargo check`（若 CI 单独跑 test 则 check 可省）零警告级错误
  - [x] 本地 `docker compose up` + `/healthz` 200（无 Docker 环境时如实说明并以 CI 构建代替验证，禁止声称"已验证"）——本机无 Docker 二进制，改以 release 二进制直接验证 `/healthz` 200 + SIGTERM 优雅退出（handler 已安装、进程 2s 内退出）；容器内探活留待 CI `relay-docker.yml` 构建验证
  - [x] 桌面零回归自查：本 story **零桌面文件改动**——`git diff --stat` 确认变更仅落在 `relay-server/**`、`.github/workflows/relay-docker.yml`；`egosync-app/`、`crates/companion-proto/`、`companion-android/` 零触碰（桌面侧中继接线属 12.4+，见范围外）

### Review Findings

> 三路对抗审查（Blind Hunter / Edge Case Hunter / Acceptance Auditor，2026-08-28）。三方独立撞同一组缺陷，证据互相印证。已满足的 AC 不罗列（AC1 骨架/端口/healthz、AC2 挑战-应答/防抢占/纯转发、AC3 零持久化/零磁盘写/日志纪律、AC4 CI cargo test+ghcr 发布核心条款均有证据）。

#### decision-needed

- [x] [Review][Decision] phone 槽位抢占防御策略 — **裁决 (a)：现在实现公钥绑定**（2026-08-28）。phone 首注册绑定公钥入槽，重复注册校验新公钥与旧槽一致、不一致拒绝。转为下方 patch 项 `phone 槽公钥绑定防抢占`。换机重配对场景待对照 12.2 配对流程确认（若换机伴随 desktop 重新配对生成新 relay_id 则无代价）。

#### patch

- [x] [Review][Patch] phone 槽公钥绑定防抢占 [relay-server/src/auth.rs:167-175, relay-server/src/registry.rs:70-82] — 用户裁决 (a)。phone 首注册绑定 `remote_static_pubkey` 入槽；同 relay_id 再次 phone 注册时校验新公钥与旧槽一致，不一致拒绝且不登记（堵死第三方抢占；合法手机换密钥须 desktop 侧重配对）。需补抢占防御回归测试。
- [x] [Review][Patch] 转发无界 mpsc + 写无超时 [relay-server/src/forward.rs:29,44-56] — `unbounded_channel` + `socket.send` 无超时，慢/半死对端致内存无界增长、任务挂死、槽位不清理，违反 AC5。改为有界通道 + 满则断连（镜像断线即丢语义）。需补慢消费者回归测试（BH-12）。
- [x] [Review][Patch] 鉴权超时实为每 recv 独立 10s + send 无超时 [relay-server/src/auth.rs:107,110,133,139,157] — 注释称「整体超时」但两次 recv 各套 10s，最坏 ~20s（P4 预算 2 倍）；两次 send 无超时可被慢客户端无限挂住。改为单 `Instant` deadline 共享、对剩余时间统一套 timeout。需补 AUTH 整体超时上限回归测试（BH-12）。
- [x] [Review][Patch] WS 帧大小无上限 [relay-server/src/lib.rs:38-43, relay-server/src/auth.rs:119] — `WebSocketUpgrade` 未设 `max_message_size`，握手 text 与转发 binary 均依赖库默认 ~64MB，未鉴权连接可发巨量 text 先分配内存再拒绝。在 upgrade 处设 `max_message_size`（转发态与握手态统一上限）。需补大帧拒绝测试（BH-12）。
- [x] [Review][Patch] 服务端无主动保活 Ping [relay-server/src/forward.rs:41-83] — 注释「Ping/Pong 由协议栈自处理」属实（tungstenite 0.26 自动回 Pong 已源码核实），但**服务端从不主动发 Ping** → 无 FIN 的死对端（NAT 超时/断电）永不被探测，僵尸连接与槽位永久滞留注册表，违反 AC5。select! 增加周期 Ping 分支 + pong 缺失/空闲超时则 break。需补保活测试（BH-12）。
- [x] [Review][Patch] 优雅退出无 Close 广播 [relay-server/src/main.rs:29-32, relay-server/src/forward.rs:41-83] — `with_graceful_shutdown` 仅停接受新连接，已升级 WS 任务无 shutdown 信号，进程挂到 Docker SIGKILL（10s），客户端收不到干净 Close。需广播 shutdown + 对活跃连接发 Close + 超时兜底。
- [x] [Review][Patch] LOADTEST.md VPS 压测命令双 `--` 致跑 0 测试 [relay-server/LOADTEST.md:48] — `-- --nocapture -- --test-threads=1`：第二个 `--` 后内容被当测试名过滤器，匹配不到任何测试 → 静默运行 0 个测试，AC5 验收入口断链。去掉第二个 `--`（该二进制仅 1 个测试，`--test-threads=1` 本无意义可一并删）。
- [x] [Review][Patch] Dockerfile 未 COPY Cargo.lock [relay-server/Dockerfile:14-24] — 只 COPY 两个 Cargo.toml + 源码，容器内重新解析依赖，镜像与 CI `cargo test` 依赖基线可漂移，AC4 可复现构建意图落空。依赖层与源码层均 COPY `relay-server/Cargo.lock` 并构建命令加 `--locked`。
- [x] [Review][Patch] relay-server/target/ 未被 .gitignore 覆盖 [.gitignore] — 根 .gitignore 仅忽略 `crates/companion-proto/target/`，`git status` 显示数百 MB 构建产物未被跟踪；一旦 `git add relay-server/` 即污染仓库。**提交前阻断项**。追加 `relay-server/target/`。
- [x] [Review][Patch] sprint-status last_updated 时间戳回退 + 时区漂移 [_bmad-output/implementation-artifacts/sprint-status.yaml:38] — `2026-08-29T09:10:00+08:00` → `2026-08-28T11:05:00+0800`（向过去回退约 22h + 时区格式与模板不一致）。修正为当前时间戳、带冒号时区 `+08:00`。
- [x] [Review][Patch] Docker 运行时加固：无 USER + 无资源限制 [relay-server/Dockerfile:28-38, relay-server/docker-compose.yml] — 运行时 root 用户、无 read_only/no-new-privileges/mem 限制。加 `USER`（非 root，端口 7333>1024 OK）+ compose `read_only`/`security_opt: no-new-privileges`/`mem_limit`。（wget 因 compose healthcheck 探活必需，保留。）
- [x] [Review][Patch] CI 镜像 tag 问题 [/.github/workflows/relay-docker.yml:71] — 仅 `:latest` 不可回滚；`github.repository_owner` 含大写时 ghcr 推送失败（GHCR 要求小写）。owner 转 `toLower` + 追加 `:sha-<short>` 版本 tag。
- [x] [Review][Patch] tokio-tungstenite 双版本并存 [relay-server/Cargo.toml:18, relay-server/Cargo.lock] — axum 0.8.9 实际依赖 0.29.0，dev-dep 为 0.26.2，两 major 并存；裁决 #1 记载「与 axum 0.8 内建的 0.26 对齐」事实错误。dev-dep 升至 `0.29` 对齐消除双版本 + 修正裁决 #1/story 关键技术情报。
- [x] [Review][Patch] 测试基建加固 [relay-server/tests/common/mod.rs:90,151-157, relay-server/tests/relay_load.rs:72] — (1) `free_port()` 先 bind 再释放有 TOCTOU，复用监听器地址；(2) 子进程 stderr `Stdio::piped()` 仅 `take_stderr` 消费，debug 级日志可写满 64KB 管道致子进程停摆，spawn 专线程持续读取；(3) `RELAY_LOAD_PAIRS=0` 致 `samples[len/2]` 越界 panic，clamp 到最小 1 或断言。
- [x] [Review][Patch] AC5 无自动化内存度量标记 [relay-server/LOADTEST.md] — `relay_load.rs` 只断言逐字节一致 + 打印延迟，RSS/连接数/增长趋势全为人工命令；裁决 #4 允许文档化形态（非违规），但合入时点「数百并发、内存线性」零实测证据须显式标注。补「压测未执行」状态标记。

#### defer

- [x] [Review][Defer] 每帧转发抢全局 Mutex [relay-server/src/forward.rs:63, relay-server/src/registry.rs:70-81] — deferred, 架构级优化（本地缓存 peer sender + 槽位替换时刷新），AC5 已由现有 20 对测试满足，超出本 story 范围；留待有压测证据（P6 命令修正后）证实瓶颈后再优化。register() 返回值被丢弃属设计使然（先到者无对端），不单独计缺陷。
- [x] [Review][Defer] 全局连接数上限/速率限制 [relay-server/src/lib.rs:38-43] — deferred, 公网 DoS 加固项，spec 未要求（AC5 要求「数百并发」而非防海量）；`register` 返回值丢弃 + 无连接信号量。留 V1 加固。

#### dismissed

- R1. BH-3「转发循环静默丢弃 Ping」机制描述：**误诊**——tungstenite 0.26.2 在 read 路径自动将 Ping 入 `additional_send` 队列并于下次 flush 回 Pong（`protocol/mod.rs:547-555` + flush 文档 "automatically queued pong responses"），转发循环持续 `socket.recv()` 会自动回 Pong；Acceptance Auditor 双重核实。真实缺口归 patch「服务端无主动保活 Ping」（上方）。
- R2. BH-5「wget 扩大攻击面」部分（wget 系 compose healthcheck 探活必需）+ BH-10「CI 无 clippy/cargo-audit」部分（非本 story AC4 验收项，属既有 CI 风格范围）：噪音。

## Dev Notes

### 架构硬边界（违反即返工）

1. **加密边界**：relay-server 经 path 依赖 `companion-proto`（`companion-proto = { path = "../crates/companion-proto" }`）；relay-server 内**禁止** `use snow`——只允许操作 `HandshakeSession / TransportSession / generate_static_keypair`（架构四条硬边界 #1）。
2. **中继边界**：只见 relay_id 与密文帧；无 DB、无磁盘写、断线即丢（架构硬边界 #3）；**新增任何落盘行为都是违约**（架构 Enforcement #3）。
3. **仓库根禁止 Cargo workspace**（12.1/12.2 已确立两次；relay-server 是第三个独立 Cargo 项目，三者以 path 依赖共享 crate，互不构成 workspace）。
4. **帧协议冻结**：8 帧类型、schema.json、PROTOCOL_VERSION 一律不动。relay 控制协议（register JSON + XX 握手承载）是 **relay-server 内部 app 层约定**，写文档注释于 auth.rs，不进 schema.json、不 bump 版本——它与 E2E 帧是两个正交层面（转发态里跑的才是 E2E 帧字节）。
5. **零知识纪律（NFR-M7）**：tracing 只记 relay_id、角色、字节数、事件类别（connected/registered/forwarded-bytes/disconnected/rejected）；帧字节、任何公私钥材料**永不入日志**（relay_id 本身是公钥哈希，可记——12.2 先例）。

### 设计裁决（规则七，显式择一）

| # | 冲突/歧义 | 裁决 | 理由 |
|---|------|------|------|
| 1 | AC1 写 "tokio-tungstenite" vs axum 内建 ws | **生产依赖用 `axum::extract::ws`（feature "ws"，内建 tokio-tungstenite——评审核实 axum 0.8.9 实际解析 0.29）**；`tokio-tungstenite` 仅作 dev-dependency 供测试客户端 | axum 0.8 ws 即 tungstenite 封装，AC 指名的是传输技术而非要求直依赖；直连 tungstenite 自管 upgrade 属重复造轮（规则二）；测试客户端确需独立 WS 实现 |
| 2 | AC2 "挑战-应答" 实现载体 | **XX 握手即挑战-应答**：中继每连接生成一次性密钥对作 initiator，客户端以真实静态私钥作 responder 完成握手 = 持有对应私钥的证明；desktop 槽位再叠加 `hash(pubkey)==relay_id` 校验 | 零新增密码学原语（crate API 原样可用，无需改 crate）；XX 本身就是相互认证协议，"挑战"内生于握手；一次性密钥对保证中继零身份状态；比自造 sign/DH 协议简单一个数量级 |
| 3 | 手机槽位鉴权语义（AC 只说"证明持有对应静态私钥"，而 relay_id = 桌面公钥哈希，手机无法证明持有桌面私钥） | **双槽位模型**：desktop 槽 = 严格校验（hash==relay_id）；phone 槽 = 证明持有其**所出示公钥**对应私钥（完成 XX 握手即证），但不校验该公钥与 relay_id 的关系 | 字面要求"持有**对应**静态私钥"对手机不可满足（手机不持桌面私钥）——歧义按最小可满足解释执行；手机对桌面的真身份认证由 E2E Noise XX（12.2 配对闸门 + nonce 闭环）承担，中继 phone 槽被抢占的后果仅是 DoS（抢占者无法解密任何帧），零知识隐私无恙；若 12.4 需更强 phone 槽 ACL（如 desktop 注册时在内存登记期望手机公钥），可在 relay-server 内独立增强，不破坏本 story 冻结的控制协议 |
| 4 | AC5 "数百并发压测" 的验收形态 | **可复用集成测试（CI 小规模冒烟）+ `LOADTEST.md` 文档化大压测流程** | AC 明示"基准测试**或**文档化压测脚本"二选一；CI 跑数百真实长连接不可行（runner 资源/时长）；criterion 基准测不出"长连接内存线性"这类指标，集成测试形态更贴题 |
| 5 | Dockerfile runtime 基底 | **`debian:bookworm-slim` + compose 层 healthcheck**（distroless 无 shell，HEALTHCHECK 探活需自备二进制，复杂度不划算） | 单二进制无外部资源，slim 已够小（~十几 MB 增量）；healthcheck 放 compose 用 wget/curl 探 /healthz，与 AC4 "docker compose up 探活通过" 的验收形态一致 |
| 6 | 零持久化断言 "进程重启后注册表为空" 的可观测性（中继无 /debug 状态端点，注册表不可直接查询） | **行为断言**：kill → 重启 → 同 relay_id 全新注册成功且无旧对端转发；辅以子进程临时工作目录无新增文件的零磁盘写断言 | 给中继加状态查询端点违反零知识最小暴露原则；内存注册表 + 无文件句柄 + 行为断言三者已构成完整证明链 |

### 关键技术情报

- **companion-proto 现成 API**（12.1 交付、12.2 实战验证，`crates/companion-proto/src/crypto.rs`）：
  - `generate_static_keypair() -> Result<(priv, pub)>` —— 中继侧每连接一次性密钥对
  - `HandshakeSession::{initiator, responder}(local_static_private)`、`write_message/read_message`（握手期无长度前缀，一条 WS 消息一条握手消息）
  - `remote_static_pubkey() -> Option<Vec<u8>>`（12.2 评审后新增的透传，可直接用）
  - `into_transport()` 本 story **不需要**（鉴权后即弃，转发态不加密不解析）
  - **本 story 零 crate 改动**。
- **relay_id 推导（必须逐字镜像）**：`hex_encode(&Sha256::digest(desktop_static_pubkey))[..16]`（小写 hex、取前 16 字符）[Source: egosync-app/src-tauri/src/services/companion_pairing.rs:119]。relay-server 自带 `sha2 = "0.10"` + hex 编码（手写 6 行 hex fn 或用 12.2 同款本地 fn，不引 hex crate）。
- **axum 0.8 破坏性变更**（[Source: axum CHANGELOG 0.8.0](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)，已核）：
  - 路径参数语法 `/{param}`（旧的 `/:param` 会 panic）
  - `axum::extract::ws::Message` 的 Binary 变体持 `Bytes`（非 `Vec<u8>`）、Text 持 `Utf8Bytes`——match 时注意类型
  - `WebSocket::close` 已移除——显式发送 close 消息或直接 drop
  - 内建 tokio-tungstenite 0.29（axum 0.8.9 实际解析版本，评审核实）；最低 Rust 1.75
  - `Router::into_make_service` + `axum::serve(listener, app)` 常规启动
- **snow API 陷阱**（12.1/12.2 实测继承）：一切以 crate 现有封装签名为准，不参考网上 snow/axum 示例拼装；`write_message/read_message` 返回 `Result<Vec<u8>>` 已被 crate 封装。
- **Noise XX 测试陷阱**（12.2 Debug Log #2）：`-> e` 首消息无 MAC，构造"握手失败"测试用**短于 32 字节**的非法输入（全零 64 字节会被当合法消息接受导致死锁）。
- **12.2 评审教训直接继承**：P4（鉴权 10s 超时防挂起）、P6（新连接取代僵尸会话——单槽替换语义）、P2（对端写失败必须走统一清理，不得 `?` 提前 return 绕过槽位回收）。
- **Docker 多阶段**：stage1 `rust:1-bookworm` 分层缓存（先 Cargo.toml + 虚拟 src 跑 `cargo build --release` 热依赖层，再拷真实源码增量构建）；产物 `target/release/relay-server`；stage2 `debian:bookworm-slim` + 二进制；注意 path 依赖要求 build context 含 `crates/companion-proto`——**Dockerfile 放 `relay-server/` 而 build context 设仓库根**（`docker build -f relay-server/Dockerfile .`，compose 同理设 context: `..`），否则 path 依赖拷不进镜像。
- **CI 风格锚点**（对齐既有 ci.yml）：actions/checkout@v4、dtolnay/rust-toolchain@stable、Swatinem/rust-cache@v2（`workspaces: relay-server -> target`）；注意既有 ci.yml 有"删除本地 rsproxy 镜像配置"步骤——那是 `egosync-app/src-tauri/.cargo/config.toml` 的事，relay-server 无此文件，勿照抄。
- **ghcr 发布**：`docker/build-push-action@v6` + `login-action`，镜像名 `ghcr.io/${{ github.repository_owner }}/egosync-relay`，GITHUB_TOKEN 权限 `packages: write`；PR 事件只 build 不 push（`push: ${{ github.ref == 'refs/heads/main' }}`）。

### 测试策略（规则九：验证意图）

- 意图锚点：**"中继不可读"是结构性保证**（转发字节级一致 + 零解析断言）→ 转发往返测试断言**逐字节相等**而非"能收到"；**"抢占无用"**（防抢占 WHY）→ 错误私钥被拒后正确客户端仍能注册（抢占不产生持续占用）；**"重启即失忆"**（零知识 WHY）→ kill+重启后同 relay_id 全新注册、旧对端零残留；**"中继不落地"**（隐私承诺 WHY）→ 临时工作目录零新增文件。
- 双客户端模拟：测试内用 `companion_proto` 生成两套密钥（desktop 一套、phone 一套），desktop 的 relay_id 按真实推导生成——模拟的是"两个诚实的端"，不是"端到端协议"（E2E XX 透传只测字节透传，不重复 12.1 的互通测试）。
- 转发态测试载荷：直接喂随机字节（中继视角本就是不可解密文）——不要用 encode_frame 造帧再断言"中继没解析"（中继不解析，任何字节都该原样过）。

### Project Structure Notes

```text
EgoSync/
├── relay-server/                        # [N] 全新独立 Cargo bin 项目（无根 workspace）
│   ├── Cargo.toml                       # [N] axum0.8(ws)/tokio/companion-proto(path)/serde/sha2/tracing
│   ├── Dockerfile                       # [N] 多阶段；build context = 仓库根（path 依赖）
│   ├── docker-compose.yml               # [N] 端口映射 + healthcheck + restart
│   ├── LOADTEST.md                      # [N] 文档化压测流程（AC5）
│   ├── src/
│   │   ├── main.rs                      # [N] axum router / /healthz / WS upgrade / env 配置 / tracing init
│   │   ├── registry.rs                  # [N] 内存注册表（双槽 + 断线清理 + 单槽替换）
│   │   ├── forward.rs                   # [N] 纯二进制双向透传
│   │   └── auth.rs                      # [N] 控制协议 + XX 挑战-应答 + relay_id 校验
│   └── tests/
│       ├── relay.rs                     # [N] 集成测试（含零持久化/零磁盘写断言）
│       └── relay_load.rs                # [N] 并发负载冒烟（规模可调）
├── .github/workflows/
│   └── relay-docker.yml                 # [N] cargo test + docker build/push ghcr
└── （egosync-app/、crates/、companion-android/ 零改动）
```

与架构 addendum 目录树（`relay-server/{Cargo.toml, Dockerfile, docker-compose.yml, src/{main,registry,forward,auth}.rs}`）完全一致；`tests/` 与 `LOADTEST.md` 为 AC 明示要求（"验收进 tests/"、"基准测试或文档化压测脚本"），属合规增量。

### Previous Story Intelligence（12.1/12.2 → 12.3）

- **crate API 以实物为准**：12.1 教训"API 字面与实测有差"——动手前先读 `crates/companion-proto/src/crypto.rs` 实际签名，本 story 的鉴权设计已按实物 API 设计（initiator/responder + remote_static_pubkey），**若实现中发现需要 crate 不存在的 API，停下升级为设计变更并记录，禁止 relay-server 内 use snow**。
- **WS 握手承载先例**：12.2 桌面监听已确立"握手期一条 WS 消息一条 Noise 消息（无长度前缀）、transport 期 crate 内建长度前缀"——relay 控制协议沿用同一承载约定，两端心智一致。
- **日志纪律先例**：12.1 crate 内 tracing 调用数为 0；12.2 只记帧类型判别式/长度/对端标识。relay 侧同口径：relay_id 可记（公钥哈希非机密），帧字节与密钥材料绝不记。
- **12.2 遗留接口位**：QR payload `relay_addr` 恒为空（12.2 裁决"relay-server 属 12.3"）——本 story 交付中继本体但**不改桌面 QR 逻辑**；relay_addr 的真实配置接线属 12.4（手机按 QR 的 relay_addr 连中继）或后续桌面 story，不在本 story 范围。

### Git Intelligence

基线 0a932fd（HEAD，工作区干净）：最近提交为 12.2 桌面配对交付（含评审整改 22 项）与 12.1 协议 crate。`relay-server/` 目录尚不存在，全量为新建；桌面侧无进行中变更，零回归风险天然成立（只要不触碰既有目录）。

### Latest Tech Information

- **axum 0.8.x**（0.8.0 于 2025-01 发布，当前 0.8.9+）：已核 CHANGELOG 破坏性变更（见关键技术情报）；`axum::extract::ws` 用法稳定。来源：[axum 0.8.0 发布公告](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)、[axum CHANGELOG](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)。
- **tokio-tungstenite 0.29**：axum 0.8 内建实际解析版本（早期情报误记 0.26，评审核实后 dev-dependency 已对齐 0.29，避免双版本并存）。
- **Rust Docker 镜像**：`rust:1-bookworm` 构建 + `debian:bookworm-slim` 运行为当前稳定惯例；distroless 可行但 healthcheck 需自备探针（裁决 5 弃用）。
- Noise 套件 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 互通性已由 12.1 黄金向量双向验证——本 story 无跨语言不确定性（Android 端消费在 12.4）。

### 范围外（明确不做，防 scope creep）

- **桌面侧中继接线**：companion_connection 增加 relay 注册客户端、QR `relay_addr` 真实值、设置页中继地址配置——属 12.4/后续 story（12.2 已留字段位）。
- **Android RelayClient**（12.4）；E2E 帧的业务处理（13.x）。
- 离线投递/暂存（架构明令排除）；帧解析（AC 明令禁止）；中继 metrics/可观测性增强（架构 Nice-to-Have 延后）；多实例横向扩展。
- phone 槽位期望公钥 ACL（裁决 3 已留 12.4 增强口，本 story 不做）。
- TLS/wss：V1 中继明文 WS 承载 E2E 密文（NFR-M1 零知识不依赖 TLS）；反向代理加 TLS 属部署文档范畴，不在代码内。
- 不改 companion-proto（零 crate 改动）；不动 opencode/agent/桌面任何模块；不建根 workspace。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-12.3]（AC 唯一事实源）
- [Source: _bmad-output/planning-artifacts/epics.md#Epic-依赖图（增量）]（12.1 → {12.2, 12.3} 并行、12.4 依赖 12.2+12.3）
- [Source: _bmad-output/planning-artifacts/architecture.md#云中继本体]（axum 0.8 + 内存注册表 + 挑战-应答 + 纯转发 + 成本模型）
- [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-Addendum—手机伴侣基建]（relay-server 目录树）
- [Source: _bmad-output/planning-artifacts/architecture.md#Architectural-Boundaries]（四条硬边界之 #1 加密、#3 中继）
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation-Patterns-Addendum—手机伴侣基建]（零持久化断言进 tests/、日志纪律、反模式）
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-...md]（12.3 范围：注册表/挑战-应答/纯转发/healthz/Docker/CI/零持久化断言；排除项：任何落盘、离线投递、帧解析）
- [Source: _bmad-output/implementation-artifacts/12-2-desktop-pairing-and-connection-service.md]（relay_id 推导、XX 握手承载先例、评审教训 P2/P4/P6、日志纪律）
- [Source: crates/companion-proto/src/crypto.rs]（实际 API 签名）
- [Source: egosync-app/src-tauri/src/services/companion_pairing.rs:119]（relay_id 推导落地实现）
- [Source: .github/workflows/ci.yml]（CI 风格锚点）
- [axum 0.8.0 发布公告](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)、[axum CHANGELOG 0.8.0](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)
- [Source: _bmad-output/project-context.md]（基线 100 条规范）

## Dev Agent Record

### Agent Model Used

glm-5.3（Amelia / bmad-dev-story 执行）。

### Debug Log References

- **TDD 红→绿节奏**：每个 Task 先写失败测试再实现。Task 1 healthz 测试初红（空 Router → 404）；Task 2 registry 7 测试 `todo!()` 红 → 实现后绿；Task 3 auth 4 纯函数测试 `todo!()` 红 → 绿；Task 4/5 集成 9 测试红（桩 handle_socket 不做事）→ forward.rs 完整编排后 8 绿，3 个子进程测试因测试基建缺陷失败。
- **子进程探活 panic（规则十三取证）**：`blocking_raw_http_get` 在子进程未就绪时直接 panic（ConnectionRefused），使 `wait_ready` 无法轮询就绪。根因是 helper 把连接拒绝当致命错误而非「未就绪」。修复为 `connect_timeout` + 失败返回 status 0，`wait_ready` 正常轮询。
- **take_stderr 死锁**：初版 `take_stderr` 在子进程仍存活时读管道 → 无 EOF 永久阻塞（`cargo test` 卡 10 分钟）。根因：必须先 kill 再读管道（管道在进程退出后达 EOF）。修复为「kill → 读 → wait」。
- **tracing 默认写 stdout**：日志纪律测试正向控制（relay_id 应出现）失败——`tracing_subscriber::fmt()` 默认写 stdout，而测试捕获 stderr（stdout 被 null）。根因明确后，main.rs 显式 `.with_writer(std::io::stderr)`（亦为容器日志惯例）。未引入诊断日志代码，全部为外部观察取证。
- **优雅退出「假性失败」排查**：手动冒烟报告 rc=143 且 ps 显示残留监听进程，一度怀疑 SIGTERM handler 未安装。取证：对真实 relay PID（pgrep -x）读 `/proc/PID/status` 得 `SigCgt` 含 bit14（SIGTERM）与 bit1（SIGINT）→ **handler 已正确安装**，SIGTERM 后 2s 内干净退出。`$!` 与 pgrep PID 不一致 → 本测试外壳（bash -c 包装层）使 `$!` 指向包装 shell 而非 relay；rc=143 是包装层被信号终止的假象。**优雅退出实现无缺陷**，无需改代码。规则十三：所有诊断均为外部观察，未向代码注入任何临时诊断日志，无遗留待移除。

### Completion Notes List

- **全量交付**：`relay-server/` 独立 bin crate（lib + bin，零根 workspace），4 源模块（main/lib + registry/auth/forward）+ 2 集成测试 + 1 负载测试 + Dockerfile + compose + CI 工作流 + LOADTEST.md。
- **测试**：lib 单测 11（registry 7 + auth 4，含 derive_relay_id 镜像参考实现校验）+ 集成 11（healthz / 注册逐字节转发 / 防抢占 / 损坏握手拒绝 / 非 register 首消息拒绝 / text 忽略 / 断线 close 传播 / 单槽替换 / 零持久化进程重启 / 零磁盘写 / 日志纪律）+ 负载冒烟 1（20 对 × 10 轮，p50 1.8ms / p99 20ms）= 23 全绿，零警告。
- **零知识纪律落实**：tracing 只记 relay_id/角色/字节数/事件类别；日志纪律测试以 RUST_LOG=debug 全量捕获 stderr 断言哨兵载荷（hex + 原始）与双端公私钥 hex 均不入日志，且正向控制 relay_id 出现。
- **零持久化/零磁盘写**：真子进程（CARGO_BIN_EXE）重启断言 + 空临时工作目录递归扫描断言，覆盖 AC3。
- **依赖**：仅 `futures-util` 为 dev-dependency 增项（tokio-tungstenite 标准 Sink/Stream trait 伴生，非新功能依赖——story 指名「测试客户端用 tokio-tungstenite」其标准用法即需之）；生产依赖零超出 story 规格。
- **Docker**：本机无 Docker 二进制，按 story Task 8「无 Docker 环境如实说明并以 CI 构建代替验证」执行——release 二进制直接验证 `/healthz` 200 + 启动日志（含地址、不含密钥）+ SIGTERM 优雅退出；容器内 healthz 探活留待 CI `relay-docker.yml`。Dockerfile/compose/CI YAML 语法已校验。
- **桌面零回归**：`git diff --stat` 确认变更仅 `relay-server/**` + `.github/workflows/relay-docker.yml`（+ BMad 工件 sprint-status/故事文件）；`egosync-app/`、`crates/companion-proto/`、`companion-android/` 零触碰。

### File List

- `relay-server/Cargo.toml` [N]
- `relay-server/src/lib.rs` [N]
- `relay-server/src/main.rs` [N]
- `relay-server/src/registry.rs` [N]
- `relay-server/src/auth.rs` [N]
- `relay-server/src/forward.rs` [N]
- `relay-server/tests/common/mod.rs` [N]
- `relay-server/tests/relay.rs` [N]
- `relay-server/tests/relay_load.rs` [N]
- `relay-server/Dockerfile` [N]
- `relay-server/docker-compose.yml` [N]
- `relay-server/LOADTEST.md` [N]
- `.github/workflows/relay-docker.yml` [N]

### Change Log

- 2026-08-28：Story 12.3 完整实现——relay-server 无状态加密中继（axum 0.8 + 内存注册表 + XX 挑战-应答 + 纯二进制转发）；交付 23 测试全绿、Dockerfile/compose、CI 工作流、压测文档。零桌面改动。状态 → review。
- 2026-08-28：三路对抗代码评审（Blind Hunter / Edge Case Hunter / Acceptance Auditor）→ 1 项决策（D1 phone 槽公钥绑定，用户裁决 (a) 现在实现）+ 15 项 patch 全部修复 + 2 项 defer + 2 项 dismiss。核心整改：phone 槽公钥绑定防抢占、有界转发通道+写超时、鉴权整体超时预算、WS 帧上限 128KB（贴 E2E 密文帧上限）、服务端保活 Ping+空闲回收、优雅退出 Close 广播+8s 兜底、Dockerfile COPY lock+--locked+非 root、compose read_only/no-new-privileges/mem_limit、CI 镜像 tag 小写 owner+sha 版本、dev-dep tokio-tungstenite 0.29 对齐消除双版本、.gitignore 补 relay-server/target/、LOADTEST.md 命令修正+压测未执行标记、测试基建加固（stderr 读取线程/wait_ready 快速失败/压测参数 clamp）。新增 10 个回归测试，`cargo test` 33 个测试全绿（14 单测 + 18 集成 + 1 负载）。Docker 构建本机不可验证（无 Docker 二进制），留 CI relay-docker.yml 验证。状态 → done。
