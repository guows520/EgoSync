#!/usr/bin/env bash
# noise-java 黄金向量生成入口（开发期手动运行一次，产物随仓库提交）。
# 前置：JDK 17+、cargo（PATH 中）、网络（首次自动下载 noise-java jar）。
# 运行：cd crates/companion-proto/interop/noise-java && ./run.sh
set -euo pipefail
cd "$(dirname "$0")"
CRATE_ROOT="$(cd ../.. && pwd)"

EXPECTED_SHA256="c01a0dbd33a6f700ae9d7d96d377a3c78f4c883b93689465f8abbfccf2e2e255"

mkdir -p lib
if [ ! -f lib/noise-java.jar ]; then
  echo "下载 noise-java jar（JitPack，固定 commit）..."
  curl -fsSL --retry 3 -o lib/noise-java.jar \
    'https://jitpack.io/com/github/rweather/noise-java/49377b6dfc6a1e75740bce2318118291a57c0d6e/noise-java-49377b6dfc6a1e75740bce2318118291a57c0d6e.jar' || {
    rm -f lib/noise-java.jar
    echo "错误：下载 noise-java jar 失败" >&2
    exit 1
  }
  # sha256 校验（开发期供应链完整性）
  ACTUAL_SHA256="$(sha256sum lib/noise-java.jar | cut -d' ' -f1)"
  if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
    rm -f lib/noise-java.jar
    echo "错误：noise-java jar sha256 不匹配（期望 $EXPECTED_SHA256，实际 $ACTUAL_SHA256）" >&2
    exit 1
  fi
fi

mkdir -p classes
javac -encoding UTF-8 -cp lib/noise-java.jar -d classes GenVectors.java
java -cp "lib/noise-java.jar:classes" GenVectors "$CRATE_ROOT"
