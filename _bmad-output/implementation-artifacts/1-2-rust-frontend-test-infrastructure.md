# Story 1.2: 建立 Rust + 前端最小测试基础设施

Status: done

## Story

As a 开发者,
I want 项目从第一天就有可运行的测试链路（Rust 单元/集成 + 前端 Vitest + CI workflow 骨架）,
So that 后续每个 Story 都能在提交时验证不破坏已有功能。

## Acceptance Criteria

1. **AC-1: 前端测试可运行**
   - Given 开发者在 `GUI/` 目录下
   - When 执行 `npm run test:frontend`
   - Then Vitest 运行并报告 ≥ 1 个测试通过，退出码 0

2. **AC-2: Rust 测试可运行**
   - Given 开发者在 `GUI/src-tauri/` 目录下
   - When 执行 `cargo test`
   - Then Rust 测试运行并报告 ≥ 1 个测试通过，退出码 0

3. **AC-3: 全量测试一键运行**
   - Given 开发者在 `GUI/` 目录下
   - When 执行 `npm run test:all`
   - Then 按顺序运行前端测试 + Rust 测试，全部通过

4. **AC-4: CI 三平台验证**
   - Given 推送到 GitHub 任意分支
   - When CI workflow 触发
   - Then 三平台 matrix（ubuntu-latest/macos-latest/windows-latest）均执行 `cargo test` + `npm run test:frontend` + `npm run tauri build`
   - And 任一步骤失败则整个 workflow 失败

5. **AC-5: Rust 测试自动发现**
   - Given 后续 Story 添加新 Rust 测试文件（如 `src-tauri/tests/test_llm.rs`）
   - When 执行 `cargo test`
   - Then 自动发现并运行，无需修改配置

## Tasks / Subtasks

- [x] Task 1: 安装前端测试依赖 (AC: #1)
  - [x] 1.1 安装 Vitest + jsdom + @testing-library/react + @testing-library/jest-dom
  - [x] 1.2 创建 `vitest.config.ts`（environment: jsdom, globals: true, include 规则）
  - [x] 1.3 在 `package.json` 添加 `"test:frontend": "vitest run"` 脚本
  - [x] 1.4 创建 `src/test-setup.ts`（import @testing-library/jest-dom）
  - [x] 1.5 更新 `tsconfig.json` 添加 vitest 类型引用

- [x] Task 2: 编写前端冒烟测试 (AC: #1)
  - [x] 2.1 创建 `src/App.test.tsx` — 验证 App 组件可渲染无报错
  - [x] 2.2 运行 `npm run test:frontend` 确认通过（1 test passed, 4.98s）

- [x] Task 3: 编写 Rust 单元测试 (AC: #2, #5)
  - [x] 3.1 在 `src-tauri/src/lib.rs` 底部添加 `#[cfg(test)] mod tests` — 最小断言测试
  - [x] 3.2 运行 `cargo test` 确认通过（1 passed, 1m11s 含编译）

- [x] Task 4: 创建 Rust 集成测试骨架 (AC: #2, #5)
  - [x] 4.1 创建 `src-tauri/tests/common/mod.rs` — 集成测试公共模块（空占位）
  - [x] 4.2 创建 `src-tauri/tests/test_app.rs` — 应用级集成测试占位
  - [x] 4.3 运行 `cargo test` 确认单元 + 集成测试均通过（2 passed, 8.59s 增量）

- [x] Task 5: 添加全量测试脚本 (AC: #3)
  - [x] 5.1 在 `package.json` 添加 `"test:all"` 脚本（顺序执行前端 + Rust 测试）
  - [x] 5.2 运行 `npm run test:all` 确认退出码 0（vitest 1 passed + cargo 2 passed）

- [x] Task 6: 创建 GitHub Actions CI workflow (AC: #4)
  - [x] 6.1 创建 `.github/workflows/ci.yml` — 三平台 matrix 构建 + 测试
  - [x] 6.2 配置 job：checkout → setup rust → setup node → npm ci → cargo test → vitest → tauri build
  - [x] 6.3 确认 workflow 文件语法正确（YAML 结构验证通过）

## Dev Notes

### 当前项目状态（Story 1.1 完成后）

- `GUI/src-tauri/` 已存在：Cargo.toml、lib.rs、main.rs、tauri.conf.json、build.rs
- lib.rs 包含 `pub fn run()` + tracing-subscriber 初始化 + Tauri builder
- main.rs 仅调用 `egosync_lib::run()`
- package name = `egosync`, lib name = `egosync_lib`
- **无任何测试依赖**：package.json 无 vitest，Cargo.toml 无 dev-dependencies
- **无 `.github/` 目录**
- `.cargo/config.toml` 配有 rsproxy 镜像 — CI 不应依赖此镜像，使用默认 crates.io

### 前端测试配置精确规范

**依赖安装（devDependencies）：**
```bash
npm install -D vitest jsdom @testing-library/react @testing-library/jest-dom
```

**vitest.config.ts（项目根 `GUI/` 下）：**
```typescript
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
})
```

**注意**：vitest.config.ts 独立于 vite.config.ts。vite.config.ts 包含 Tauri 特定配置（clearScreen、envPrefix、build target 等），测试不需要这些。分开避免污染。

**src/test-setup.ts：**
```typescript
import '@testing-library/jest-dom'
```

**tsconfig.json 更新** — 在 `compilerOptions` 中添加 types：
```json
{
  "compilerOptions": {
    "types": ["vitest/globals", "@testing-library/jest-dom"]
  }
}
```

⚠️ 注意：`"types"` 字段会限制全局类型可见性。只添加 `vitest/globals` 和 `@testing-library/jest-dom`，其余类型通过 import 获得。

### package.json scripts 精确更新

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test:frontend": "vitest run",
    "test:all": "vitest run && cd src-tauri && cargo test"
  }
}
```

`test:all` 在 Windows 上使用 `&&` 链接命令（PowerShell/cmd 均支持）。

### Rust 测试配置

**单元测试 — lib.rs 底部添加：**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn app_compiles() {
        // 验证 crate 可编译，依赖无冲突
        assert!(true);
    }
}
```

**集成测试 — tests/test_app.rs：**
```rust
//! 应用级集成测试骨架
//! 后续 Story 在此目录添加 test_chat.rs, test_roles.rs 等

mod common;

#[test]
fn integration_test_placeholder() {
    // 验证集成测试目录结构正确，cargo test 自动发现
    assert_eq!(1 + 1, 2);
}
```

**集成测试公共模块 — tests/common/mod.rs：**
```rust
//! 集成测试公共模块
//! 后续可放置 test fixtures, DB setup helpers 等

#[allow(dead_code)]
pub fn setup() {
    // 将来放置测试 setup 逻辑
}
```

**说明：** Rust 集成测试无需额外 Cargo.toml 配置。`src-tauri/tests/` 目录下的 `.rs` 文件会被 `cargo test` 自动发现（满足 AC-5）。

### GitHub Actions CI 精确配置

**文件路径：** `探索/.github/workflows/ci.yml`（项目根目录，非 GUI/ 下）

**关键注意事项：**
- 三平台：ubuntu-latest, macos-latest, windows-latest
- Linux 需安装 Tauri 系统依赖：`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
- macOS / Windows 无需额外系统依赖（WebView2 内置于 Windows runner）
- Rust cache 加速编译（actions-rs/toolchain 或 dtolnay/rust-toolchain + Swatinem/rust-cache）
- Node cache 加速 npm install
- `cargo test` 在 `GUI/src-tauri` 目录执行
- CI 不使用 rsproxy 镜像（.cargo/config.toml 仅在本地生效，CI runner 使用 crates.io）
- `npm run tauri build` 产出验证：workflow 成功即可，不上传 artifact（后续 release.yml 处理）

**CI 不需要覆盖 `.cargo/config.toml`：** 该文件在 `GUI/src-tauri/.cargo/` 下，CI runner 可直接访问 crates.io。如果 rsproxy 不可达但未配置 fallback，CI 可能失败。

⚠️ **风险：** `GUI/src-tauri/.cargo/config.toml` 硬编码 rsproxy。CI runner 在海外，rsproxy 可能反而降速。有两个选项：
1. 在 CI 中覆盖/删除该 config（推荐）
2. 在 config.toml 中添加 fallback（Story 1.1 deferred）

**推荐方案：** CI workflow 中添加步骤移除 `.cargo/config.toml`，或设置 `CARGO_NET_GIT_FETCH_WITH_CLI=true` 环境变量绕过。最简方案：CI step 中 `rm -f GUI/src-tauri/.cargo/config.toml`（仅 CI 运行时移除）。

### 前端冒烟测试示例

**src/App.test.tsx：**
```tsx
import { render, screen } from '@testing-library/react'
import App from './App'

describe('App', () => {
  it('renders without crashing', () => {
    render(<App />)
    // App.tsx 包含 "EgoSync" 文本（侧边栏标题）
    expect(document.body).toBeTruthy()
  })
})
```

注意：`App.tsx` 是 1458 行单文件，render 可能因 Lucide icons 或 mock 数据报错。如果 render 失败，降级为更简单的测试（如测试 utility 函数 `cn()`）。**测试目标是验证基础设施可用，非覆盖 App 功能。**

备选冒烟测试（如 App 渲染失败）：
```tsx
import { describe, it, expect } from 'vitest'

describe('Test infrastructure', () => {
  it('vitest runs correctly', () => {
    expect(1 + 1).toBe(2)
  })
})
```

### 不在本 Story 范围

- ❌ 为业务逻辑编写完整测试（后续 Story 逐步添加）
- ❌ 配置 E2E 测试（Tauri driver，属于 Epic 8）
- ❌ 代码覆盖率门槛（V1 不设强制门槛）
- ❌ 修改 App.tsx 或 lib.rs 的业务逻辑
- ❌ 添加 SQLx/keyring 等业务依赖
- ❌ release.yml（分发 workflow，属于 Epic 8）

### 从 Story 1.1 继承的学习

- Cargo 首次编译慢（~3.5 分钟），CI 需缓存
- `egosync_lib` 是 crate name，`egosync` 是 package name
- lib.rs 使用 `try_init()` 避免多次初始化 panic — 测试环境中可能多次调用
- Tauri 2.x 使用 capabilities/default.json 权限系统
- `"type": "module"` 在 package.json 中 — vitest 原生支持 ESM

### Project Structure Notes

本 Story 完成后新增/修改文件预期：

```
探索/
├── .github/
│   └── workflows/
│       └── ci.yml                    # 新增：CI 三平台构建+测试
└── GUI/
    ├── package.json                  # 修改：+test scripts, +devDeps
    ├── package-lock.json             # 修改：依赖锁定更新
    ├── vitest.config.ts              # 新增：Vitest 配置
    ├── tsconfig.json                 # 修改：+types
    ├── src/
    │   ├── test-setup.ts             # 新增：测试环境设置
    │   └── App.test.tsx              # 新增：前端冒烟测试
    └── src-tauri/
        ├── src/
        │   └── lib.rs                # 修改：+#[cfg(test)] mod tests
        └── tests/
            ├── common/
            │   └── mod.rs            # 新增：集成测试公共模块
            └── test_app.rs           # 新增：集成测试骨架
```

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Test Execution] — 测试命令定义
- [Source: _bmad-output/planning-artifacts/architecture.md#Testing Framework] — Vitest + RTL + cargo test
- [Source: _bmad-output/planning-artifacts/architecture.md#Test Location] — 测试文件位置规范
- [Source: _bmad-output/planning-artifacts/architecture.md#Infrastructure & Deployment] — CI/CD 定义
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.2] — AC 定义
- [Source: _bmad-output/implementation-artifacts/1-1-tauri-desktop-app-existing-ui.md] — 前序 Story 学习
- [Source: _bmad-output/project-context.md#测试规则] — 测试标准

### Review Findings

- [x] [Review][Patch] CI release build 过重 — 已修复：`npm run tauri build` → `cargo build`（debug 编译，release build 留给 release.yml）[`.github/workflows/ci.yml:71-73`]
- [x] [Review][Defer] `tsconfig.json` types 字段限制全局类型可见性 — 后续添加 `@types/node` 等需手动加入 types 数组 [`GUI/tsconfig.json:18`] — deferred, 当前不影响
- [x] [Review][Defer] CI macOS target 硬编码 `aarch64-apple-darwin` — 若 GitHub Actions runner 架构变更需更新 [`.github/workflows/ci.yml:22`] — deferred, 当前正确
- [x] [Review][Defer] CI 移除整个 `.cargo/config.toml` — 若未来该文件包含非镜像配置会丢失 [`.github/workflows/ci.yml:50`] — deferred, 当前只含 rsproxy

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (Cascade)

### Debug Log References

- vitest.config.ts 独立于 vite.config.ts，避免 Tauri 特定配置污染测试环境
- App.tsx 1458 行可成功 render 在 jsdom 中，未需降级到纯断言测试
- CI 中移除 .cargo/config.toml（rsproxy 镜像）以使用默认 crates.io
- Rust 增量编译后 cargo test 秒级完成（首次 1m11s）

### Completion Notes List

- ✅ AC-1: `npm run test:frontend` — Vitest 运行 1 test passed，退出码 0
- ✅ AC-2: `cargo test` — 单元测试 1 passed + 集成测试 1 passed，退出码 0
- ✅ AC-3: `npm run test:all` — 前端 + Rust 全部通过，退出码 0
- ✅ AC-4: CI workflow 创建（`.github/workflows/ci.yml`），三平台 matrix + Rust cache + 移除 rsproxy
- ✅ AC-5: 集成测试 `tests/test_app.rs` 被 `cargo test` 自动发现，无需配置

### Change Log

- 2026-05-21: Story 1.2 实现完成 — Rust + 前端测试基础设施 + CI workflow

### File List

**新增文件：**
- `GUI/vitest.config.ts` — Vitest 测试配置（jsdom, globals, setupFiles）
- `GUI/src/test-setup.ts` — 测试环境设置（@testing-library/jest-dom）
- `GUI/src/App.test.tsx` — 前端冒烟测试（App 渲染验证）
- `GUI/src-tauri/tests/common/mod.rs` — Rust 集成测试公共模块
- `GUI/src-tauri/tests/test_app.rs` — Rust 集成测试骨架
- `.github/workflows/ci.yml` — GitHub Actions CI 三平台构建+测试

**修改文件：**
- `GUI/package.json` — +test:frontend, +test:all scripts, +vitest/jsdom/@testing-library devDeps
- `GUI/package-lock.json` — 依赖锁定更新
- `GUI/tsconfig.json` — +types: vitest/globals, @testing-library/jest-dom
- `GUI/src-tauri/src/lib.rs` — +#[cfg(test)] mod tests 单元测试
