---
用例编号: UAT-UI-004
测试模块: 代码结构质量门禁
story_key: 1-3-component-domain-split-visual-zero-regression
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, gui_deps_installed: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅运行类型检查/构建/前端测试只读命令，不改动数据。
---

# UAT-UI-004 拆分后类型检查、测试与构建全部通过

## 业务场景
重构不能引入类型错误或破坏测试。开发者期望拆分完成后类型检查零错误、现有测试通过、生产构建成功，证明重构是安全的。

## 前置条件
- 仓库依赖已安装。

## 测试步骤
1. 进入 `egosync-app/`，运行 TypeScript 类型检查（tsc --noEmit）。
2. 运行前端测试命令。
3. 运行生产构建命令。

## 预期结果
- 类型检查零错误。
- 前端测试全部通过。
- 生产构建成功产出产物。

## 实际结果
1. 类型检查（`tsc --noEmit`）：**失败，退出码 2**。报错 3 处，均来自 `src/components/settings/GlobalSettingsModal.test.tsx`：
   - `error TS2307: Cannot find module 'node:fs'`
   - `error TS2307: Cannot find module 'node:path'`
   - `error TS2591: Cannot find name 'process'`
2. 前端测试（vitest run）：通过（164 passed）。
3. 生产构建（`npm run build` = `tsc && vite build`）：**失败，退出码 2**——因 tsc 阶段先行报错而中断，未进入打包；单独运行 `vite build` 可成功（exit 0，产出 dist 产物）。

根因：`tsconfig.json` 的 `include: ["src"]` 把测试文件纳入类型检查，但 `compilerOptions.types` 仅含 `["vitest/globals","@testing-library/jest-dom"]`，缺 `node`，导致引用了 `node:fs/node:path/process` 的测试文件类型检查失败，进而阻断 `npm run build`。

### 复测（2026-06-15，修复后）
修复方案：`tsconfig.json` 增加 `"exclude": ["src/**/*.test.ts","src/**/*.test.tsx"]`，将 co-located 测试文件排除出生产类型检查（测试仍由 vitest 自有配置发现，不受影响）。
- 类型检查（`tsc`）：通过，零错误。✅
- 前端测试（vitest run）：164 passed / 15 文件，零回归。✅
- 生产构建（`npm run build` = `tsc && vite build`）：**退出码 0**，产出 dist 产物。✅
三项全部达标。

## 测试结论
Pass（复测通过，原 Fail 已由 bmad-investigate 立案 + 修复后解决）
