//! AC5 压测验证：并发长连接转发冒烟。
//!
//! CI 友好规模（默认 20 对 × 10 轮），规模经 `RELAY_LOAD_PAIRS` /
//! `RELAY_LOAD_ROUNDS` 可调——VPS 大压测复用同一测试（见 LOADTEST.md）。
//!
//! 测试意图（规则九）：数百并发长连接下「每一对、每一轮」都必须逐字节一致——
//! 压测不是测吞吐数字，是测高并发下纯转发语义不退化。

mod common;

use std::time::{Duration, Instant};

use futures_util::future::join_all;

use common::{InProcessServer, RelayClient};
use relay_server::auth::derive_relay_id;
use relay_server::build_router;

fn env_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// N 对并发长连接 × M 轮双向转发，全部逐字节一致 + 记录耗时与延迟分布。
#[tokio::test]
async fn concurrent_pair_forwarding_load() {
    // WHY: 单 VPS（$5/月级）部署场景要求数百并发长连接稳定——CI 里以小规模
    // 验证语义正确性与无死锁，VPS 上放大同一测试验证规模上限（AC5）。
    // env 误设 0 时不得触发 samples 越界 panic——钳到最小 1（评审 P14 整改）。
    let pairs = env_or("RELAY_LOAD_PAIRS", 20).max(1);
    let rounds = env_or("RELAY_LOAD_ROUNDS", 10).max(1);
    let payload = vec![0xABu8; 1024]; // 1KB 密文帧（快照帧量级）

    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let started = Instant::now();
    let tasks = (0..pairs).map(|pair_idx| {
        let url = url.clone();
        let payload = payload.clone();
        async move {
            let (d_priv, d_pub) = common::keypair();
            let relay_id = derive_relay_id(&d_pub);
            let (p_priv, _) = common::keypair();

            let mut desktop =
                RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
            let mut phone =
                RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

            // 每轮：desktop→phone 单向延迟（逐字节一致断言含在内）
            let mut latencies = Vec::with_capacity(rounds);
            for round in 0..rounds {
                let sent_at = Instant::now();
                desktop.send_binary(&payload).await;
                let got = phone.recv_binary().await;
                latencies.push(sent_at.elapsed());
                assert_eq!(
                    got, payload,
                    "pair {pair_idx} round {round} 转发必须逐字节一致"
                );
            }
            latencies
        }
    });
    let all = join_all(tasks).await;

    let total = started.elapsed();
    let mut samples: Vec<Duration> = all.into_iter().flatten().collect();
    samples.sort();
    let p50 = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];

    // 结果摘要（cargo test -- --nocapture 可见；VPS 压测记录此行）
    println!(
        "负载冒烟: pairs={pairs} rounds={rounds} payload=1KB 消息数={} 总耗时={total:?} p50={p50:?} p99={p99:?}",
        samples.len()
    );
}
