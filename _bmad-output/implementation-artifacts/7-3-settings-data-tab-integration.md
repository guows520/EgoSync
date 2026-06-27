---
baseline_commit: 0c9a1831d0d9d3c28c05ccf48d3ece37228e4ffd
---

# Story 7.3: GlobalSettingsModal 数据 Tab 接通真实导出和销毁功能

Status: done

## Story

As a 用户,
I want 在全局设置中方便地找到数据管理功能,
so that 导出和销毁操作简单直观。

## 背景与现状（务必先读）

**⚠️ 关键发现：本 Story 的验收标准已由 Story 7.1 和 7.2 的实现完整覆盖。**

Story 7.1 实现了导出后端 + 前端导出按钮接通；Story 7.2 实现了销毁后端 + 前端销毁确认 UI 接通。两者均已完成（status: done）并通过代码审查。本 Story 是 Epic 7 中的"集成验证"故事——确认数据 Tab 整体体验符合预期，补齐可能遗漏的集成测试。

**核心交付：**
1. 验证数据 Tab 的导出区域和危险区域布局符合 AC
2. 验证导出按钮的 loading → success/error 状态流转
3. 验证销毁按钮的确认流程（输入文字 → 按钮启用 → 执行 → 跳转 onboarding）
4. 补齐端到端集成测试（如有缺口）
5. 确认无 mock 残留

### 已建成的基础（本 story 的验证对象）

**前端 GlobalSettingsModal 数据 Tab（已实现，需验证）：**
- `src/components/settings/GlobalSettingsModal.tsx:821-1004` — `tab === 'data'` 完整区域
- `:823-910` — 导出区域：标题 + 说明 + 格式选择 + "导出存档"按钮 + loading + success/error
- `:949-1003` — 危险区域：红色标题 + 警告说明 + "销毁所有数据"按钮 + 确认流程
- `:299-320` — `handleExport` 函数：调用 `dataService.dataExport(selectedFormats)`
- `:322-336` — `handleDestroy` 函数：调用 `dataService.dataDestroy()` → `onDataDestroyed()`
- `:67-75` — 导出/销毁状态变量

**前端 service 层（已实现，需验证）：**
- `src/services/dataService.ts:1-16` — `dataExport` + `dataDestroy` 封装 invoke 调用

**Rust 后端（已实现，需验证复用）：**
- `src-tauri/src/commands/data.rs:9-62` — `data_export` command（含 rfd 目录选择）
- `src-tauri/src/commands/data.rs:64-76` — `data_destroy` command
- `src-tauri/src/services/data_export.rs` — 导出 + 销毁 service 逻辑

**App.tsx 销毁后跳转（已实现，需验证）：**
- `src/App.tsx` — `handleDataDestroyed` 回调：关闭设置 → 刷新角色 → 跳转 onboarding
- `src/App.tsx:393` — `onDataDestroyed={handleDataDestroyed}` prop 传递

**现有测试（已实现，需验证覆盖度）：**
- `src/components/settings/GlobalSettingsModal.test.tsx:290-406` — 数据导出测试组（5 个测试）
- `src/components/settings/GlobalSettingsModal.test.tsx:408-511` — 数据销毁测试组（6 个测试）

### 已知偏差（已接受，无需修改）

**目录选择方案偏差：** Epic AC 写"导出使用 `@tauri-apps/api/dialog` 的 `save` API"，实际实现使用 Rust 端 `rfd::FileDialog::new().pick_folder()`（方案 B）。Story 7.1 代码审查已明确接受此偏差——方案 B 避免了 dialog capability 声明，且已满足 AC1 功能需求。**本 story 不需要修改此实现。**

## Acceptance Criteria

1. **AC1**: Given 用户在 GlobalSettingsModal 点击"数据与主权" Tab，When Tab 打开，Then 上方显示导出区域：标题 + 说明 + "导出存档"按钮，And 下方显示危险区域：红色标题 + 警告说明 + "销毁所有数据"按钮

2. **AC2**: Given 导出按钮点击，When 导出执行中，Then 按钮变为加载状态（spinner + "导出中..."），And 完成后显示 ✓ "导出完成" + 文件路径

3. **AC3**: Given 销毁按钮点击，When 二次确认弹窗显示，Then 确认输入框 + 取消/确认按钮，And 确认按钮在用户输入正确文字前禁用

4. **AC4**: Given 前端，Then GlobalSettingsModal 数据 Tab 接通真实后端（替换原型 mock 按钮），And 销毁确认 Modal 新增（内联在数据 Tab 中）

5. **AC5**: Given Rust 后端，Then 复用 Story 7.1 / 7.2 的 Tauri commands

## Tasks / Subtasks

- [x] **Task 1: 验证数据 Tab 布局符合 AC1** (AC: #1)
  - [x] 1.1 打开 `GlobalSettingsModal.tsx`，确认 `tab === 'data'` 区域结构：导出区域（上方）→ 归档角色（中间）→ 危险区域（下方）
  - [x] 1.2 确认导出区域包含：标题"导出数据" + 说明文字 + "导出存档"按钮
  - [x] 1.3 确认危险区域包含：红色标题"危险区域" + 警告说明 + "销毁所有数据"按钮
  - [x] 1.4 如布局与 AC 不符，调整 DOM 结构使其符合

- [x] **Task 2: 验证导出状态流转符合 AC2** (AC: #2)
  - [x] 2.1 确认 `handleExport` 函数：点击"导出存档" → 显示格式选择 → 选格式 → 点"确认导出" → `isExporting=true`（spinner + "导出中..."）→ 成功显示 ✓ "导出完成" + 文件路径 / 失败显示错误
  - [x] 2.2 确认取消目录选择时静默处理（`result.files.length === 0` 时不显示成功或错误）
  - [x] 2.3 如状态流转与 AC 不符，修复 `handleExport` 逻辑

- [x] **Task 3: 验证销毁确认流程符合 AC3** (AC: #3)
  - [x] 3.1 确认"销毁所有数据"按钮点击后显示内联确认区域（非 Modal 弹窗）
  - [x] 3.2 确认警告文字："此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。"
  - [x] 3.3 确认输入框 placeholder：`输入"确认销毁"以继续`
  - [x] 3.4 确认"确认销毁"按钮在 `destroyConfirmText !== '确认销毁'` 时 `disabled`
  - [x] 3.5 确认销毁成功后调用 `onDataDestroyed` → App.tsx 关闭设置 + 刷新角色 + 跳转 onboarding
  - [x] 3.6 如流程与 AC 不符，修复 `handleDestroy` 或确认 UI 逻辑

- [x] **Task 4: 验证无 mock 残留符合 AC4** (AC: #4)
  - [x] 4.1 确认 `dataService.ts` 中 `dataExport` 和 `dataDestroy` 均调用真实 `invoke()`
  - [x] 4.2 确认 `GlobalSettingsModal.tsx` 中导出/销毁按钮的 `onClick` 均调用 `handleExport`/`handleDestroy`，无 mock handler
  - [x] 4.3 确认 `App.tsx` 中 `handleDataDestroyed` 回调正确传递给 `<GlobalSettingsModal>`

- [x] **Task 5: 验证后端复用符合 AC5** (AC: #5)
  - [x] 5.1 确认 `commands/data.rs` 中 `data_export` 和 `data_destroy` 已注册在 `lib.rs` 的 `invoke_handler` 中
  - [x] 5.2 确认无新增 command 需求——本 story 纯前端验证 + 测试补齐

- [x] **Task 6: 补齐集成测试** (AC: #1, #2, #3, #4)
  - [x] 6.1 检查现有测试覆盖度：导出测试 6 个 + 销毁测试 6 个（原 5+6，实际导出有 6 个）
  - [x] 6.2 补齐缺失测试（4 个新增）：
    - 导出区域和危险区域同时可见的布局验证测试
    - 导出 loading 状态显示验证（`isExporting` 时显示 spinner + "导出中..."）
    - 销毁 loading 状态显示验证（`isDestroying` 时显示 spinner + "销毁中..."）
    - 导出成功后文件路径列表显示验证（多格式场景）
  - [x] 6.3 运行 `npx vitest` 确认所有测试通过（29/29 passed）

- [x] **Task 7: 运行全量验证** (AC: #1-5)
  - [x] 7.1 运行 `npx vitest` 确认前端测试全部通过（331/332，1 个预存失败与本案无关）
  - [x] 7.2 运行 `cargo test` 确认后端测试全部通过（653/653 passed，无回归）
  - [x] 7.3 修复了 3 个预存测试 bug（JSON 数据→JSON 格式文本不匹配），新增 4 个集成测试

## Dev Notes

### 关键技术决策

- **本 story 是验证型 story**：7.1 和 7.2 已完成所有实现工作。本 story 的核心价值是验证集成正确性、补齐测试缺口、确认无 mock 残留。

- **目录选择方案偏差已接受**：Epic AC 写"导出使用 `@tauri-apps/api/dialog` 的 `save` API"，实际使用 Rust 端 `rfd::FileDialog`。Story 7.1 代码审查已接受此偏差（见 7-1-data-export-json-markdown.md Review Findings）。**不要修改此实现。**

- **归档角色区域位置**：当前数据 Tab 布局为 导出区域 → 归档角色 → 危险区域。AC1 只要求"上方显示导出区域"和"下方显示危险区域"，归档角色在中间不违反 AC。

### 架构合规

- **分层规则**：本 story 不新增 command 或 service，仅验证现有实现
- **命名规范**：无新增文件，无需关注命名
- **错误处理**：验证现有错误处理模式（`AppError` 序列化为 JSON → 前端提取 message）
- **serde 桥接**：无新增结构体

### 前端 UI 规范

- **不新增组件文件**：所有验证和修复在现有 `GlobalSettingsModal.tsx` 内
- **Tailwind 样式**：如有微调，遵循现有按钮样式模式
- **Loading 状态**：`isExporting`/`isDestroying` + `Loader2` 图标 + `animate-loading-spin` class
- **成功反馈**：`Check` 图标 + 绿色文字 + 文件路径
- **错误反馈**：`AlertCircle` 图标 + 红色文字

### 反模式警告

- **不要**修改 `commands/data.rs` 中的目录选择方案——偏差已接受
- **不要**新增 Rust command 或 service——本 story 纯验证 + 测试
- **不要**修改 `dataService.ts` 中的 invoke 调用——已正确封装
- **不要**重构现有 `handleExport`/`handleDestroy` 逻辑——除非验证发现 bug
- **不要**修改现有通过的测试——只新增缺失测试
- **不要**修改 `error.rs` 枚举

### Project Structure Notes

新增文件：无

修改文件（仅在验证发现问题时）：
- `src/components/settings/GlobalSettingsModal.tsx` — 如验证发现布局或状态流转问题
- `src/components/settings/GlobalSettingsModal.test.tsx` — 补齐缺失测试

不修改文件：
- `src-tauri/src/commands/data.rs` — 已实现，复用 7.1/7.2
- `src-tauri/src/services/data_export.rs` — 已实现，复用 7.1/7.2
- `src/services/dataService.ts` — 已实现，无需修改
- `src/App.tsx` — `handleDataDestroyed` 已实现
- `src-tauri/src/lib.rs` — command 注册已完成

### Previous Story Intelligence

**Story 7.1 学习要点：**
- 导出格式选择采用 inline checkbox 模式（非 Modal 弹窗）
- 目录选择用 Rust 端 `rfd::FileDialog`（方案 B），前端不调用 `@tauri-apps/api/dialog`
- 取消目录选择返回空 `ExportResult`，前端静默处理
- Loading spinner 用 `animate-loading-spin`（非 `animate-spin`）
- 导出成功显示文件路径列表（`exportResult.files.map()`）

**Story 7.2 学习要点：**
- 销毁确认采用 inline 区域（非 Modal 弹窗），在危险区域内展开
- 确认按钮 `disabled` 条件：`destroyConfirmText !== '确认销毁'`
- 销毁成功后通过 `onDataDestroyed` 回调通知 App.tsx，由父组件控制跳转 onboarding
- 销毁中状态：spinner + "销毁中..." 文字，禁用确认和取消按钮
- 错误提取模式：`e.ValidationError || e.DbError || Object.values(e)[0]`

### Git Intelligence

最近 2 个提交：
- `0c9a183` feat(7.2): 数据销毁功能 + 代码审查修复
- `fbdbc83` feat(7.1): 数据导出功能 + 代码审查修复 + 导出体验优化

这两个提交已完整实现数据 Tab 的导出和销毁功能。本 story 验证这两个提交的集成正确性。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.3] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7] — Epic 7 上下文
- [Source: _bmad-output/implementation-artifacts/7-1-data-export-json-markdown.md] — Story 7.1 实现详情 + Review Findings
- [Source: _bmad-output/implementation-artifacts/7-2-data-destroy-initial-state.md] — Story 7.2 实现详情 + Review Findings
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、禁止事项
- [Source: src/components/settings/GlobalSettingsModal.tsx:821-1004] — 数据 Tab 完整 UI
- [Source: src/components/settings/GlobalSettingsModal.tsx:299-336] — handleExport + handleDestroy
- [Source: src/services/dataService.ts:1-16] — dataService 封装
- [Source: src-tauri/src/commands/data.rs:1-77] — data_export + data_destroy commands
- [Source: src/App.tsx:387-394] — GlobalSettingsModal 渲染 + onDataDestroyed prop
- [Source: src/components/settings/GlobalSettingsModal.test.tsx:290-511] — 现有数据 Tab 测试

## Dev Agent Record

### Agent Model Used

Claude (Cascade) — 2026-06-27

### Debug Log References

- 3 个预存测试失败：`GlobalSettingsModal.test.tsx` 中 `/JSON 数据/` 匹配文本与 UI 实际文本"JSON 格式"不一致，导致 3 个测试失败
- 根因：测试编写时使用了预期的标签文本"JSON 数据"，但实现时 UI 文本为"JSON 格式"
- 修复：将 3 处 `/JSON 数据/` 改为 `/JSON 格式/`

### Completion Notes List

- ✅ Task 1-5: 验证数据 Tab 布局、导出状态流转、销毁确认流程、无 mock 残留、后端复用——全部符合 AC，无需修改实现代码
- ✅ Task 6: 修复 3 个预存测试 bug（JSON 数据→JSON 格式），新增 4 个集成测试（布局验证、导出 loading、销毁 loading、多格式路径）
- ✅ Task 7: 前端 331/332 passed（1 个预存失败 `SettingsTab.test.tsx` 与本案无关），后端 653/653 passed 无回归
- 本 story 是验证型 story，7.1/7.2 已完成所有实现，本 story 确认集成正确性并补齐测试缺口

### File List

- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx` — 修复 3 处测试文本不匹配 + 新增 4 个集成测试 + 代码审查修复（`act` 导入 + 2 处 `await act()` 包裹消除 React act 警告）

### Change Log

- 2026-06-27: 修复 3 个预存测试 bug（`/JSON 数据/` → `/JSON 格式/`），新增 4 个集成测试（布局验证、导出 loading、销毁 loading、多格式路径显示）
- 2026-06-27: 代码审查修复 — 将 2 处延迟 promise 解析包裹进 `await act(async () => {...})`，消除 React act() 警告

### Review Findings

- [x] [Review][Patch] loading 测试在断言后解析 deferred promise，未包裹 `act()`/未 await，触发 React "not wrapped in act(...)" 警告 — 已修复：将两处 `resolve*()` 包裹进 `await act(async () => {...})`，新增 `act` 导入；实证 `vitest run` 29/29 通过，两个 loading 测试不再输出 act 警告 [egosync-app/src/components/settings/GlobalSettingsModal.test.tsx:437,597]

> 说明：测试文件中仍有预存的 act 警告，归属于未改动的 `展示 MCP 工具标签入口并预留全局管理区块` 测试，属故事 7-3 范围外的既有问题，本次未处理。
