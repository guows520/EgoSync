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

## Web 自托管服务（server/）

`server/` 是云端自托管版二进制（axum）：同一前端构建产物（`egosync-app/dist`）双宿主复用——桌面走 Tauri，浏览器直接由 server 伺服（含 SPA 回退），认证/业务/事件面与桌面同构（详见 `_bmad-output/planning-artifacts/architecture.md` 云端托管章节）。

### Docker 一键部署（生产推荐）

`server/` 目录提供完整部署物（多阶段 Dockerfile + docker-compose + Caddy 自动 HTTPS）：

```bash
cd server
# 创建 .env（至少设 EGOSYNC_DOMAIN；详见部署指南 11.2）
echo "EGOSYNC_DOMAIN=ego.example.com" > .env
echo "EGOSYNC_TOKEN=$(openssl rand -base64 24)" >> .env   # 推荐：预设访问令牌
docker compose up -d --build
```

两服务拓扑：`egosync-server`（引擎 + opencode + 双 SQLite 单容器，数据挂命名卷 /data，WAL 模式）+ `caddy`（TLS 终结与代理，自动签发证书；http 明文 308 重定向 https）。升级 = 拉新代码重跑 `docker compose up -d --build`；宿主机重启自动恢复（`restart: unless-stopped`）。

**从零到浏览器的完整步骤**（域名解析、密钥注入、时区、备份、自有反代要求、1C1G 资源预算）见 **[docs/user-guide/11-云端自托管部署.md](docs/user-guide/11-云端自托管部署.md)**。

本地运行（开发验证用）：

```bash
cd egosync-app && npm run build   # 先产出 dist（server 伺服的就是它）
cd ../server && cargo run         # http://localhost:8080
```

### 环境变量全表

| 变量 | 语义 | 默认 |
| --- | --- | --- |
| `EGOSYNC_DATA_DIR` | 数据目录（egosync.db / conversations.db / secrets.json 落点） | 必填（compose 形态 `/data`） |
| `EGOSYNC_TOKEN` | 预置访问令牌（存在则首访初始化向导关闭、登录按 env 常时比对） | 未设（走首访向导） |
| `EGOSYNC_STATIC_DIR` | 静态目录（默认回退 `../egosync-app/dist`） | `/app/static`（镜像内） |
| `EGOSYNC_HOST` / `EGOSYNC_PORT` | 监听地址 / 端口 | `127.0.0.1` / `8080`（安全默认；compose 形态 `0.0.0.0`） |
| `EGOSYNC_BEHIND_PROXY` | 反代感知门控（`1`/`true`）：同源判定读 `X-Forwarded-Proto`（TLS 反代缺省端口归一），且反代 TLS 面 Cookie 加 `Secure`；未设则 X-Forwarded-\* 一律忽略（直连语义） | 未设 |
| `EGOSYNC_OPENCODE_PATH` | opencode 二进制路径或其所在目录（文件取父目录；缺省按系统 PATH 解析） | 未设（PATH fallback） |
| `EGOSYNC_SECRET_{api_key_ref}` | LLM Key env 引导通道（键名原样区分大小写；文件优先，文件有值时 env 不生效） | 未设 |
| `TZ` / `RUST_LOG` | 容器时区（调度判定基准）/ 日志级别（stdout JSON 结构化输出） | 系统 / `info` |

部署要点：

- **静态目录缺失 = API-only 警告运行**：`EGOSYNC_STATIC_DIR` 与默认路径皆无产物时 server 照常提供 API，日志告警——浏览器访问将得到 404；先 `npm run build` 再启动。
- **重部署无需清缓存**：index.html 响应统一 `Cache-Control: no-cache`，带 hash 的资产可被浏览器安全长缓存。
- **首访流程**：无凭据实例打开页面即进「初始化」向导（设置 ≥8 位令牌）；登录会话为 30 天持久 Cookie（关浏览器重开免重登），侧栏提供登出入口。
- **生产部署**：进程只讲 HTTP——置于 TLS 反代（Caddy/自有反代，反代要求见部署指南 11.4）之后；认证端点限流 5 次/分钟/IP（反代拓扑下所有外部请求来自代理容器，限流实际按实例计——XFF 感知限流见 deferred-work）。
- **密钥安全**：LLM Key 只存于服务端内存与 `/data/secrets.json`（0600）；密钥缺失返回指明重录路径的结构化错误（设置 → 模型服务配置 → 重新保存密钥）。

## 手机伴侣连接

桌面应用支持与手机伴侣 App（`companion-android/`）建立加密连接（局域网直连优先，可选中继）：

- **配对方式**：在桌面「全局设置 → 手机伴侣」中生成配对二维码，手机扫码完成绑定。二维码单次有效（5 分钟内使用），重新生成会使旧码立即失效。局域网内扫码即绑定；配置了中继服务器时，手机不在同一局域网也可扫码经中继完成首次配对（需在桌面确认后生效，防二维码泄露被冒用）。
- **服务发现**：桌面通过 mDNS/NSD 广播服务 `_egosync._tcp`（实例名 `EgoSync-<relay_id 前 8 位>`）——配对窗口打开期间（等待扫码时）与配对过至少一台设备后均持续广播，手机在同一局域网内可自动发现桌面，免扫码重连；局域网发现失败且二维码携带中继地址时，手机自动回退经中继连接。
- **中继服务器（可选）**：在「全局设置 → 手机伴侣」配置自建中继（见 `relay-server/`，Docker 一键部署）后，手机跨网络经中继加密转发连接；中继只转发密文，无法解读内容。未配置时手机需与电脑处于同一局域网。
- **端口策略**：WebSocket 监听端口由系统动态分配（`0.0.0.0:0`），实际端口随 NSD 广播自动携带，无需手动配置。
- **Windows 防火墙**：首次使用时系统可能弹出「允许 EgoSync 访问网络」提示，请勾选「专用网络」并允许，否则手机无法发现电脑。
- **连接安全**：采用 Noise XX（`Noise_XX_25519_ChaChaPoly_BLAKE2s`）双向身份认证握手，桌面静态密钥存储于系统钥匙串（keyring），已配对设备凭静态公钥免配对直连；移除配对后该设备立即失去连接资格。
- **数据导入**：配对绑定不随数据导入恢复（配对密钥保存在本机系统钥匙串，不可跨设备迁移）；恢复数据后需在桌面「全局设置 → 手机伴侣」重新扫码配对。

构建产物位于 Tauri target 目录。仓库中的 GitHub Actions 提供跨平台 CI 与基于版本标签的发布流程。