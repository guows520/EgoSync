# noise-java 黄金向量生成 harness

## 用途

为 `tests/golden_vectors.rs` 产出 **noise-java 侧**黄金向量 fixtures
（`tests/fixtures/noise_java_vectors.json`），验证 snow（Rust）与 noise-java（Java）
在 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 上的跨语言加密互通——Epic 12 的最大技术风险出清点。

生成过程是一次**真实的跨语言握手**：

- noise-java 作为发起方（固定静态+临时密钥），spawn `cargo run --example gen_snow_responder`
  作为响应方（固定静态+临时密钥）；
- 双方经 stdin/stdout 行协议完成 XX 三步握手；
- 传输期双向互验：noise-java 加密的密文由 snow 解密回显校验；snow 加密的密文由
  noise-java 解密校验；
- 任一方向校验失败即抛异常，**不产出**向量文件（显式失败）。

## 前置

- JDK 17+（`java`/`javac`）
- cargo（stable 工具链，rustc ≥ 1.85）
- 网络（首次运行自动从 JitPack 下载 noise-java jar，固定 commit
  `49377b6dfc6a1e75740bce2318118291a57c0d6e`）

## 运行

```bash
cd crates/companion-proto/interop/noise-java
./run.sh
```

产物写入 `crates/companion-proto/tests/fixtures/noise_java_vectors.json`。

## 重新生成时机

仅在协议变更（bump protocolVersion / 更换 Noise 套件）时需要重新生成；
日常开发直接使用已提交的 fixtures（`cargo test` 自动校验，无需本 harness 与 JDK）。

## 目录说明

```text
GenVectors.java   # 生成器本体
run.sh            # 一键运行入口（编译 + 下载 jar + 执行）
lib/              # 运行时下载的 noise-java.jar（gitignore，不入库）
classes/          # javac 产物（gitignore，不入库）
```
