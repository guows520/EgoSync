<p align="center">
  <img src="./assets/egosync-logo.png" alt="EgoSync Logo" width="160" />
</p>

<h1 align="center">EgoSync</h1>

<p align="center">
  EgoSync 是一套以人生角色为核心、由数字分身管家统一协调的个人多智能体系统。
</p>


## 项目简介

EgoSync 将用户在生活中的不同身份外化为可协作的 AI 角色，例如产品经理、父母、学习者或管理者。每个角色可以维护独立的目标、职责、任务、记忆与技能；数字管家则负责跨角色协调、识别冲突并帮助用户聚焦真正重要的事情。

项目采用本地优先架构，桌面端基于 Tauri 构建，业务数据默认保存在本地 SQLite 数据库中。

## 核心能力

- **多角色管理**：创建和管理具有独立目标、职责、记忆与技能的个人角色。
- **数字管家**：统一协调各个角色，通过对话提供建议、提醒和行动支持。
- **任务管理**：支持任务增删改查、拖拽排序、完成状态与四象限分类。
- **主动协助**：提供主动建议、分级通知、重要任务保护与截止时间提醒。
- **复盘与规划**：支持晨间简报、每周复盘、重要事项规划和状态概览。
- **AI 能力配置**：支持模型配置、Skill 管理、MCP Server 管理与 OpenCode sidecar。
- **本地数据控制**：提供本地存储、数据导入导出和数据清除能力。
- **桌面体验**：支持浅色/深色主题，并面向 Windows、macOS 和 Linux 构建。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 前端 | React 18、TypeScript、Vite 6 |
| 样式 | Tailwind CSS |
| 后端 | Rust、Tokio |
| 数据库 | SQLite、SQLx |
| 前端测试 | Vitest、Testing Library |
| 端到端测试 | WebdriverIO、Axe、Pa11y |
| 持续集成 | GitHub Actions |

## 项目结构

```text
.
├── egosync-app/                   # 桌面应用主目录
│   ├── src/                       # React 前端源码
│   │   ├── components/            # 页面与通用组件
│   │   ├── hooks/                 # React Hooks
│   │   ├── services/              # 前端服务与 Tauri 调用封装
│   │   └── types/                 # TypeScript 类型
│   ├── src-tauri/                 # Tauri/Rust 后端
│   │   ├── migrations/            # SQLite 数据库迁移
│   │   ├── resources/             # sidecar 等打包资源
│   │   └── src/                   # 命令、模型、仓储与业务服务
│   └── tests/e2e/                 # 端到端、性能与无障碍测试
├── _bmad-output/                  # 产品、架构、故事与实施产物
├── docs/                          # 项目补充文档
├── UAT测试/                       # 用户验收测试资料
└── .github/workflows/             # CI 与发布流程
```

## 环境要求

开始前请准备：

- Node.js 25（与当前 CI 环境一致）
- npm
- Rust stable（项目声明的最低 Rust 版本为 `1.77.2`）
- 当前操作系统所需的 Tauri 2 原生构建依赖
- OpenCode（仅在开发 AI/Agent 执行能力时需要，命令需可从系统 `PATH` 发现）

## 本地开发

```bash
git clone https://github.com/guows520/EgoSync.git
cd EgoSync/egosync-app
npm install
npm run tauri dev
```

`npm run tauri dev` 会同时启动 Vite 开发服务器和 Tauri 桌面窗口。

如只需调试前端页面：

```bash
cd egosync-app
npm run dev
```

默认开发地址为 `http://localhost:5173`。部分依赖 Tauri 命令的能力无法在普通浏览器环境中完整运行。

### OpenCode sidecar

开发模式下，应用会从系统 `PATH` 查找 `opencode`。如需制作包含 sidecar 的发行包，请将对应平台的可执行文件放入：

```text
egosync-app/src-tauri/resources/
```

文件名约定：

- Windows：`opencode.exe`
- macOS / Linux：`opencode`

## 常用命令

以下命令均在 `egosync-app` 目录中执行：

| 命令 | 用途 |
| --- | --- |
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run tauri dev` | 启动完整桌面开发环境 |
| `npm run build` | 执行 TypeScript 检查并构建前端 |
| `npm run tauri build` | 构建当前平台的桌面安装包 |
| `npm run test:frontend` | 运行前端单元测试 |
| `npm run test:all` | 运行前端测试与 Rust 测试 |
| `npm run preview` | 预览前端生产构建 |

## 端到端测试

端到端测试拥有独立的依赖配置：

```bash
cd egosync-app/tests/e2e
npm install
npm test
```

其他可用命令包括：

```bash
npm run test:ci
npm run test:perf
npm run pa11y
npm run audit:animations
```

## 构建与发布

在本地构建当前平台安装包：

```bash
cd egosync-app
npm install
npm run tauri build
```

构建产物位于 Tauri target 目录。仓库中的 GitHub Actions 提供跨平台 CI 与基于版本标签的发布流程。