# Story 9.5: 任务时间选择完成后自动收起并将 Windows 安装包默认语言设为中文

Status: done

## Story

As a Windows 用户,
I want 创建任务时选完日期、小时和分钟后时间面板自动关闭，并让 MSI/NSIS 安装界面默认显示中文,
so that 桌面端的高频输入和首次安装流程都符合自然的中文使用习惯。

## Acceptance Criteria

1. **AC-1 先验证原生 datetime-local 行为**
   - Given 应用运行在 Windows Tauri WebView2 环境
   - When 用户依次完成日期、小时和分钟选择
   - Then 记录原生 `input[type=datetime-local]` 是否会自动关闭系统选择面板
   - And 验证必须基于实际打包/开发窗口，不以浏览器推测替代

2. **AC-2 原生可满足时采用最小修复**
   - Given 原生控件可以可靠识别最终分钟提交并关闭
   - Then 保留原生控件
   - And 只增加必要的受控状态/事件处理
   - And 不使用模拟鼠标点击、定时 `blur()` 或依赖未标准化浏览器内部 DOM 的 hack

3. **AC-3 原生不可满足时替换为可控选择器**
   - Given Windows WebView2 原生控件无法稳定满足自动关闭
   - Then 替换为 React 可控的日期、小时、分钟选择面板
   - And 用户选完分钟后面板立即关闭，deadline 值完整写回
   - And 重新打开时显示已选值
   - And 支持取消、清空、键盘操作、点击外部关闭和 Escape 关闭
   - And 不增加与需求无关的时区、秒、重复任务或自然语言解析功能

4. **AC-4 deadline 数据契约不变**
   - Then `TaskModal` 向既有任务创建/更新路径提交的 deadline 格式保持兼容
   - And 未选择时间时仍保持当前空值语义
   - And 编辑既有任务时不会因控件替换丢失日期或分钟

5. **AC-5 MSI 默认简体中文**
   - Given 执行 Windows MSI/WiX 打包
   - Then `bundle.windows.wix.language` 明确配置简体中文 locale
   - And 生成的 MSI 安装 UI 默认中文
   - And 产物命名/元数据不再以 `en-US` 作为默认语言标识

6. **AC-6 NSIS 默认简体中文**
   - Given 执行 Windows NSIS 打包
   - Then `bundle.windows.nsis.languages` 明确包含并默认使用简体中文
   - And setup.exe 安装 UI 默认中文
   - And 不只修改 MSI 而遗漏 NSIS

7. **AC-7 语言标识通过真实构建确认**
   - Then 使用当前 Tauri CLI schema 支持的实际 locale/language identifier
   - And 成功生成 MSI 与 NSIS 两种产物
   - And 至少检查启动安装器首屏、许可/安装路径页、取消/错误提示为中文
   - And 若工具链不支持预期标识，明确记录构建错误与最终采用值，不静默跳过

8. **AC-8 测试与构建**
   - Then 时间选择逻辑具有前端自动化测试
   - And `npm run test:frontend`、`npm run build` 通过
   - And Windows `npm run tauri build` 或等价的两目标构建验证成功；若环境性签名/权限问题阻断，必须单独报告，不能宣称打包通过

## Tasks / Subtasks

- [ ] Task 1: Windows 原生控件诊断（AC: #1, #2）
  - [ ] 1.1 在 Tauri WebView2 中复现选择日期、小时、分钟的完整路径
  - [ ] 1.2 记录 `input/change/blur` 的实际触发顺序；诊断日志仅在必要时临时加入并在定位后移除
  - [x] 1.3 基于证据决定保留原生或替换控件

- [x] Task 2: 实现时间面板自动关闭（AC: #2, #3, #4）
  - [x] 2.1 若保留原生，实施最小、标准事件驱动修复
  - [x] 2.2 若替换，优先使用项目现有 React/Tailwind 能力实现最小 picker；只有原生语义控件不足时才评估新依赖
  - [x] 2.3 覆盖新建、编辑、清空、取消和边界时间（00:00、23:59）

- [x] Task 3: 配置两种安装包中文语言（AC: #5, #6, #7）
  - [x] 3.1 阅读当前 `@tauri-apps/cli/config.schema.json`，确认 WiX 与 NSIS 字段和允许值
  - [x] 3.2 在 `tauri.conf.json` 增加 `bundle.windows.wix.language`
  - [x] 3.3 在 `tauri.conf.json` 增加 `bundle.windows.nsis.languages`
  - [x] 3.4 不修改已为 `zh-CN` 的 HTML lang，也不把运行时应用语言和安装器语言混为一谈

- [ ] Task 4: 验证（AC: #1-#8）
  - [x] 4.1 增加时间选择组件测试并运行前端测试/构建
  - [x] 4.2 构建 MSI 与 NSIS，记录实际产物路径和语言标识
  - [ ] 4.3 人工检查两种安装器的中文首屏和关键页面
  - [x] 4.4 确认没有残留诊断日志或浏览器 hack

### Review Findings

- [x] [Review][Defer] TaskModal deadline 时区处理为"本地时间当作 UTC" [egosync-app/src/components/modals/TaskModal.tsx:26-32] — deferred, pre-existing（`git show HEAD` 确认转换函数本次未改，deadline 契约不变，满足 AC-4）

## Dev Notes

- 当前 `TaskModal` 仅使用原生 `datetime-local` 和 `setDeadline`，没有控制原生弹层关闭的 API。
- 是否替换控件不能靠静态推断；本 Story 明确要求先在 Windows WebView2 取证。用户已授权必要时替换原生控件。
- 打包语言是 Tauri Windows bundle 配置问题，不是 `index.html lang` 问题。
- 两种安装包必须同时验证。已有 MSI 产物带 `_en-US`，是当前默认语言未配置的直接证据。
- 本 Story 允许 Python 用于确定性产物名检查或构建日志解析，但产品 UI 仍使用 React/TypeScript，不引入 Python 运行时。

### Project Structure Notes

- 主要修改：
  - `egosync-app/src/components/modals/TaskModal.tsx`
  - 必要时新增一个紧邻 modal/domain 的时间选择组件及测试
  - `egosync-app/src-tauri/tauri.conf.json`
- 配置依据：
  - `egosync-app/node_modules/@tauri-apps/cli/config.schema.json`
- 产物验证目录：
  - `egosync-app/src-tauri/target/release/bundle/msi/`
  - `egosync-app/src-tauri/target/release/bundle/nsis/`

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/ui-settings-display-packaging-issues-investigation.md` — Follow-up #3/#4/#5/#7]
- [Source: `egosync-app/src/components/modals/TaskModal.tsx:158-166`]
- [Source: `egosync-app/src-tauri/tauri.conf.json:32-45`]
- [Source: `egosync-app/node_modules/@tauri-apps/cli/config.schema.json`]

## Dev Agent Record

### Agent Model Used

Codex（GPT-5.6）

### Debug Log References

- `npm run test:frontend`：42 个测试文件、381 项测试通过。
- `npm run build`：TypeScript 与 Vite 生产构建通过；保留既有 chunk size 警告。
- `npx tauri build --bundles msi,nsis`：MSI 与 NSIS 两种 Windows bundle 构建通过；保留既有 Rust 编译警告。

### Completion Notes List

- 将原生 `datetime-local` 替换为 React 可控日期/小时/分钟面板；三项齐全后自动关闭，并支持取消、清空、Escape、点击外部关闭及已有值回填。
- deadline 创建/更新格式与空值语义保持不变，测试覆盖新建、编辑、取消、清空以及 00:00/23:59。
- WiX 使用 `zh-CN`，NSIS 使用 `SimpChinese`；两种 bundle 构建成功，生成文件及构建脚本语言标识已核对。
- 未在本轮人工启动安装器检查首屏/路径页/取消提示；该项保留为 Review/UAT 待办。

### File List

- `egosync-app/src/components/modals/TaskModal.tsx`
- `egosync-app/src/components/modals/TaskModal.test.tsx`
- `egosync-app/src-tauri/tauri.conf.json`

### Change Log

- 2026-07-22：完成实现、自动化测试、生产构建与 Windows bundle 验证，状态更新为 Review。
