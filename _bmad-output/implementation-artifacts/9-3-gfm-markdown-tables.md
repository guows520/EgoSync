# Story 9.3: 对话消息正确渲染标准 GFM Markdown 表格

Status: review

## Story

As a 在对话中阅读结构化信息的用户,
I want AI 输出的标准 Markdown 表格被渲染为可读表格,
so that 城市、任务、对比项等多列信息不会退化为难以阅读的普通段落。

## Acceptance Criteria

1. **AC-1 标准 GFM 表格解析为 HTML 表格**
   - Given 消息包含带管道符和表头分隔行的标准 GFM 表格
   - When `ChatBubble` 渲染消息
   - Then DOM 包含 `table`、`thead`、`tbody`、`th`、`td`
   - And 中文、emoji、粗体、链接等内联 Markdown 正常渲染

2. **AC-2 不兼容 TAB 表格**
   - Given 输入仅使用 TAB 分隔且没有 GFM 管道和分隔行
   - Then 不增加自动转换、启发式识别或预处理逻辑
   - And 普通段落与代码块中的 TAB 保持现有含义

3. **AC-3 表格视觉与溢出处理**
   - Then 表格拥有清晰的表头、单元格边框、合理间距和行背景
   - And 宽表格在消息区域内横向滚动，不撑破气泡或页面布局
   - And 浅色与深色模式均具有足够可读性

4. **AC-4 现有 Markdown 扩展不回归**
   - Then 现有链接安全转换、代码块复制、记忆引用链接与普通 Markdown 行为保持不变
   - And fenced code 中看起来像表格的文本仍是代码块

5. **AC-5 生成格式约束**
   - Given 应用中存在面向前端可见回答格式的 TypeScript 提示或模板
   - Then 只在相关前端/TypeScript 生成约束中要求表格采用标准 GFM 形式
   - And 不修改 Rust Prompt
   - And 若当前没有合适的前端生成约束入口，则不为此 Story 新建一套提示系统，以渲染端支持和测试为完成基线

6. **AC-6 依赖与验证**
   - Then 项目增加与当前 `react-markdown` 兼容的 `remark-gfm` 依赖并通过 lockfile 固定
   - And `npm run test:frontend` 与 `npm run build` 通过

## Tasks / Subtasks

- [x] Task 1: 接入 GFM 解析（AC: #1, #4, #6）
  - [x] 1.1 安装 `remark-gfm`，同步 `package.json` 和 lockfile
  - [x] 1.2 在 `ChatBubble` 的 `ReactMarkdown` 上配置 `remarkPlugins={[remarkGfm]}`
  - [x] 1.3 保留现有 `urlTransform` 和 `components` 映射

- [x] Task 2: 表格渲染样式（AC: #3, #4）
  - [x] 2.1 增加 `table` 容器的横向滚动策略
  - [x] 2.2 为 `table/thead/th/td/tr` 增加与现有 typography 风格一致的浅色/深色 class
  - [x] 2.3 不改变代码块组件和记忆引用处理

- [x] Task 3: 测试（AC: #1-#6）
  - [x] 3.1 使用标准 GFM 城市表格断言 table 语义结构和内容
  - [x] 3.2 断言中文、emoji、粗体、链接正常
  - [x] 3.3 断言 fenced code 中的管道表格不被解析为 table
  - [x] 3.4 断言 TAB 文本不会被自定义转换为 table
  - [x] 3.5 运行 `npm run test:frontend` 与 `npm run build`

## Dev Notes

- 根因已通过实际静态渲染确认：当前 `ReactMarkdown` 未配置 `remark-gfm`，标准管道表格也会成为普通段落。
- 用户明确不要求 TAB 格式兼容，因此禁止增加正则预处理、列数猜测或 TAB→管道转换。
- 不引入 Mermaid、PlantUML 或图表库；此需求是 Markdown 表格而不是图表。
- 依赖版本以当前安装时与 `react-markdown@10.1.0` 兼容且测试通过为准，不在 Story 中猜测硬编码未来版本。

### Project Structure Notes

- 主要修改：
  - `egosync-app/src/components/chat/ChatBubble.tsx`
  - `egosync-app/package.json`
  - 项目现有 npm lockfile
  - 对应 ChatBubble 前端测试
- 不修改 `src-tauri` Rust Prompt。

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/ui-settings-display-packaging-issues-investigation.md` — Follow-up #6/#7]
- [Source: `egosync-app/src/components/chat/ChatBubble.tsx:272-288`]
- [Source: `egosync-app/package.json:14-26`]

## Dev Agent Record

### Agent Model Used

Codex（GPT-5.6）

### Debug Log References

- `npm run test:frontend`：42 个测试文件、381 项测试通过。
- `npm run build`：TypeScript 与 Vite 生产构建通过；保留既有 chunk size 警告。
- `npx tauri build --bundles msi,nsis`：MSI 与 NSIS 两种 Windows bundle 构建通过；保留既有 Rust 编译警告。

### Completion Notes List

- 接入 `remark-gfm`，标准 GFM 管道表格可渲染为语义化表格；未增加 TAB 文本兼容。
- 表格支持横向滚动、浅色/深色样式，并保留 URL 安全转换、代码块和记忆引用逻辑。
- 测试覆盖中文、emoji、粗体、链接、代码围栏和 TAB 非转换行为。

### File List

- `egosync-app/package.json`
- `egosync-app/package-lock.json`
- `egosync-app/src/components/chat/ChatBubble.tsx`
- `egosync-app/src/components/chat/ChatBubble.test.tsx`

### Change Log

- 2026-07-22：完成实现、自动化测试、生产构建与 Windows bundle 验证，状态更新为 Review。
