除非显式覆盖，否则本规则适用于本项目中的所有任务。
核心倾向：非琐碎工作，谨慎优先于速度；琐碎任务可自主判断处理。

## 项目地图（先读这里）

- EgoSync：本地优先的个人多智能体桌面应用。桌面端 Tauri 2 + React 18 + TypeScript + Vite + Tailwind；后端 Rust/Tokio + SQLite(SQLx)。
- `egosync-app/`：`src/` 前端、`src-tauri/` Rust 后端与 SQLite 迁移、`tests/e2e/` 端到端测试（依赖独立安装）。
- `companion-android/`：手机伴侣高保真原型（纯前端 + mock，桌面端是唯一事实源），局部规范见其 README.md。
- `_bmad-output/project-context.md`：技术栈细节、关键实现规则、使用指南——项目细节以此为准，动手前先读。
- 权威文档：`_bmad-output/planning-artifacts/architecture.md`、`prd-egosync.md`；运行/发布细节 → README.md 与 `.github/workflows/`。

## 常用命令

- 桌面端（`egosync-app/` 下）：`npm run tauri dev`（完整开发环境）/ `npm run test:all`（vitest + cargo test）/ `npm run build`（tsc 类型检查 + 构建，本项目无独立 lint 脚本）/ `npm run tauri build`（当前平台安装包）
- 端到端（`egosync-app/tests/e2e/` 下，先 `npm install`）：`npm test` / `npm run test:ci` / `npm run pa11y`
- Android 原型（`companion-android/` 下）：`./gradlew :app:assembleDebug` / `./gradlew :app:testDebugUnitTest`

## 完成定义（DoD）

- `npm run build` 通过（tsc 零类型错误）；涉及 Rust 的改动 `npm run test:all` 全绿
- E2E 相关改动须跑 `tests/e2e` 对应脚本；无法执行的验证必须说明原因，禁止声称“已验证”

## Never 列表

- 绝不自动 push、绝不强制 push
- 绝不为让测试变绿而删测试或改断言
- 绝不提交密钥/.env/生产配置；绝不手改生成物（src-tauri/target 等）
- 连续失败 3 次：停止尝试，附完整报错向用户报告

## Commit 规范

- `<type>(<scope>): 中文描述`，type 用 feat/fix/docs/chore/test/refactor（沿用既有风格）

## 个人通用约定

1. 相关文档和代码注释等采用中文。
2. 如果项目基于 bmad 框架驱动开发且存在 project-context.md，项目细节放在该文档中维护；AGENTS.md 只保留地图、关键命令与硬约束。
3. Windows 系统用 `python`；Ubuntu/macOS 用 `python3`。
4. 如果输出的文档预计比较长，请分块输出，避免超时。
5. 需要我确认或做决定时，请用大白话解释，把我当成完全不懂技术的朋友：不堆专业名词，绕不开的术语先拿生活里的比方讲明白；几句话说清——出了什么事、为什么需要我拿主意、每种选法各有什么结果、你推荐哪种、为什么。
