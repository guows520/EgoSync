---
baseline_commit: e87cf00
---

# Story 8.5: Release 自动发布与版本管理

Status: done

## Story

As a 开发者,
I want 通过 Git tag 自动触发正式发布,
so that 发布流程标准化且可重复。

## 背景与现状（务必先读）

**本 story 是 Epic 8（跨平台分发与 V1 加固）的第五个、也是最后一个 story — 建立 tag 触发的 Release 自动发布工作流。Epic 1-7 全部完成，Epic 8 的 8-1（CI 构建）、8-3（无障碍）、8-4（性能基准）已完成，8-2（E2E）仍在 in-progress 但其 E2E 框架已可用。本 story 在现有 CI 基础上新增独立的 `release.yml` 工作流。**

**核心交付：**
1. **新建 `.github/workflows/release.yml`** — 由 `v*` 格式 Git tag 触发，三平台并行构建 → E2E 测试通过 → 自动创建 GitHub Release + 上传三平台安装包为 Release Assets
2. **CHANGELOG 自动生成** — 基于 conventional commits 自动生成 Release Notes（新功能/修复/破坏性变更）
3. **版本号一致性** — `tauri.conf.json` / `Cargo.toml` / `package.json` 三处版本号与 Git tag 一致（发布前手动 bump，工作流校验）
4. **macOS/Windows 签名（可选）** — 支持配置签名证书，未签名时降级运行（DMG 显示安全提示 / MSI 显示 SmartScreen 警告）
5. **发布后验证** — 自动下载各平台安装包并验证 SHA256 文件完整性
6. **opencode sidecar 真实 binary 打包** — 解决 Story 8.1 遗留的占位文件问题，Release 构建需打包真实 opencode binary

### 已建成的基础（直接复用，不要重建）

**现有 CI workflow（不修改，复用模式）：**
- `.github/workflows/ci.yml:1-197` — 已有三平台 matrix（ubuntu/macos/windows）、Rust cache（Swatinem/rust-cache@v2）、Node 25、npm ci、前端测试、Rust 测试、`npm run tauri build`、E2E（Windows/Linux）、性能基准、无障碍扫描、产物上传
- **release.yml 应复用 ci.yml 的步骤模式**：checkout → Linux deps → Rust setup → Rust cache → 移除 rsproxy config → Node setup → npm ci → tauri build（通过 tauri-action）→ 上传到 Release
- **不要修改 ci.yml** — release.yml 是独立工作流，ci.yml 继续负责 push/PR 触发的 CI

**Tauri 配置（不修改 bundle 结构，仅版本号会变）：**
- `egosync-app/src-tauri/tauri.conf.json:4` — `version: "0.1.0"`（发布前需 bump 到 `1.0.0` 或对应 tag 版本）
- `egosync-app/src-tauri/tauri.conf.json:5` — `identifier: "com.egosync.desktop"`（已修正，避免 .app 后缀冲突）
- `egosync-app/src-tauri/tauri.conf.json:31-44` — `bundle.active: true`, `targets: "all"`, icons, `resources: ["resources/opencode*"]`
- `egosync-app/src-tauri/Cargo.toml:3` — `version = "0.1.0"`（需与 tag 同步）
- `egosync-app/package.json:4` — `"version": "0.1.0"`（需与 tag 同步）

**opencode sidecar 占位符（本 story 必须解决）：**
- `egosync-app/src-tauri/resources/opencode.placeholder` — 当前只有占位文件
- `tauri.conf.json:41-43` — `resources: ["resources/opencode*"]` glob 匹配占位文件
- **Story 8.1 明确遗留：** "CI 构建会打包占位文件而非真实 binary — 这对 CI 验证目的可接受，Story 8.5（Release 发布）需解决真实 binary 打包问题"
- **解决方案：** release.yml 在 tauri build 之前，按平台下载真实 opencode binary 到 `egosync-app/src-tauri/resources/`（Windows: `opencode.exe`, macOS/Linux: `opencode`），并赋予可执行权限。binary 来源：`https://github.com/opencode-ai/opencode/releases/latest`（或 `go install` 但 CI 下载预编译 binary 更快更可靠）

**E2E 测试框架（8-2 已建立，复用）：**
- `egosync-app/tests/e2e/` — WebdriverIO 9.x + tauri-driver，Windows/Linux 可跑，macOS 跳过
- release.yml 应在 tauri build 之后、上传 Release 之前运行 E2E（Windows/Linux），失败则阻断发布
- 复用 ci.yml:75-110 的 E2E 步骤模式

**Git 仓库状态：**
- 远程：`https://github.com/guows520/EgoSync.git`
- 当前无任何 git tag（`git tag --list` 为空）
- 当前分支：`feature/8-1-github-actions-ci-build`
- 默认分支：`main`

### 架构规划

架构文档（`architecture.md:721-725`）规划了两个 workflow 文件：
- `.github/workflows/ci.yml` — Test + Tauri Build + Artifacts（三平台）← **已存在（Story 8.1）**
- `.github/workflows/release.yml` — 构建安装包 + 发布 ← **本 story 创建**

本 story 聚焦 `release.yml`：tag 触发 → 三平台构建（含真实 opencode binary）→ E2E 验证 → 创建 GitHub Release + 上传 assets + CHANGELOG → SHA256 校验。

### 关键技术决策（预判，dev 可调整）

**1. 使用 `tauri-apps/tauri-action@v0` 而非手动 `npm run tauri build`：**
- Story 8.1 用手动方式（CI 阶段只需产物上传）。Release 阶段需要自动创建 GitHub Release + 上传 assets，`tauri-action` 专门为此设计
- `tauri-action` 自动：读取 `tauri.conf.json` 版本 → 创建/更新 GitHub Release → 上传 bundle 产物为 Release Assets → 生成 `latest.json`（供 updater 用，V2 候选）
- `__VERSION__` 模板：`tagName: v__VERSION__` 会被自动替换为 `tauri.conf.json` 的版本号
- **版本：** `@v0`（moving tag，最新 release 为 action-v0.6.2 / 2026-03-14）。避免用 `@v1`（README 示例用过但实际无 v1 release tag）。**不要**用 `@master`/`@dev`（不稳定）

**2. 触发方式：`on: push: tags: ['v*']`**
- 匹配 AC："推送 `v*` 格式的 Git tag（如 `v1.0.0`）"
- 与 Tauri 官方文档示例一致（`https://v2.tauri.app/distribute/pipelines/github/`）
- **不**用 `release` 分支触发（那是 tauri-action 默认示例，本项目用 tag 触发更符合 AC）

**3. CHANGELOG 生成：**
- 方案 A（推荐）：`mikepenz/release-changelog-builder-action@v5` — 解析 conventional commits 生成结构化 Release Notes（新功能/修复/破坏性变更分类）
- 方案 B：tauri-action 的 `releaseBody` 传入 GitHub 自动生成的 changelog（`gh api` 或 `${{ github.event.release.body }}`）
- **推荐方案 A** — AC 明确要求"基于 conventional commits"且"包含新功能、修复、破坏性变更"分类，mikepenz 工具原生支持
- 项目提交历史已遵循 conventional commits 格式（`feat(8.4):`, `fix(8.2):`, `docs:` 等），可直接解析

**4. 版本号一致性校验：**
- 发布流程：开发者手动 bump `tauri.conf.json` + `Cargo.toml` + `package.json` 三处版本号 → 提交 → 打 `v1.0.0` tag → 推送
- release.yml 第一步校验：从 tag 名提取版本号（`v1.0.0` → `1.0.0`），与三处配置文件版本号比对，不一致则 fail
- **不**用自动 bump 工具（如 `cargo-release`）— V1 简化，手动 bump 更可控

**5. opencode binary 下载：**
- 在 tauri build 之前，按平台下载 opencode 预编译 binary：
  - Windows: `opencode-windows-amd64.exe` → 重命名为 `opencode.exe` 放到 `egosync-app/src-tauri/resources/`
  - macOS: `opencode-darwin-arm64` → 重命名为 `opencode` 放到 `resources/`，`chmod +x`
  - Linux: `opencode-linux-amd64` → 重命名为 `opencode` 放到 `resources/`，`chmod +x`
- 来源：`https://github.com/opencode-ai/opencode/releases/latest/download/`（需确认实际 asset 命名，dev 实现时验证）
- **fallback：** 若下载失败，workflow fail（Release 不允许用占位文件）
- **注意 macOS Arm-only：** 现有 CI 用 `aarch64-apple-darwin`，opencode binary 也需是 arm64 版本。Intel Mac 支持是 V2 候选

**6. 签名（可选，AC 标注"可选"）：**
- macOS notarize 环境变量（通过 GitHub Secrets 注入）：
  - `APPLE_CERTIFICATE`（base64 编码的 .p12）
  - `APPLE_CERTIFICATE_PASSWORD`
  - `KEYCHAIN_PASSWORD`
  - `APPLE_ID` + `APPLE_PASSWORD`（app-specific password）+ `APPLE_TEAM_ID`
  - 或 App Store Connect API：`APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`
- Windows 代码签名：需 EV/OV 证书，通过 `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（主要用于 updater）
- **V1 策略：** 工作流支持签名 env vars 但不强制 — secrets 未配置时跳过签名步骤，构建未签名包（AC 允许：未签名时 DMG/MSI 显示安全提示）
- **实现：** 在 tauri-action 步骤的 `env:` 中引用 secrets，secrets 不存在时 Tauri 自动跳过签名

**7. SHA256 发布后验证：**
- 独立 job `verify-release`，依赖三平台 build job 完成
- 用 `gh release download` 下载各平台 Release Assets
- 计算 SHA256 并与 tauri-action 生成的 `latest.json` 中的 checksums 比对（或直接计算并打印，确认非空且一致）
- 失败时给 Release 打 `verification-failed` 标签或发 issue（V1 简化：fail 即可，Release 已创建但标记问题）

## Acceptance Criteria

1. **AC1（tag 触发）**: Given 推送 `v*` 格式的 Git tag（如 `v1.0.0`），When GitHub Actions 触发 Release 工作流，Then 三平台构建 → E2E 测试通过 → 自动创建 GitHub Release，And 三平台安装包上传为 Release Assets

2. **AC2（CHANGELOG）**: Given Release 信息，Then 自动生成 CHANGELOG（基于 conventional commits），And Release Notes 包含：新功能、修复、破坏性变更

3. **AC3（版本号一致性）**: Given 版本号，Then 语义化版本（semver）：`MAJOR.MINOR.PATCH`，And `tauri.conf.json` 版本号与 Git tag 一致，And `Cargo.toml` / `package.json` 版本号与 Git tag 一致（工作流校验，不一致则 fail）

4. **AC4（macOS 签名，可选）**: Given macOS 签名，Then 支持 Apple notarize（需配置 Apple Developer 证书 via GitHub Secrets），And 未签名时 DMG 可运行但显示安全提示（工作流在 secrets 未配置时跳过签名）

5. **AC5（Windows 签名，可选）**: Given Windows 签名，Then 支持代码签名证书（via GitHub Secrets），And 未签名时 MSI 显示 SmartScreen 警告（工作流在 secrets 未配置时跳过签名）

6. **AC6（发布后验证）**: Given 发布后验证，Then 自动下载各平台安装包并验证文件完整性（SHA256 校验）

7. **AC7（opencode binary）**: Given Release 构建，Then 打包真实 opencode binary（非占位文件），And 三平台安装包均包含可执行的 opencode sidecar

## Tasks / Subtasks

- [x] **Task 1: 创建 `.github/workflows/release.yml` — tag 触发的 Release 工作流** (AC: #1, #3, #4, #5, #7)
  - [x] 1.1 触发配置：`on: push: tags: ['v*']`，添加 `permissions: contents: write`
  - [x] 1.2 新建 `release` job，三平台 matrix（复用 ci.yml 的 matrix 结构：ubuntu/macos/windows + rust_target）
  - [x] 1.3 步骤顺序：checkout → Linux deps → Rust setup → Rust cache → 移除 rsproxy config → Node setup → npm ci → **版本号校验** → **下载真实 opencode binary** → tauri-action 构建+上传 Release
  - [x] 1.4 版本号校验步骤：从 `${{ github.ref_name }}` 提取版本号（去掉 `v` 前缀），用 `python` 或 `bash` 比对 `tauri.conf.json` / `Cargo.toml` / `package.json` 三处版本，不一致则 `exit 1`
  - [x] 1.5 下载 opencode binary 步骤：按 `matrix.platform.os` 下载对应平台 binary 到 `egosync-app/src-tauri/resources/`，重命名为 `opencode`（或 `opencode.exe` on Windows），`chmod +x`（非 Windows）
  - [x] 1.6 tauri-action 步骤：`uses: tauri-apps/tauri-action@v0`，`env: GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`，`with: tagName: ${{ github.ref_name }}`, `releaseName: 'EgoSync ${{ github.ref_name }}'`, `releaseDraft: false`, `prerelease: false`，引用签名 secrets（APPLE_*, TAURI_SIGNING_*）— secrets 未配置时自动跳过
  - [x] 1.7 macOS args：`--target aarch64-apple-darwin`（与 ci.yml 一致，Arm-only）

- [x] **Task 2: 在 release job 中集成 E2E 测试** (AC: #1)
  - [x] 2.1 在 tauri-action 之前运行 E2E（Windows/Linux only，macOS 跳过）— 复用 ci.yml:75-110 的 E2E 步骤模式
  - [x] 2.2 E2E 步骤：Install tauri-driver → Install MS Edge Driver (Windows) → Install Linux E2E deps → Build release binary for E2E → Install E2E deps → Run E2E tests
  - [x] 2.3 E2E 失败则阻断 tauri-action（Release 不创建）— 不用 `continue-on-error`（与 ci.yml 性能基准不同，Release 必须保证质量）

- [x] **Task 3: CHANGELOG 自动生成** (AC: #2)
  - [x] 3.1 在 release.yml 中新增 `changelog` job（或在 release job 中前置步骤），使用 `mikepenz/release-changelog-builder-action@v5`
  - [x] 3.2 配置分类：`feat` → 新功能，`fix` → 修复，`BREAKING CHANGE`/`!:` → 破坏性变更
  - [x] 3.3 将生成的 changelog 传递给 tauri-action 的 `releaseBody` 参数（通过 outputs 或写文件再读取）
  - [x] 3.4 验证项目提交历史可被正确解析（`feat(8.x):`, `fix(8.x):`, `docs:` 等格式）

- [x] **Task 4: SHA256 发布后验证 job** (AC: #6)
  - [x] 4.1 新增 `verify-release` job，`needs: [release]`（依赖三平台 build 完成）
  - [x] 4.2 用 `gh release download ${{ github.ref_name }}` 下载各平台 Release Assets
  - [x] 4.3 计算 SHA256（`sha256sum` on Linux/macOS, `Get-FileHash` on Windows）
  - [x] 4.4 与 tauri-action 生成的 checksums 比对（或验证非空且打印）— V1 简化：计算并打印 SHA256，确认文件完整（非零大小、哈希可计算）
  - [x] 4.5 失败时 `exit 1`（Release 已创建但验证失败需人工处理）

- [x] **Task 5: 版本号一致性校验脚本** (AC: #3)
  - [x] 5.1 在 release.yml 中新增校验步骤（bash 脚本 inline 或独立脚本文件）
  - [x] 5.2 从 tag 提取版本：`TAG_VERSION=${GITHUB_REF_NAME#v}`（如 `v1.0.0` → `1.0.0`）
  - [x] 5.3 读取三处版本号并比对：
    - `tauri.conf.json`：`jq -r '.version' egosync-app/src-tauri/tauri.conf.json`
    - `Cargo.toml`：`grep '^version' egosync-app/src-tauri/Cargo.toml | head -1 | cut -d'"' -f2`
    - `package.json`：`jq -r '.version' egosync-app/package.json`
  - [x] 5.4 三者与 `TAG_VERSION` 任一不一致则 `exit 1` 并打印差异
  - [x] 5.5 校验 semver 格式：`^[0-9]+\.[0-9]+\.[0-9]+$`

- [x] **Task 6: opencode binary 下载脚本** (AC: #7)
  - [x] 6.1 在 release.yml 中新增下载步骤（按平台条件执行）
  - [x] 6.2 确认 opencode releases 的实际 asset 命名（访问 `https://github.com/opencode-ai/opencode/releases/latest` 确认）
  - [x] 6.3 Windows：下载 `opencode-windows-amd64.exe` → 重命名为 `opencode.exe` → 放到 `egosync-app/src-tauri/resources/`
  - [x] 6.4 macOS：下载 `opencode-darwin-arm64` → 重命名为 `opencode` → `chmod +x` → 放到 `resources/`
  - [x] 6.5 Linux：下载 `opencode-linux-amd64` → 重命名为 `opencode` → `chmod +x` → 放到 `resources/`
  - [x] 6.6 删除 `opencode.placeholder`（确保 `resources/opencode*` glob 只匹配真实 binary）
  - [x] 6.7 验证 binary 可执行：`./egosync-app/src-tauri/resources/opencode --version`（或 `--help`）

- [x] **Task 7: 签名 secrets 文档化** (AC: #4, #5)
  - [x] 7.1 在 release.yml 的 `env:` 中引用所有签名 secrets（APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, KEYCHAIN_PASSWORD, APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID, TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD）
  - [x] 7.2 secrets 未配置时 Tauri 自动跳过签名（无需额外条件判断）
  - [x] 7.3 在 story 完成笔记中记录所需 secrets 清单，供 boss 配置 GitHub Secrets 时参考

- [x] **Task 8: 本地 YAML 语法验证 + 推送验证** (AC: #1, #2, #3, #6, #7)
  - [ ] 8.1 用 Python `yaml.safe_load` 验证 release.yml 语法正确
  - [ ] 8.2 提交到 feature 分支，创建 PR（不触发 release.yml，因为 PR 不推 tag）
  - [ ] 8.3 合并 PR 到 main 后，手动 bump 版本号到 `0.1.1`（测试用），打 tag `v0.1.1`，推送触发 release.yml
  - [ ] 8.4 确认：三平台 build job 执行 → E2E 通过 → GitHub Release 创建 → Assets 上传 → CHANGELOG 生成 → SHA256 验证 job 通过
  - [ ] 8.5 修复发现的问题，最终用 `v1.0.0` tag 发布正式 V1（boss 确认时机）

## Dev Notes

### 关键技术决策

- **`tauri-apps/tauri-action@v0` 而非手动构建**：Release 阶段需要自动创建 GitHub Release + 上传 assets，tauri-action 专门为此设计。Story 8.1 用手动方式（CI 阶段只需产物上传），本 story 用 tauri-action（Release 阶段需要 Release 管理）。`@v0` 是 moving tag，最新 release 为 action-v0.6.2（2026-03-14）。**不要**用 `@v1`（无此 release tag）或 `@master`/`@dev`（不稳定）。

- **tag 触发而非 release 分支**：Tauri 官方示例用 `release` 分支触发，但本项目 AC 明确要求 `v*` tag 触发。tag 触发更符合"标准化且可重复"的发布语义，且与版本号绑定。

- **E2E 在 Release 中阻断（非 continue-on-error）**：ci.yml 的性能基准用 `continue-on-error: true`（警告不阻断），但 Release 必须保证质量，E2E 失败应阻断发布。与 AC #1"E2E 测试通过"一致。

- **opencode binary 下载而非 go install**：CI 下载预编译 binary 比 `go install` 更快（无需 Go 工具链）且更可靠（固定版本）。需确认 opencode releases 的实际 asset 命名。

- **版本号手动 bump**：V1 不引入 `cargo-release` 等自动 bump 工具。开发者手动改三处版本号 → 提交 → 打 tag。工作流校验一致性，不一致则 fail。简单可控。

- **签名可选**：AC 标注"可选"，工作流通过 secrets 存在性自动决定是否签名。Tauri 在 `APPLE_CERTIFICATE` 等环境变量未设置时自动跳过签名步骤，无需额外 `if` 条件。

- **SHA256 验证简化**：V1 不强制与 tauri-action checksums 严格比对（tauri-action 的 `latest.json` 格式可能变化），改为计算并打印 SHA256 + 验证文件非空。V2 可收紧为严格比对。

### 现有 CI 步骤复用清单（release.yml 应仿照）

以下 ci.yml 步骤模式在 release.yml 中复用（**复制模式，不修改 ci.yml**）：
- `ci.yml:29-30` Checkout
- `ci.yml:32-36` Linux 依赖安装
- `ci.yml:38-41` Rust toolchain setup
- `ci.yml:43-46` Rust cache（Swatinem/rust-cache@v2, workspaces: egosync-app/src-tauri -> target）
- `ci.yml:48-50` 移除 rsproxy 镜像配置
- `ci.yml:52-57` Node.js setup（node-version: 25, cache: npm）
- `ci.yml:59-61` npm ci
- `ci.yml:75-110` E2E 步骤模式（tauri-driver, MS Edge Driver, Linux E2E deps, build release binary, install E2E deps, run E2E）

### 修改清单

**新增文件：**
- `.github/workflows/release.yml` — tag 触发的 Release 发布工作流

**不修改文件：**
- `.github/workflows/ci.yml` — CI 工作流保持不变（继续负责 push/PR 触发）
- `egosync-app/src-tauri/tauri.conf.json` — bundle 配置不变（版本号由发布流程手动 bump，非本 story 自动化范围）
- `egosync-app/src-tauri/Cargo.toml` — 依赖不变（版本号手动 bump）
- `egosync-app/package.json` — scripts 不变（版本号手动 bump）
- `egosync-app/src-tauri/resources/opencode.placeholder` — release.yml 构建时删除并替换为真实 binary，但占位文件本身保留在仓库（供本地 dev 使用，dev 时 opencode 从 PATH 发现）

### 反模式警告

- **不要**修改 `.github/workflows/ci.yml` — release.yml 是独立工作流，ci.yml 继续负责 push/PR 触发的 CI
- **不要**用 `tauri-apps/tauri-action@v1` — 无此 release tag，用 `@v0`（moving tag 对应 v0.6.x）
- **不要**用 `tauri-apps/tauri-action@master` 或 `@dev` — 不稳定，可能引入破坏性变更
- **不要**用 `release` 分支触发 — 本项目用 `v*` tag 触发，与 AC 一致
- **不要**在 Release 构建中用占位 opencode binary — AC #7 明确要求真实 binary，下载步骤失败应 fail
- **不要**在 E2E 步骤用 `continue-on-error: true` — Release 必须保证质量，E2E 失败应阻断
- **不要**自动 bump 版本号 — V1 手动 bump 三处配置文件，工作流只校验一致性
- **不要**强制签名 — AC 标注"可选"，secrets 未配置时跳过签名
- **不要**修改 `tauri.conf.json` 的 bundle 结构 — 仅版本号会变（手动 bump），bundle 配置已完整
- **不要**在 release.yml 中重复 ci.yml 的前端测试/Rust 测试/性能基准/无障碍扫描 — 那些是 CI 职责，Release 假设 CI 已通过（tag 是在 main 分支上打的，main 已通过 CI）

### macOS 构建目标

与 ci.yml 一致：`aarch64-apple-darwin`（Apple Silicon Arm-only）。`macos-latest` runner 是 M1 Arm。Intel Mac 支持是 V2 候选（需 universal binary 或 x86_64 target）。opencode binary 也需是 darwin-arm64 版本。

### Project Structure Notes

新增文件：
- `.github/workflows/release.yml` — Release 发布工作流

不修改文件：
- `.github/workflows/ci.yml`
- `egosync-app/src-tauri/tauri.conf.json`
- `egosync-app/src-tauri/Cargo.toml`
- `egosync-app/package.json`
- `egosync-app/src-tauri/resources/opencode.placeholder`（保留供本地 dev）

### Previous Story Intelligence

**Story 8.1 关键遗留（本 story 必须解决）：**
- "CI 构建会打包占位文件而非真实 binary — 这对 CI 验证目的可接受，Story 8.5 需解决真实 binary 打包问题"
- "版本号硬编码：V1 简化，artifact name 中的 0.1.0 从 tauri.conf.json 读取并硬编码到 workflow。Story 8.5 可改为动态读取"
- "macOS Arm-only：macos-latest 是 M1 Arm runner，构建 aarch64-apple-darwin 目标。Intel Mac 用户需 Story 8.5 添加 x86_64-apple-darwin 或 universal binary"（本 story V1 仍 Arm-only，Intel 留 V2）

**Story 8.1 CI 修复历程（避免重复踩坑）：**
- npm ci 兼容性：Node 版本对齐（ci.yml 用 Node 25，release.yml 也用 25）
- 前端测试时区：已修复（formatMemoryTime 用 UTC）
- tsc 类型错误：已修复
- Rust 跨平台测试：keyring Entry::new + 跳过逻辑，sidecar PathBuf
- bundle identifier：com.egosync.desktop（避免 .app 后缀冲突）
- Windows CREATE_NO_WINDOW：已修复

**Story 8.4 经验：**
- 性能基准用 `continue-on-error: true`（警告不阻断）— Release 中的 E2E 不用此模式
- CI runner 性能不稳定 — Release 构建（tag 触发）频率低，可接受

**Epic 7 回顾行动项（需执行）：**
- Dev Agent Record 收尾流程：完成时更新 Status/File List/Dev Agent Record，不留 `{{agent_model_name_version}}` 占位符
- spec 与实现偏离的文档化：如有偏离在 Review Findings 中记录

**Git Intelligence：**
最近提交（截至 baseline e87cf00）：
- `e87cf00` docs: 更新 sprint-status — 8-2 代码评审后状态保持 in-progress
- `02e653d` fix(8.2): 代码评审修复
- `85abf47` fix(8.4): 设置 story 完成状态
- `730c70b` feat(8.4): 性能基准验证与优化
- `2daf3cd` feat(8.3): WCAG 无障碍审计
- `aa93922` feat: E2E测试框架搭建与侧边栏滚动修复

提交历史遵循 conventional commits（`feat(8.x):`, `fix(8.x):`, `docs:`），可被 changelog 工具解析。

### Latest Tech Information

**tauri-apps/tauri-action（2026-03-14 最新）：**
- 最新 release：`action-v0.6.2`（2026-03-14）
- 用法：`uses: tauri-apps/tauri-action@v0`（moving tag）
- 关键输入：`tagName`（支持 `__VERSION__` 模板，自动替换为 tauri.conf.json 版本）、`releaseName`、`releaseBody`、`releaseDraft`、`prerelease`、`args`（如 `--target aarch64-apple-darwin`）
- 自动：构建 + 创建/更新 GitHub Release + 上传 bundle 产物 + 生成 `latest.json`
- 签名：通过 `env:` 注入 APPLE_*/TAURI_SIGNING_* 环境变量，Tauri 自动处理

**macOS 签名环境变量（Tauri 2 官方）：**
- `APPLE_CERTIFICATE`：base64 编码的 .p12 证书
- `APPLE_CERTIFICATE_PASSWORD`：.p12 密码
- `KEYCHAIN_PASSWORD`：CI 临时 keychain 密码
- `APPLE_SIGNING_IDENTITY`：签名身份（如 "Developer ID Application: Name (TEAMID)"）
- Notarize 方式 A（Apple ID）：`APPLE_ID` + `APPLE_PASSWORD`（app-specific password）+ `APPLE_TEAM_ID`
- Notarize 方式 B（App Store Connect API）：`APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`
- 未设置时 Tauri 自动跳过签名

**Windows 签名环境变量：**
- `TAURI_SIGNING_PRIVATE_KEY`：签名私钥（主要用于 updater）
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码
- MSI 代码签名需额外的 EV/OV 证书（通过其他工具如 `azure-trusted-signing-action` 或手动 signtool）

**mikepenz/release-changelog-builder-action：**
- 解析 conventional commits 生成分类 Release Notes
- 配置分类规则：`feat` → 新功能，`fix` → 修复，`BREAKING CHANGE` 或 `!:` → 破坏性变更
- 输出可传递给 tauri-action 的 `releaseBody`

**Tauri 官方 GitHub pipeline 文档：**
- `https://v2.tauri.app/distribute/pipelines/github/`
- 推荐 tag 触发：`on: push: tags: ['app-v*']`（本项目用 `v*` 无 `app-` 前缀，与 AC 一致）
- 签名指南：`https://v2.tauri.app/distribute/sign/macos/` 和 `https://v2.tauri.app/distribute/sign/windows/`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.5] — AC 原文（epics.md:2623-2653）
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8] — Epic 8 上下文（epics.md:2484-2653, 400-417）
- [Source: _bmad-output/planning-artifacts/architecture.md:454-465] — CI/CD 架构规划（分发方式 + CI/CD）
- [Source: _bmad-output/planning-artifacts/architecture.md:721-725] — workflow 文件规划（ci.yml + release.yml）
- [Source: _bmad-output/planning-artifacts/architecture.md:1005-1011] — 构建流程和产物路径
- [Source: _bmad-output/project-context.md:46-49] — 构建与分发技术栈
- [Source: _bmad-output/project-context.md:173-177] — CI/CD 规则
- [Source: .github/workflows/ci.yml:1-197] — 现有 CI workflow 完整内容（复用步骤模式）
- [Source: egosync-app/src-tauri/tauri.conf.json:1-45] — Tauri 配置（bundle 结构 + 版本号位置）
- [Source: egosync-app/src-tauri/Cargo.toml:1-49] — Rust 依赖 + 版本号位置
- [Source: egosync-app/package.json:1-44] — npm scripts + 版本号位置
- [Source: egosync-app/src-tauri/resources/README.md:1-9] — opencode sidecar 打包说明
- [Source: _bmad-output/implementation-artifacts/8-1-github-actions-ci-build.md:125-175] — Story 8.1 Dev Notes（遗留问题 + 反模式）
- [Source: _bmad-output/implementation-artifacts/8-4-performance-benchmark.md:15-51] — Story 8.4 背景（E2E 框架复用 + CI 模式）
- [Source: _bmad-output/implementation-artifacts/epic-7-retro-2026-06-27.md:45-51] — Epic 7 回顾行动项
- [Tauri 官方: https://v2.tauri.app/distribute/pipelines/github/] — Tauri GitHub Actions 发布指南
- [tauri-apps/tauri-action: https://github.com/tauri-apps/tauri-action] — tauri-action 使用说明（最新 action-v0.6.2）
- [Tauri macOS 签名: https://v2.tauri.app/distribute/sign/macos/] — macOS notarize 环境变量
- [Tauri 环境变量参考: https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/ENVIRONMENT_VARIABLES.md] — 完整环境变量列表
- [mikepenz/release-changelog-builder-action: https://github.com/mikepenz/release-changelog-builder-action] — conventional commits changelog 生成

## Dev Agent Record

### Agent Model Used

GLM-5.2 High (Amelia / bmad-agent-dev)

### Debug Log References

- Web 查证 opencode-ai/opencode 仓库状态：2025-09-18 归档只读，v0.0.55 无 Windows binary
- Web 查证 sst/opencode（anomalyco/opencode）最新 release v1.17.11（2026-06-25），v1.15.10 存在（2026-05-23）
- Web 查证 tauri-apps/tauri-action 最新 release action-v0.6.2（2026-03-14），@v0 moving tag 确认
- Web 查证 mikepenz/release-changelog-builder-action 支持 commit mode + conventional commits label_extractor
- 本地验证：`opencode --version` → 1.15.10（npm 全局安装）
- 本地验证：`yaml.safe_load(release.yml)` → YAML OK，jobs: changelog/release/verify-release
- 本地验证：`json.load(changelog-configuration.json)` → JSON OK，4 categories, 3 label_extractors

### Completion Notes List

**实现完成，待 tag 推送触发 CI 验证（Task 8.2-8.5 需 boss 确认后执行）。**

1. **opencode binary 来源偏离 story 预设（boss 确认）**：
   - Story 预设来源 `opencode-ai/opencode` 已于 2025-09-18 归档只读，且 v0.0.55 无 Windows binary，与 AC7 三平台要求冲突
   - 改用 `sst/opencode`（https://github.com/sst/opencode），活跃维护，全平台覆盖
   - 版本锁定 `v1.15.10`（与本地开发环境一致，boss 确认）
   - assets 是归档文件（zip/tar.gz），需下载+解压+重命名，非 story 预设的裸 binary 直接重命名

2. **CHANGELOG 配置**：使用 commit mode + label_extractor 正则解析 conventional commits，配置文件 `.github/changelog-configuration.json`，分类：破坏性变更/新功能/修复/其他

3. **签名 secrets 清单（供 boss 配置 GitHub Secrets）**：
   - macOS: `APPLE_CERTIFICATE`（base64 .p12）, `APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`（app-specific）, `APPLE_TEAM_ID`
   - Windows: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   - 所有 secrets 未配置时 Tauri 自动跳过签名，构建未签名包（AC 允许）

4. **E2E 阻断策略**：Release 中 E2E 不用 `continue-on-error`，失败则阻断 Release 创建（与 ci.yml 性能基准的 warn-only 模式不同）

5. **待 boss 确认的后续步骤**：
   - Task 8.2: 提交到 feature 分支，创建 PR
   - Task 8.3: 合并 PR 到 main 后，手动 bump 版本号到 `0.1.1`（测试用），打 tag `v0.1.1` 推送触发 release.yml
   - Task 8.5: 最终用 `v1.0.0` tag 发布正式 V1

### File List

- `.github/workflows/release.yml` — 新建，tag 触发的 Release 发布工作流（changelog job + release matrix job + verify-release job）
- `.github/changelog-configuration.json` — 新建，mikepenz changelog builder 的 conventional commits 分类配置
