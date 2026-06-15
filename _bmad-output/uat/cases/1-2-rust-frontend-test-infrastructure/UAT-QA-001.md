---
用例编号: UAT-QA-001
测试模块: 前端测试链路
story_key: 1-2-rust-frontend-test-infrastructure
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, gui_deps_installed: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅运行只读测试命令，不改动业务数据；测试在隔离的 jsdom 环境中执行。
---

# UAT-QA-001 开发者一键运行前端测试并看到通过结果

## 业务场景
开发者在提交代码前希望快速验证前端没有被破坏，期望一条命令就能跑完前端测试并清晰看到通过/失败结论。

## 前置条件
- 仓库已克隆，`egosync-app/` 依赖已安装。

## 测试步骤
1. 进入 `egosync-app/` 目录。
2. 执行前端测试命令（test:frontend）。
3. 等待测试运行完毕，查看汇总结果与退出码。

## 预期结果
- Vitest 运行并报告至少 1 个测试通过。
- 命令退出码为 0（成功），终端显示绿色通过摘要。

## 实际结果
执行 `npm run test:frontend`（vitest run）：

```
Test Files  15 passed (15)
     Tests  164 passed (164)
  Duration  25.71s
```

- Vitest 正常运行，报告 164 个测试全部通过（远超"至少 1 个"门槛）。
- 命令退出码为 0，终端显示绿色通过摘要。

> 说明：vitest.config 原 `reporter: 'basic'` 在 Vitest 4 已移除，本次以默认 reporter 运行（仅报告呈现方式差异，不影响测试链路本身的可用性判定）。

## 测试结论
Pass
