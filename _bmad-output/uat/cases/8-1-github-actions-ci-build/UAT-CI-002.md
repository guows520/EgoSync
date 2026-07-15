---
用例编号: UAT-CI-002
测试模块: CI构建
story_key: 8-1-github-actions-ci-build
version_anchor: ad076308d48e02b7489ca154bd466d1655bc4263
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 代码仓库
      ref: EgoSync GitHub 仓库
      state: { 可访问: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证测试步骤保留。
---

# UAT-CI-002 CI构建前先跑前端与Rust测试

## 业务场景
开发者希望 CI 在构建安装包之前先跑前端测试和 Rust 测试，测试失败就不必浪费资源构建，确保发布产物来自通过测试的代码。

## 前置条件
- CI workflow 触发

## 测试步骤
1. 推送代码
2. 观察 CI 步骤顺序
3. 制造一个测试失败，观察是否阻止构建

## 预期结果
- CI 步骤顺序：checkout → 依赖安装 → 前端测试 → Rust 测试 → tauri build → 产物上传
- 前端测试或 Rust 测试失败时不执行 tauri build
- 测试通过后才构建安装包

## 实际结果
<留空>

## 测试结论
<留空>
