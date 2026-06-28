# EgoSync E2E 测试

## 概述

使用 WebdriverIO + tauri-driver 对 EgoSync 桌面应用进行端到端测试，覆盖 7 条核心用户旅程。

## 前置条件

### 1. 构建 Debug 二进制

```bash
cd egosync-app
npm run tauri build -- --debug --no-bundle
```

生成的二进制路径：
- Windows: `src-tauri/target/debug/egosync.exe`
- Linux: `src-tauri/target/debug/egosync`

### 2. 安装 tauri-driver

```bash
cargo install tauri-driver --locked
```

### 3. 平台特定依赖

**Linux:**
```bash
sudo apt-get install -y webkit2gtk-driver xvfb
```

**Windows:**
需要安装 Microsoft Edge Driver（tauri-driver 依赖）。

**macOS:**
不支持 tauri-driver（WKWebView 无 WebDriver 工具）。

### 4. 安装 E2E 依赖

```bash
cd egosync-app/tests/e2e
npm install
```

## 运行测试

### 本地运行

**Linux（需要 xvfb）：**
```bash
cd egosync-app/tests/e2e
xvfb-run --auto-servernum npm test
```

**Windows：**
```bash
cd egosync-app/tests/e2e
npm test
```

### CI 运行

E2E 测试在 GitHub Actions CI 中自动运行（仅 Windows + Linux，macOS 跳过）。
在 `tauri build` 步骤之后执行，失败时截图保存到 CI 产物。

## 测试旅程

| # | 文件 | 旅程 |
|---|------|------|
| 1 | `cold-start-onboarding.spec.ts` | 冷启动引导 |
| 2 | `butler-conversation.spec.ts` | 管家对话 |
| 3 | `role-crud.spec.ts` | 角色 CRUD |
| 4 | `llm-streaming.spec.ts` | LLM 流式响应 |
| 5 | `task-management.spec.ts` | 任务管理 |
| 6 | `conflict-arbitration.spec.ts` | 冲突仲裁 |
| 7 | `briefing-review.spec.ts` | 简报复盘 |

## LLM 依赖说明

3 条旅程依赖 LLM（冷启动引导、管家对话、LLM 流式响应）。
CI 环境无 API Key，opencode sidecar 占位文件无法提供真实 LLM 响应。

**策略：验证 UI 行为，不验证 LLM 响应内容。**
- 预置对话消息到 DB，验证 UI 正确渲染历史消息
- 验证用户输入 → 消息发送 → UI 显示用户消息
- 流式响应：验证 UI 状态（输入框、发送/停止按钮），不验证 token 内容

## 数据隔离

每次测试运行前通过 `beforeSession` 钩子清理 `app_data_dir` 下的 SQLite 数据库文件，确保测试隔离。

- Windows: `%APPDATA%/com.egosync.desktop/`
- Linux: `~/.config/com.egosync.desktop/`

## 失败诊断

测试失败时自动截图保存到 `screenshots/` 目录。
CI 中通过 `actions/upload-artifact` 上传截图供后续分析。
