---
用例编号: UAT-QA-005
测试模块: Rust 测试自动发现
story_key: 1-2-rust-frontend-test-infrastructure
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 临时 Rust 集成测试文件
      ref: temp-integration-test
      state: { path: "egosync-app/src-tauri/tests/test_uat_probe.rs", contains_one_passing_test: true }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 临时新增一个测试文件验证自动发现，验证后删除该文件，避免污染代码库。
---

# UAT-QA-005 新增 Rust 测试文件无需改配置即被自动运行

## 业务场景
后续每个 Story 都会新增 Rust 测试。开发者期望只要把测试文件放进 tests 目录，运行测试时就能自动被发现并执行，不必修改任何配置。

## 前置条件
- Rust 工具链就绪，`egosync-app/src-tauri/tests/` 目录存在。

## 测试步骤
1. 在 `egosync-app/src-tauri/tests/` 新建一个临时测试文件，内含一个一定通过的断言。
2. 不修改任何配置文件，直接运行 Rust 测试命令。
3. 查看测试报告中是否包含新文件的测试用例。
4. 删除临时测试文件。

## 预期结果
- 新建的测试文件被 cargo test 自动发现并执行，结果计入通过数。
- 无需修改 Cargo.toml 或其他配置即可生效。

## 实际结果

## 测试结论
