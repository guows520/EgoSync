# relay-server 多阶段构建（AC4）。
#
# ⚠️ build context 必须为仓库根（path 依赖 ../crates/companion-proto 需可拷入）：
#   docker build -f relay-server/Dockerfile -t egosync-relay .

ARG RUST_IMAGE=rust:1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

# ── stage 1：构建（依赖层与源码层分开缓存） ──
FROM ${RUST_IMAGE} AS builder
WORKDIR /build

# 依赖层：只拷 Cargo.toml + Cargo.lock + 虚拟 src，最大化缓存命中
#（COPY Cargo.lock + --locked：镜像与 CI cargo test 的依赖基线一致，可复现构建）
COPY relay-server/Cargo.toml relay-server/Cargo.lock ./relay-server/
COPY crates/companion-proto/Cargo.toml ./crates/companion-proto/Cargo.toml
RUN mkdir -p relay-server/src crates/companion-proto/src \
    && echo 'fn main() {}' > relay-server/src/main.rs \
    && touch relay-server/src/lib.rs crates/companion-proto/src/lib.rs \
    && cargo build --release --locked --manifest-path relay-server/Cargo.toml \
    && rm -rf relay-server/src crates/companion-proto/src

# 源码层：真实源码增量构建
COPY crates/companion-proto ./crates/companion-proto
COPY relay-server/src ./relay-server/src
RUN cargo build --release --locked --manifest-path relay-server/Cargo.toml

# ── stage 2：运行时（单二进制，slim 基底 + wget 探活） ──
FROM ${RUNTIME_IMAGE}
RUN apt-get update \
    && apt-get install -y --no-install-recommends wget ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/relay-server/target/release/relay-server /usr/local/bin/relay-server

# 非 root 运行（评审加固：容器逃逸面最小化；端口 7333 > 1024 无需特权，
# 零磁盘写纪律下亦无需要写入的路径）
USER 65534:65534

ENV RELAY_HOST=0.0.0.0 \
    RELAY_PORT=7333 \
    RUST_LOG=info
EXPOSE 7333
CMD ["relay-server"]
