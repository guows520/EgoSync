---
用例编号: UAT-QA-003
测试模块: 全量测试一键运行
story_key: 1-2-rust-frontend-test-infrastructure
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, gui_deps_installed: true, rust_toolchain_ready: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅运行测试，不改动数据。前端测试与 Rust 测试顺序执行。
---

# UAT-QA-003 一条命令跑完前端+Rust 全量测试

## 业务场景
开发者在提交前希望用单一命令同时验证前端与后端，避免分别记两条命令、漏跑某一侧。

## 前置条件
- 前端依赖与 Rust 工具链均就绪。

## 测试步骤
1. 进入 `egosync-app/` 目录。
2. 执行全量测试命令（test:all）。
3. 观察前端测试与 Rust 测试是否依次运行并全部通过。

## 预期结果
- 先运行前端测试再运行 Rust 测试，两侧全部通过。
- 整体退出码为 0；任一侧失败应导致整体非 0。

## 实际结果
分别验证 `test:all` 串联的两侧（vitest run && cargo test），并确认其串联语义：

- 前端侧：Vitest 164 passed，退出码 0。
- 后端侧：cargo test 344 单元 + 1 集成 passed，退出码 0。
- `test:all` 定义为 `vitest run && cd src-tauri && cargo test`，`&&` 短路语义保证任一侧失败则整体非 0；两侧均成功故整体退出码为 0。

## 测试结论
Pass
