---
baseline_commit: d5345a6309fe0b06814d92dbd7216a92991f616e
---

# Story 12.1: `companion-proto` 协议 crate——帧 schema 冻结与跨语言加密互通

Status: done

## Story

As a 伴侣基建开发者,
I want 一个三端共享的加密协议 crate，冻结 8 种帧类型 schema 与 Noise XX 参数套件，并用黄金测试向量验证 snow↔noise-java 互通,
So that 桌面/中继/Android 三端在后续 Story 中只依赖同一份协议事实源，跨语言加密互通这一最大技术风险被最先出清。

## Acceptance Criteria（AC）

> 完整 AC 以 `_bmad-output/planning-artifacts/epics.md` Story 12.1 段为唯一事实源，以下为逐条搬运，编号供任务引用。

1. **AC1 目录与 crate 形态**：仓库根存在 `crates/companion-proto/`，含 `src/{lib.rs, frames.rs, crypto.rs}` 与 `src/schema.json`；package 名 `companion-proto`、lib 名 `companion_proto`；**未**在仓库根创建 Cargo workspace；`crypto.rs` 是全仓唯一依赖 `snow` 的位置。
2. **AC2 八种帧类型与往返编解码**：`HELLO / SNAPSHOT / STATE_DELTA / COMMAND / COMMAND_RESULT / STREAM_TOKEN / NOTICE / PING` 全部定义；枚举值为小写字符串、payload 字段 camelCase（serde `rename_all` 与基线一致）；编码为长度前缀二进制 Noise 传输消息；encode→decode 往返测试逐帧类型通过；`HELLO` 帧必含 `protocolVersion: u16`，缺版本字段的解码被拒绝。
3. **AC3 schema.json 单一事实源**：Noise 参数套件冻结为 `Noise_XX_25519_ChaChaPoly_BLAKE2s`；记录 8 种帧的 payload schema 与版本号；变更流程约束（bump protocolVersion + 同步 schema.json）以文档注释写入 `lib.rs`。
4. **AC4 snow 双角色自测**：发起方/响应方在进程内完成 Noise XX 握手、派生会话密钥、互发加密帧且解密一致，进 `cargo test`。
5. **AC5 跨语言黄金向量互验**：crate 测试内置由 snow 生成的握手转录 + 加密帧黄金向量 fixtures，且能验证由 noise-java 生成（开发期手动产出、随测试提交）的等价向量；两侧向量验证均进 `cargo test`，CI 自动执行。
6. **AC6 日志纪律（NFR-M7）**：crate 内所有 `tracing` 调用不输出帧内容明文与会话密钥材料。

## Tasks / Subtasks

- [x] Task 1：创建 `crates/companion-proto` crate 骨架（AC1）
  - [x] `crates/companion-proto/Cargo.toml`：package `companion-proto`、lib 名 `companion_proto`、edition 2021；依赖仅 `snow 0.10`（default features 即含 ChaChaPoly/BLAKE2s/curve25519）、`serde`（derive）、`serde_json`；dev-dependencies `tempfile`（按需）
  - [x] 确认仓库根**无** `Cargo.toml`、无 `[workspace]` 定义（现状已无，保持）
  - [x] `.gitignore` 追加 `crates/companion-proto/target/`（根 .gitignore 现无通用 target 规则）
- [x] Task 2：`frames.rs` 八种帧类型 + 长度前缀编解码（AC2）
  - [x] `FrameType` 枚举，serde 序列化为小写字符串（`#[serde(rename_all = "lowercase")]`）
  - [x] `Frame` 枚举，8 个变体对应 8 种帧；每个 payload 结构体字段 camelCase（`#[serde(rename_all = "camelCase")]`）；`HelloPayload` 含 `protocolVersion: u16`
  - [x] 传输消息格式：`[u32 BE 长度前缀][Noise 传输密文]`；提供 encode（帧→内层 JSON→交给 crypto 加密→加前缀）与 decode（剥前缀→解密→JSON 反序列化→帧）函数
  - [x] decode 对缺 `protocolVersion` 的 HELLO payload 返回错误
- [x] Task 3：`crypto.rs` Noise XX 封装（AC1、AC4）
  - [x] 封装 snow `Builder` 的握手编排（initiator/responder 两角色）与 `into_transport_mode` 后的加解密
  - [x] `snow` 依赖只出现在本文件（lib.rs/frames.rs 经由本模块类型间接使用，不 `use snow`）
  - [x] 暴露最小 API：握手驱动 + 会话加解密 + 常量 `NOISE_SUITE = "Noise_XX_25519_ChaChaPoly_BLAKE2s"`
- [x] Task 4：`schema.json` + `lib.rs` 变更流程文档（AC3）
  - [x] `src/schema.json`：noise 套件字符串、protocolVersion、8 种帧 payload 字段清单（字段名/类型）
  - [x] `lib.rs` 顶部 `//!` 文档注释写明：任何帧类型/payload 变更必须 bump protocolVersion 并同步 schema.json
- [x] Task 5：单测——帧往返 + 握手自测 + 日志纪律（AC2、AC4、AC6）
  - [x] `frames.rs` 同文件 `#[cfg(test)]`：8 种帧逐类型 encode→decode 往返断言；HELLO 缺版本拒绝断言
  - [x] `crypto.rs` 同文件 `#[cfg(test)]`：进程内 XX 双角色握手 + 派生密钥 + 互发加密帧往返（参考 Dev Notes 已验证代码骨架）
  - [x] 检查所有 `tracing::` 调用只记帧类型/长度/对端标识，不记 payload 明文与密钥字节
- [x] Task 6：黄金向量 fixtures 与互验测试（AC5）
  - [x] `tests/fixtures/` 提交两套向量：snow 生成（`snow_vectors.json`）与 noise-java 生成（`noise_java_vectors.json`）；格式含：双方静态密钥（hex）、握手消息序列（hex）、传输期密文与明文（hex）
  - [x] `tests/golden_vectors.rs`（或同文件集成测试）：① snow 向量字节级重放一致；② noise-java 向量经 snow 解密还原明文、snow 用相同密钥重加密后 noise-java 侧语义可验（以解密一致性为最低标准）
  - [x] 提交向量生成 harness（Java，noise-java 侧）与生成说明，位置建议 `crates/companion-proto/interop/noise-java/`（非 Cargo 成员，git 跟踪）
- [x] Task 7：CI 接线（AC5）
  - [x] `.github/workflows/ci.yml` 新增轻量 job `companion-proto`：ubuntu-latest，checkout → `dtolnay/rust-toolchain@stable` → `working-directory: crates/companion-proto` → `cargo test`
- [x] Task 8：验证收尾
  - [x] `cd crates/companion-proto && cargo test` 本地全绿（含黄金向量两套）
  - [x] `cd egosync-app && npm run build` 通过（桌面零回归——本 story 不触桌面代码，此项为保险断言）
  - [x] 向 sprint 汇报检查点 R 证据（握手自测 + 互验向量 + schema.json 冻结 + 日志纪律）

## Dev Notes

### 架构约束（硬边界，违反即返工）

- **加密边界**：Rust 侧 `snow` 仅允许出现在 `crates/companion-proto`（后续 src-tauri 与 relay-server 经 path 依赖复用）；本 story 后三端其余代码只操作帧类型，不见密码学细节。
- **禁止仓库根 Cargo workspace**（避免扰动基线构建行为）；crate 独立自带 `Cargo.lock` 与 `target/`。
- **帧协议即后续所有 story 的地基**：8 种帧冻结，扩展须走 schema 变更 + protocolVersion bump；本 story 不做任何"预留第 9 种帧"的投机设计（规则二：简单至上）。
- 本 story **纯新增**，不修改任何桌面既有文件（唯一例外：`.gitignore` 追加一行、`ci.yml` 新增 job）。现有 services 对伴侣一无所知（NFR-M4）。

### 关键技术情报（已在本机实测验证，2026-08-27）

**snow 0.10.0（crates.io 最新）**：
- 参数串 `"Noise_XX_25519_ChaChaPoly_BLAKE2s"` 可直接 parse；默认 feature 集已含 ChaChaPoly/BLAKE2s/curve25519，无需额外 feature。
- 本机 rustc 1.97.1 下 XX 握手 + transport 双向往返测试**已实跑通过**（临时探针项目，未入仓库）。可用骨架：

```rust
use snow::{Builder, Keypair};
let ik: Keypair = Builder::new(suite()).generate_keypair().unwrap();
let rk: Keypair = Builder::new(suite()).generate_keypair().unwrap();
let mut hi = Builder::new(suite()).local_private_key(&ik.private).unwrap().build_initiator().unwrap();
let mut hr = Builder::new(suite()).local_private_key(&rk.private).unwrap().build_responder().unwrap();
let mut buf = [0u8; 65535];
// XX 三步：-> e ； <- e,ee,s,es ； -> s,se
let n1 = hi.write_message(&[], &mut buf).unwrap(); let m1 = buf[..n1].to_vec();
hr.read_message(&m1, &mut buf).unwrap();
let n2 = hr.write_message(&[], &mut buf).unwrap(); let m2 = buf[..n2].to_vec();
hi.read_message(&m2, &mut buf).unwrap();
let n3 = hi.write_message(&[], &mut buf).unwrap(); let m3 = buf[..n3].to_vec();
hr.read_message(&m3, &mut buf).unwrap();
let (mut ti, mut tr) = (hi.into_transport_mode().unwrap(), hr.into_transport_mode().unwrap());
```

- **API 陷阱**（与旧文档示例不同，实测确认）：
  - `Builder::local_private_key` 返回 `Result<Builder>`（链上需 `.unwrap()`）；
  - `write_message`/`read_message` 返回 `usize`（写入长度），消息本体在 out 参数里，需 `buf[..n].to_vec()` 截取；
  - snow **不提供长度前缀/流式分帧**——前缀分帧是本 crate 自己的职责（`frames.rs`）。
- MSRV：snow 0.10 edition 2024 / rust-version 1.85。桌面 src-tauri 声明 rust-version 1.77.2，但 crate 独立编译不受其约束；CI 与本机 stable 工具链均满足。

**noise-java（rweather/noise-java，MIT，纯 Java）**——已核对上游源码：
- `Noise.createHash("BLAKE2s")`（纯 Java fallback `Blake2sMessageDigest`）、`createCipher("ChaChaPoly")`（`ChaChaPolyCipherState`）、`createDH("25519")`（`Curve25519DHState`）全部支持，即 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 可用（[来源：rweather/noise-java Noise.java](https://github.com/rweather/noise-java)）。
- 入口 `new HandshakeState(String protocolName, int role)`，协议名整串传入；分发经 [JitPack](https://jitpack.io/p/rweather/noise-java)（`com.github.rweather:noise-java`）。
- Android 侧接入属 Story 12.4，本 story 只需开发期 Java harness 产出向量。

**黄金向量生成方法（防坑）**：
- 双侧都要确定性密钥：静态密钥双侧固定（snow `local_private_key`；noise-java `DHState` 设固定 keypair）；snow 侧 ephemeral 用 `Builder::fixed_ephemeral_key_for_testing_only` 固定，noise-java 侧用其 DHState 固定 ephemeral 等价能力，否则 snow 向量无法字节级重放。
- 替代方案（若 noise-java 固定 ephemeral 困难）：snow→noise-java 方向只验"解密一致性"（snow 重放自己的转录 + 用 snow 解密 noise-java 密文），字节级一致仅对 snow 侧向量断言。以 AC5 原文为准：两侧向量"等价验证"，最低标准 = 解密正确。
- 时间戳/随机字段（如 pairing_nonce 类 payload）在向量中写死常量，保证可重放。

### 文件清单（本 story 全部产出）

```text
crates/companion-proto/
├── Cargo.toml
├── Cargo.lock                      # 随首次构建生成，提交
├── src/
│   ├── lib.rs                      # 变更流程文档注释 + 模块声明
│   ├── frames.rs                   # 8 帧类型 + 长度前缀编解码 + 同文件单测
│   ├── crypto.rs                   # snow 封装（唯一 use snow 处）+ 握手自测
│   └── schema.json                 # 协议单一事实源
├── tests/
│   ├── golden_vectors.rs           # 两套向量验证
│   └── fixtures/
│       ├── snow_vectors.json
│       └── noise_java_vectors.json
└── interop/noise-java/             # 开发期向量生成 harness（Java）+ 生成说明
.github/workflows/ci.yml            # [M] 新增 companion-proto job
.gitignore                          # [M] 追加 crates/companion-proto/target/
```

### 命名与规范（继承基线 100 条，NFR-M7）

- Rust：模块/文件 snake_case；结构体/枚举 PascalCase；常量 SCREAMING_SNAKE_CASE（`NOISE_SUITE`）；serde 全部 `rename_all = "camelCase"`；错误用类型化枚举（本 crate 内部 `ProtoError` 即可，AppError 变体属 Story 12.2，不要提前实现）。
- 公共 API 用 `///` 文档注释；注释中文。
- 测试验证意图而非仅行为（规则九）：帧往返测试断言"解码结果与原帧相等"（协议稳定性 WHY），HELLO 缺版本拒绝测试断言错误类型/信息（版本协商安全性 WHY）。

### 范围外（明确不做，防 scope creep）

- 不做 WS 传输、NSD、QR、paired_devices、relay-server、Android 接入——分别是 12.2/12.3/12.4。
- 不改 `egosync-app/` 任何代码；不为 crate 写 TS/Kotlin 类型生成器（架构"生成三语言类型"是后续演进，本 story 以 schema.json 事实源 + 向量互验为交付）。
- 不实现 AppError 新变体（PairingError/ConnectionError/ProtocolError 属 Story 12.2 桌面侧）。

### Previous Story Intelligence

Epic 12 首个 story，无同 epic 前驱。跨 epic 相关经验：
- `companion-android/` 原型已定稿四 Tab UI 与 `ConnectionClient` 接口（AGP 9 内置 Kotlin、手工 DI）；本 story 产出的帧协议必须能承载该 README 列出的换装点语义（SNAPSHOT 全量替换 + STATE_DELTA 增量 + COMMAND/STREAM_TOKEN），帧 payload 字段设计时以此为对照，但**不要**为 13.x 的具体业务字段做投机预定义——payload 本 story 用最小可测结构，字段冻结留给消费方 story 走 schema bump。
- 最近 git 历史（d5345a6 等）均为 companion-android 原型收尾与 Epic 12~14 规划文档，无 Rust crate 先例可抄——本 crate 是仓库首个独立 crate。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-12.1]（AC 唯一事实源）
- [Source: _bmad-output/planning-artifacts/epics.md#Requirements-Inventory（增量）]（Additional 1/4，帧类型集与配对信任链）
- [Source: _bmad-output/planning-artifacts/architecture.md#Incremental-Core-Architectural-Decisions—手机伴侣基建]（决策 1/2，四通道帧类型）
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation-Patterns-Addendum—手机伴侣基建]（命名/格式/通信模式、反模式清单）
- [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-Addendum—手机伴侣基建]（目录树、四条硬边界）
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-stories-12-1-...md]（检查点 R、测试范围、依赖图）
- [Source: _bmad-output/project-context.md]（基线 100 条规范）
- [snow 0.10.0 API（docs.rs）](https://docs.rs/snow) / [rweather/noise-java](https://github.com/rweather/noise-java)（外部技术情报，已实测/核对）

## Dev Agent Record

### Agent Model Used

glm-5.3（DeepSeek Harness，Amelia 角色）

### Debug Log References

- snow 0.10.0 API 实测修正：`Builder::new` 直接返回 Builder（非 Result）；`NoiseParams` 经 `snow::params::NoiseParams` 引用（未从 crate 根 re-export）；`write_message`/`read_message` 返回 `usize`。
- noise-java（JitPack commit `49377b6dfc`）实测修正：传输态 API 为 `split()` → `CipherStatePair.getSender()/getReceiver()`，无 `TransportState` 类；`decryptWithAd` 参数序为 `(ad, ciphertext, ctOffset, plaintext, ptOffset, length)`（与 `encryptWithAd` 不同）；握手完成后 `getAction()` 返回 `SPLIT`，调用 `split()` 后才为 `COMPLETE`。
- 生成期互验通过后向量文件才允许产出（GenVectors.java 显式失败设计）。

### Completion Notes List

- **TDD 执行**：frames encode/decode 严格走 RED（todo!() 占位，3 测试 panic）→ GREEN（实现后 6/6 绿）→ 重构（clippy 零告警）。
- **⚠️ 冲突裁决（规则七）**：Story Task 2 字面写 `rename_all = "lowercase"`，但 PascalCase 变体 `StateDelta` 经 lowercase 得 `"statedelta"`（丢下划线），与架构/epics 规范帧名 `STATE_DELTA` 的小写形式 `"state_delta"` 冲突。已择 `rename_all = "snake_case"`（`StateDelta` → `"state_delta"`）：与规范帧名一致、仍为小写字符串、保留 Rust 枚举 PascalCase 约定。**story 字面提示 `lowercase` 为被取代项**，请评审确认。
- **AC5 超出最低标准**：noise-java 支持 `HandshakeState.getFixedEphemeralKey()`，双侧四把密钥（静态+临时）全部固定，因此互验达到**双向字节级**（snow 逐字节复现 noise-java 的握手转录与密文），强于 Dev Notes 预设的降级方案"仅解密一致性"。
- **AC6 日志纪律**：crate 内 `tracing` 调用数为 0（story 规定依赖清单仅 snow/serde/serde_json，未含 tracing）——不输出任何帧明文/密钥材料，纪律天然满足；后续 story 若引入日志须遵守"只记帧类型/长度/对端标识"。
- **超出 story 文件清单的 .gitignore 追加**：`interop/noise-java/{lib,classes}/` 两行——避免 126KB jar 与 class 文件入库（harness README 已说明下载方式），属必要配套，请评审知悉。
- **CI**：`companion-proto` job 为独立轻量 job（ubuntu-latest，无矩阵），黄金向量两套互验随 `cargo test` 自动执行。
- 桌面零回归验证：`egosync-app npm run build` 通过（tsc 零类型错误）；本 story 未触碰 `egosync-app/` 任何代码（CI job 与 .gitignore 除外）。

### File List

- crates/companion-proto/Cargo.toml（新增）
- crates/companion-proto/Cargo.lock（新增，首次构建生成）
- crates/companion-proto/src/lib.rs（新增）
- crates/companion-proto/src/frames.rs（新增）
- crates/companion-proto/src/crypto.rs（新增）
- crates/companion-proto/src/schema.json（新增）
- crates/companion-proto/examples/gen_snow_vectors.rs（新增）
- crates/companion-proto/examples/gen_snow_responder.rs（新增）
- crates/companion-proto/tests/golden_vectors.rs（新增）
- crates/companion-proto/tests/fixtures/snow_vectors.json（新增）
- crates/companion-proto/tests/fixtures/noise_java_vectors.json（新增）
- crates/companion-proto/interop/noise-java/GenVectors.java（新增）
- crates/companion-proto/interop/noise-java/run.sh（新增）
- crates/companion-proto/interop/noise-java/README.md（新增）
- .github/workflows/ci.yml（修改：新增 companion-proto job）
- .gitignore（修改：追加 companion-proto target/ 与 interop lib/classes/ 忽略规则）

### Change Log

- 2026-08-27：Story 12.1 实现完成——companion-proto 协议 crate（8 帧冻结 + Noise XX 封装 + schema.json 事实源）、黄金向量双向字节级互验（snow + noise-java）、CI 接线；全部 8 个 Task 完成，cargo test 8/8 绿，桌面构建零回归。
- 2026-08-27：代码审查完成——三层对抗审查（Blind Hunter / Edge Case Hunter / Acceptance Auditor），AC1–AC6 全部 PASS、rename_all 裁决 CONFIRMED-CORRECT；12 项 Review Findings 全部修补落地：帧尺寸上限（常量+前置校验+schema 记录+超限测试）、私钥长度校验、deny_unknown_fields、protocolVersion 取值协商（裁决 A）、黄金向量非空断言、from_hex 偶数断言、run.sh curl -f + sha256 校验、GenVectors stderr INHERIT + 判空、多词帧名抽查改 StateDelta、Cargo.lock 入暂存、snow_vectors.json 补尾换行。修补后 `cargo test` 13/13 绿（11 单测 + 2 集成）、clippy 零告警、`egosync-app npm run build` 桌面零回归。

### Review Findings

> 三层对抗审查（Blind Hunter / Edge Case Hunter / Acceptance Auditor），2026-08-27。AC1–AC6 全部 PASS；`rename_all = "snake_case"` 冲突裁决经审计确认正确（CONFIRMED-CORRECT）；额外 .gitignore 两行可接受。以下 1 项需决策、11 项需修补、0 项延期、0 项驳回。

- [x] [Review][Patch] protocolVersion 取值校验落地（原 Decision-1，2026-08-27 裁决选 A）— `decode_frame` 增加 `protocol_version == PROTOCOL_VERSION` 校验，版本不匹配返回 `ProtoError::Decode`；补一条"版本不匹配被拒绝"测试；使测试注释宣称的"版本协商安全性"真正闭环。[crates/companion-proto/src/frames.rs]
- [x] [Review][Patch] 帧尺寸上限矛盾：u32 前缀承诺 4GiB，但 snow 单消息上限 65535（明文实为 65519）；超限报 `Crypto("input error")`，类别与信息皆错；schema.json 未记录上限、无超限路径测试；SNAPSHOT 全量快照 >64KiB 是真实场景 [crates/companion-proto/src/crypto.rs; src/frames.rs; src/schema.json]
- [x] [Review][Patch] 静态私钥长度不校验：>32 字节在 snow 内部切片越界 panic（Result 签名形同虚设）；<32 字节静默零填充、握手照常，实际以另一把密钥建立错钥会话。建议入口校验 len==32 [crates/companion-proto/src/crypto.rs:HandshakeSession::build]
- [x] [Review][Patch] payload 未知字段被静默接受（`{"type":"ping","data":"x"}` 解码成功），"冻结"在解码层不可强制；已实证 `#[serde(deny_unknown_fields)]` 与 `tag="type"` 兼容，可直接加 [crates/companion-proto/src/frames.rs]
- [x] [Review][Patch] 黄金向量 transport 为空数组时测试空洞通过，AC5 互验可名存实亡；应断言非空且 i2r/r2i 各至少 1 条 [crates/companion-proto/tests/golden_vectors.rs:replay_and_verify]
- [x] [Review][Patch] from_hex 对奇数长度 hex 静默截断末半字节（两处），可能掩盖向量损坏根因；应断言偶数长度 [crates/companion-proto/tests/golden_vectors.rs; examples/gen_snow_responder.rs]
- [x] [Review][Patch] run.sh curl 无 `--fail`，JitPack 404/500 HTML 错误页被写成 jar 且永久缓存，后续报无关错误；应加 `-fL` 并在失败时清理残文件 [crates/companion-proto/interop/noise-java/run.sh]
- [x] [Review][Patch] GenVectors.java 不消费子进程 stderr（cargo 编译输出写满管道可致生成器永久挂起）；snow 子进程早退时 `readLine()` 返回 null 抛 NPE，吞掉真正根因；应 `redirectError(INHERIT)` + 判 null 抛带上下文的 IllegalStateException [crates/companion-proto/interop/noise-java/GenVectors.java]
- [x] [Review][Patch] noise-java jar 从 JitPack 下载无 sha256 校验（本机执行的密码学依赖，开发期供应链风险）；应写死预期摘要并校验 [crates/companion-proto/interop/noise-java/run.sh]
- [x] [Review][Patch] "多词帧名"同值抽查实际用的是单词帧 `ping`，测不到 snake_case 分野；应改用 `StateDelta`/`CommandResult` 断言 `state_delta`/`command_result` [crates/companion-proto/src/frames.rs:frame_type_serializes_as_lowercase_snake_string]
- [x] [Review][Patch] Cargo.lock 未入暂存（磁盘存在 15209B，但 crates/ 整体 untracked）；story 文件清单声称已交付、硬边界要求随库提交；字节级黄金向量对 snow 版本敏感，无锁文件时 CI 每次取最新兼容 0.10.x 可能漂移。提交前 `git add` [crates/companion-proto/Cargo.lock]
- [x] [Review][Patch] snow_vectors.json 末尾缺换行符 [crates/companion-proto/tests/fixtures/snow_vectors.json]
