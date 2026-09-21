# egosync-server 多阶段构建（Story 17.1，epic AC 1/8）。
#
# ⚠️ build context 必须为仓库根（path 依赖 ../crates/egosync-engine 需
#   可拷入；relay-server/Dockerfile 同款写法）：
#   docker build -f server/Dockerfile -t egosync-server .
#
# 三阶段：
#   1. node 构建前端 dist（全局 dist/ 在 .gitignore——CI checkout 无
#      产物，必须在镜像内构建；transport 工件已提交入库，前端阶段零
#      cargo 依赖）；
#   2. rust:1-bookworm 构建 server（--locked、依赖层与源码层分开缓存）；
#   3. debian:bookworm-slim 运行时（opencode v1.15.10 + tzdata +
#      ca-certificates + wget + 非 root + /data 属主预置）。
#
# opencode 版本 pin：与 .github/workflows/release.yml 三处硬编码一致
#（v1.15.10）；TARGETARCH 参数化下载（linux/amd64 + linux/arm64——上游
# 有 arm64 资产）。tarball 内容经 sha256 校验（17.1 评审 #13——密钥
# 安全主题的故事不裸信传输层：值取自 GitHub Releases 官方 asset digest
# api.github.com/repositories/975734319/releases/tags/v1.15.10）。

ARG NODE_IMAGE=node:22-bookworm-slim
ARG RUST_IMAGE=rust:1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim
# 与 release.yml 的 OPENCODE_VERSION 保持一致（验证步骤比对两处）
ARG OPENCODE_VERSION=v1.15.10

# ── stage 1：前端 dist（node + npm ci，缓存友好） ──
FROM ${NODE_IMAGE} AS frontend
WORKDIR /web
# 依赖层：package-lock 变更才重装
COPY egosync-app/package.json egosync-app/package-lock.json ./
RUN npm ci
# 源码层：全量拷贝（构建上下文经 .dockerignore 裁剪——无 node_modules/
# dist/target 等本地产物），vite 无 TAURI_ENV_PLATFORM 时走 safari13
# target 分支，双宿主探测（isTauriHost）在运行时完成。
# postcss.config.js：tailwind 管线必需；scripts/：src/transport/events.test.ts
# 经 gen-transport.d.mts 类型引用（tsc include src 面内）。
COPY egosync-app/index.html egosync-app/tailwind.config.js egosync-app/postcss.config.js egosync-app/tsconfig.json egosync-app/tsconfig.node.json egosync-app/vite.config.ts ./
COPY egosync-app/scripts ./scripts
COPY egosync-app/src ./src
RUN npm run build

# ── stage 2：server 构建（依赖层与源码层分开缓存，relay-server 模式） ──
FROM ${RUST_IMAGE} AS builder
WORKDIR /build

# 依赖层：只拷 Cargo.toml + Cargo.lock + 虚拟 src，最大化缓存命中
#（COPY Cargo.lock + --locked：镜像与 CI cargo test 的依赖基线一致）
COPY server/Cargo.toml server/Cargo.lock ./server/
COPY crates/egosync-engine/Cargo.toml ./crates/egosync-engine/Cargo.toml
COPY crates/companion-proto/Cargo.toml ./crates/companion-proto/Cargo.toml
RUN mkdir -p server/src/bin crates/egosync-engine/src crates/companion-proto/src \
    && echo 'fn main() {}' > server/src/main.rs \
    && echo 'pub fn stub() {}' > server/src/lib.rs \
    && echo 'pub fn stub() {}' > crates/egosync-engine/src/lib.rs \
    && echo 'pub fn stub() {}' > crates/companion-proto/src/lib.rs \
    && cargo build --release --locked --manifest-path server/Cargo.toml \
    && rm -rf server/src crates/egosync-engine/src crates/companion-proto/src

# 源码层：真实源码增量构建。
# touch 抬升源码 mtime 到 stub 构建之后——否则 COPY 保留的原始 mtime
# 早于指纹记录时刻，cargo 会误判「无变化」而 0.5s 空构建（镜像里残留
# stub 二进制；经典构建器与 BuildKit 同坑，本机实测踩中）。
COPY crates/companion-proto ./crates/companion-proto
COPY crates/egosync-engine ./crates/egosync-engine
COPY server/src ./server/src
RUN find server/src crates/egosync-engine/src crates/companion-proto/src \
        \( -name '*.rs' -o -name '*.sql' \) -exec touch {} + \
    && cargo build --release --locked --manifest-path server/Cargo.toml \
    && strip server/target/release/egosync-server

# ── stage 3：运行时（opencode + tzdata + ca-certificates + 非 root） ──
FROM ${RUNTIME_IMAGE} AS runtime
ARG OPENCODE_VERSION
# TARGETARCH 由 BuildKit 多平台构建自动注入（linux/amd64 / linux/arm64）；
# 传统构建器不注入——回退 uname -m 推导本机架构（单平台构建等价）；
# 注入非受支持值则硬失败（见下方 RUN 块注释，17.1 评审 #19）
ARG TARGETARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends wget ca-certificates tzdata xz-utils \
    && rm -rf /var/lib/apt/lists/*

# opencode 二进制（pin v1.15.10，TARGETARCH 参数化：amd64 / arm64）
# 放置形态与引擎资源搜索序对齐：SidecarManager 以 EGOSYNC_OPENCODE_PATH
# 父目录为 resource_dir，搜索 {dir}/resources/opencode → {dir}/opencode。
# sha256 取自 GitHub Releases 官方 asset digest（升级 OPENCODE_VERSION
# 时须同步更新两值）。
# TARGETARCH 语义（17.1 评审 #19）：amd64/arm64 直用；**未注入**（经典
# 构建器）→ 宿主 uname 推导；注入了**非受支持值**（如 buildx 指定
# arm/v7）→ 硬失败——静默回退宿主架构会把 x64 二进制烙进异架构镜像，
# sidecar 降级无声（宁可构建期失败）。
RUN case "${TARGETARCH}" in \
        amd64|arm64) ;; \
        "") if [ "$(uname -m)" = "x86_64" ]; then TARGETARCH=amd64; \
            elif [ "$(uname -m)" = "aarch64" ]; then TARGETARCH=arm64; \
            else echo "unsupported host arch: $(uname -m)" >&2; exit 1; fi ;; \
        *) echo "unsupported TARGETARCH: ${TARGETARCH}（受支持：amd64 / arm64）" >&2; exit 1 ;; \
    esac \
    && case "${TARGETARCH}" in \
        amd64) OPENCODE_ARCH=x64; OPENCODE_SHA256=a4c0c94a7fdbf637e3ae479c046ca49e925370b4cee503dfba7ab677a13cd0c5 ;; \
        arm64) OPENCODE_ARCH=arm64; OPENCODE_SHA256=def583a68cb155cddd9523e52139a5b915e981eef17a274c8c8171a3ee828ae2 ;; \
    esac \
    && wget -q --no-verbose -O /tmp/opencode.tar.gz \
        "https://github.com/sst/opencode/releases/download/${OPENCODE_VERSION}/opencode-linux-${OPENCODE_ARCH}.tar.gz" \
    && echo "${OPENCODE_SHA256}  /tmp/opencode.tar.gz" | sha256sum -c - \
    && mkdir -p /tmp/opencode-extract /app/resources \
    && tar -xzf /tmp/opencode.tar.gz -C /tmp/opencode-extract \
    && OPENCODE_BIN=$(find /tmp/opencode-extract -name "opencode*" -type f ! -name "*.exe" | head -1) \
    && [ -n "${OPENCODE_BIN}" ] \
    && mv "${OPENCODE_BIN}" /app/resources/opencode \
    && chmod +x /app/resources/opencode \
    && /app/resources/opencode --version \
    && rm -rf /tmp/opencode.tar.gz /tmp/opencode-extract

# server 二进制 + 前端 dist
COPY --from=builder /build/server/target/release/egosync-server /app/egosync-server
COPY --from=frontend /web/dist /app/static

# 非 root 运行（65534）+ /data 属主预置（egosync-server 需在 /data 建
# 双库与 secrets.json；容器内 uid 无名——与镜像层 uid 一致即可）
RUN mkdir -p /data && chown 65534:65534 /data \
    && chown -R 65534:65534 /app
USER 65534:65534

# HOME 指向 /data（uid 65534 无系统 home——Docker 缺省 HOME=/nonexistent
# 会让 opencode（bun 运行时）启动期 mkdir $HOME 直接 EACCES 降级）；
# 与引擎侧 home_dir=data_dir 对齐，可写且随卷持久。
ENV HOME=/data \
    EGOSYNC_HOST=0.0.0.0 \
    EGOSYNC_PORT=8080 \
    EGOSYNC_DATA_DIR=/data \
    EGOSYNC_STATIC_DIR=/app/static \
    EGOSYNC_OPENCODE_PATH=/app/resources/opencode \
    RUST_LOG=info
EXPOSE 8080
CMD ["/app/egosync-server"]
