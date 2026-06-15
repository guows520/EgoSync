---
用例编号: UAT-QA-002
测试模块: Rust 测试链路
story_key: 1-2-rust-frontend-test-infrastructure
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, rust_toolchain_ready: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅运行 cargo test，不改动持久化数据；首次运行含编译耗时。
---

# UAT-QA-002 开发者运行 Rust 测试并看到单元+集成测试通过

## 业务场景
后端开发者希望确认 Rust 侧测试链路可用，运行测试后看到单元测试和集成测试都被执行并通过。

## 前置条件
- Rust 工具链就绪，`egosync-app/src-tauri/` 可编译。

## 测试步骤
1. 进入 `egosync-app/src-tauri/` 目录。
2. 执行 Rust 测试命令（cargo test）。
3. 查看测试运行汇总与退出码。

## 预期结果
- Rust 单元测试与集成测试均被运行，至少各 1 个通过。
- 命令退出码为 0，终端显示 passed 结果。

## 实际结果
在 `egosync-app/src-tauri/` 执行 `cargo test`：

```
running 344 tests
test result: ok. 344 passed; 0 failed; 0 ignored   (单元测试 src/lib.rs)
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored      (集成测试 tests/test_app.rs)
```

- 单元测试（lib）344 个全部通过，集成测试（tests/test_app.rs）1 个通过，单元+集成均被执行且各至少 1 个通过。
- 命令退出码为 0，终端显示 passed 结果。

## 测试结论
Pass
