# relay-server 压测流程（AC5）

> AC5 验收口径：「单 VPS（$5/月级）场景下数百并发长连接稳定、内存占用与连接数线性、无随时间增长趋势、转发延迟在可接受范围」。
> 验收形态取「可复用测试 + 文档化流程」（见 story 裁决 #4），即：CI 跑小规模冒烟，VPS 跑同一测试放大规模。
>
> ⚠️ **压测状态（2026-08-28 评审整改时点）**：CI 冒烟（20 对 × 10 轮）已随
> `relay-docker.yml` 每次推送执行；**VPS 300 对大压测尚未执行**——待实际部署后
> 按本流程执行并回填下方观测数据，在此之前「数百并发、内存线性」零实测证据。

## 复用的测试

`tests/relay_load.rs::concurrent_pair_forwarding_load`：N 对并发长连接 × M 轮双向
逐字节转发，断言每对每轮一致，并记录总耗时与单条转发延迟 p50/p99。

规模经环境变量可调：

| 变量 | 默认（CI） | 说明 |
|------|------------|------|
| `RELAY_LOAD_PAIRS` | 20 | 并发连接对数（每对 = 一桌面 + 一手机） |
| `RELAY_LOAD_ROUNDS` | 10 | 每对双向转发的轮数 |

## 本地 / CI 小规模冒烟

```bash
cd relay-server
cargo test --test relay_load -- --nocapture
```

预期（CI 默认规模，20×10=200 条，1KB 载荷）：

```
负载冒烟: pairs=20 rounds=10 payload=1KB 消息数=200 总耗时=~3.4s p50=~1.8ms p99=~20ms
```

> 数字会随机器波动；冒烟只断言「逐字节一致 + 无死锁」，不卡绝对延迟。

## VPS 大压测流程（$5/月级 VPS）

目标：验证「数百并发长连接稳定、内存随连接数线性、无随时间增长趋势」。

### 1. 拉起 relay-server

```bash
# 二进制已构建（docker build 或 cargo build --release）
RELAY_HOST=0.0.0.0 RELAY_PORT=7333 ./relay-server &
```

### 2. 以 300 对运行同一测试

```bash
cd relay-server
RELAY_LOAD_PAIRS=300 RELAY_LOAD_ROUNDS=5 cargo test --release --test relay_load -- --nocapture
```

- 300 对 = 600 条长连接（每对一桌面 + 一手机）；
- 该测试二进制仅 1 个用例，测试框架并发与压测语义本不冲突，无需 `--test-threads`。
- 轮数从 10 收到 5，缩短大压测耗时，聚焦「并发连接数」这一核心指标。

### 3. 观测指标

在另一终端对 relay-server 进程（PID=`$RELAY_PID`）持续采样：

| 指标 | 命令 | 验收口径 |
|------|------|----------|
| RSS 内存 | `ps -o rss= -p $RELAY_PID` | 与连接数**线性**：空载约 5–15MB，每百连接增长应在数 MB 量级 |
| 连接数 | `ss -tnp \| grep 7333 \| wc -l` | 应稳定在 ≈ 2×pairs（每对两端） |
| p50/p99 转发延迟 | 测试输出行 | p50 应在低个位数 ms、p99 在数十 ms 量级（VPS 网络） |
| 内存随时间增长趋势 | 连续采样 RSS | 全程转发过程中 RSS **无单调增长趋势**（零持久化：断线即丢，注册表不积累） |

### 4. 压测后回归断言

压测结束后，注册表应回到空（零持久化）。可直接复用 `tests/relay.rs` 中的
`zero_persistence_across_process_restart` 与 `zero_disk_write_in_subprocess`
作为压测后回归——任何残留即违反 AC3。

## 为什么不用 criterion 基准测试

criterion 适合测「单次操作吞吐」，但 AC5 的核心是「长连接内存线性 + 无增长
趋势」这类**进程级**指标——集成测试形态（持有真实长连接、断言逐字节一致、
附带耗时采样）更贴近验收口径（裁决 #4）。
