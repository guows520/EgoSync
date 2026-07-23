# Story 9.4: 补齐局部深色模式并统一“数字分身管家”文案

Status: review

## Story

As a 使用深色模式的用户,
I want 建议卡片和顶部选中标签在深色主题下清晰可辨，并看到统一的“数字分身管家”名称,
so that 主题切换后界面仍保持一致、可读且术语明确。

## Acceptance Criteria

1. **AC-1 建议卡片完整支持深色模式**
   - Given 应用处于深色模式
   - Then 待确认、已确认、已拒绝建议卡片的背景、边框、标题、正文、角色名、时间、状态图标均可读
   - And 拒绝原因区域、原因选项、“其他”输入与提交按钮均有匹配的深色样式
   - And hover、selected、disabled 状态不会退回亮白背景或低对比文本

2. **AC-2 顶部导航选中态支持深色模式**
   - Given 用户处于管家或角色页面的深色模式
   - When “仪表盘 / 任务 / 设置”或“任务 / 记忆 / 设置”等标签被选中
   - Then 选中态使用适合深色背景的底色、阴影/边框和文字颜色
   - And 未选中 hover 状态也不会出现突兀亮白块
   - And 角色主题色仍可作为强调色，但不能牺牲对比度

3. **AC-3 前端可见术语统一**
   - Then Butler 主界面标题由“分身管家”改为“数字分身管家”
   - And Onboarding 中相同用户可见称呼同步修改
   - And 相关前端测试、快照或查询断言同步更新

4. **AC-4 Rust Prompt 保持不变**
   - Then `src-tauri` 下 Prompt 中的“分身管家”不修改
   - And 不因 UI 文案更新改变角色身份逻辑、系统提示或后端测试

5. **AC-5 全局主题机制不改写**
   - Then 继续使用现有 Tailwind `dark:` variants 与 `.dark` class
   - And 不新增第二套主题状态、不替换全局主题切换机制、不进行无关样式重构

6. **AC-6 回归验证**
   - Then 浅色模式视觉不回归
   - And 键盘焦点、reduced-motion 和建议确认/拒绝行为保持不变
   - And `npm run test:frontend` 与 `npm run build` 通过

## Tasks / Subtasks

- [x] Task 1: 建议卡片深色样式（AC: #1, #5, #6）
  - [x] 1.1 为 `ActionCard` 根容器、文本、状态图标、按钮和拒绝区域补充最小 dark variants
  - [x] 1.2 覆盖 pending/confirmed/rejected 与 other reason 输入路径
  - [x] 1.3 保持所有事件处理、动画和状态机不变

- [x] Task 2: 导航选中态深色样式（AC: #2, #5, #6）
  - [x] 2.1 修复 `ButlerView` 顶部 tabButton 的 active/inactive/hover 深色样式
  - [x] 2.2 修复 `RoleHeader` 顶部 tabButton 的 active/inactive/hover 深色样式
  - [x] 2.3 验证动态角色色 `style` 与 dark class 不冲突

- [x] Task 3: 文案更新（AC: #3, #4）
  - [x] 3.1 更新 `ButlerView` 用户可见标题
  - [x] 3.2 更新 `OnboardingView` 用户可见消息和标题
  - [x] 3.3 搜索并更新相关前端测试断言，明确排除 `src-tauri`

- [x] Task 4: 测试和视觉验证（AC: #1-#6）
  - [x] 4.1 覆盖深色 class 和三种建议状态
  - [x] 4.2 覆盖管家/角色导航 active 状态
  - [x] 4.3 覆盖“数字分身管家”可见且 Rust Prompt 未发生修改（通过文件清单/差异审查）
  - [x] 4.4 运行 `npm run test:frontend` 与 `npm run build`

## Dev Notes

- 全局主题机制已正常工作，问题是局部组件缺少 `dark:` variants；不要修改 `App.tsx` 主题切换逻辑或 Tailwind darkMode 配置。
- 本 Story 只修改前端用户可见术语。用户明确要求 Rust Prompt 不改，代码审查时必须检查 diff 中不包含 `src-tauri` Prompt 文件。
- 不要“顺手”统一全项目颜色 token；仅修复已确认的三个组件区域。

### Project Structure Notes

- 主要修改：
  - `egosync-app/src/components/butler/ActionCard.tsx`
  - `egosync-app/src/components/butler/ButlerView.tsx`
  - `egosync-app/src/components/role/RoleHeader.tsx`
  - `egosync-app/src/components/onboarding/OnboardingView.tsx`
  - 对应前端测试

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/ui-settings-display-packaging-issues-investigation.md` — Follow-up #3/#4/#7]
- [Source: `egosync-app/src/components/butler/ButlerView.tsx:94-107`]
- [Source: `egosync-app/src/components/role/RoleHeader.tsx:27-39`]
- [Source: `egosync-app/src/components/butler/ActionCard.tsx:136-257`]
- [Source: `egosync-app/src/components/onboarding/OnboardingView.tsx:178,306`]

## Dev Agent Record

### Agent Model Used

Codex（GPT-5.6）

### Debug Log References

- `npm run test:frontend`：42 个测试文件、381 项测试通过。
- `npm run build`：TypeScript 与 Vite 生产构建通过；保留既有 chunk size 警告。
- `npx tauri build --bundles msi,nsis`：MSI 与 NSIS 两种 Windows bundle 构建通过；保留既有 Rust 编译警告。

### Completion Notes List

- 为建议卡片的容器、文本、优先级、状态、拒绝原因和输入控件补齐深色样式。
- 管家及角色顶部页签补齐 active/inactive/hover 深色状态，保留角色动态颜色。
- 前端可见文案统一为“数字分身管家”；`src-tauri` Rust 源码差异为空。

### File List

- `egosync-app/src/components/butler/ActionCard.tsx`
- `egosync-app/src/components/butler/ActionCard.test.tsx`
- `egosync-app/src/components/butler/ButlerView.tsx`
- `egosync-app/src/components/butler/ButlerView.test.tsx`
- `egosync-app/src/components/onboarding/OnboardingView.tsx`
- `egosync-app/src/components/role/RoleHeader.tsx`
- `egosync-app/src/components/role/RoleHeader.test.tsx`

### Change Log

- 2026-07-22：完成实现、自动化测试、生产构建与 Windows bundle 验证，状态更新为 Review。
