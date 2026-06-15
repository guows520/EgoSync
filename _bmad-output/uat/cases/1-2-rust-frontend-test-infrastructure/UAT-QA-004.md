---
用例编号: UAT-QA-004
测试模块: CI 三平台门禁
story_key: 1-2-rust-frontend-test-infrastructure
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试分支
      ref: ci-trigger-branch
      state: { pushed: true, contains_trivial_change: true }
      auto_generatable: true
      requirement:
    - type: GitHub 仓库与 Actions
      ref: github-actions
      state: { workflow_present: ".github/workflows/ci.yml", actions_enabled: true }
      auto_generatable: false
      requirement: 需要可访问的 GitHub 仓库且已启用 Actions（含运行额度），用于触发三平台 matrix 运行。
  isolation: write-isolated
  notes: 在专用测试分支推送以触发 CI，不影响主分支；验证后删除测试分支。
---

# UAT-QA-004 推送代码触发三平台 CI 全绿

## 业务场景
团队希望每次推送都能在三大操作系统上自动验证构建与测试，任一平台失败都能拦住合并，保证跨平台质量。

## 前置条件
- GitHub 仓库已启用 Actions，存在 `.github/workflows/ci.yml`。
- 准备一个专用测试分支用于触发。

## 测试步骤
1. 在测试分支做一处无害改动并推送到 GitHub。
2. 打开 Actions 页面，观察 CI workflow 是否被触发。
3. 查看 ubuntu / macos / windows 三个 matrix job 的执行情况。
4. 检查每个 job 是否执行了 Rust 测试、前端测试与构建步骤。

## 预期结果
- CI workflow 被触发并在三平台并行运行。
- 每个平台均执行 cargo test + 前端测试 + 构建，全部成功（绿色）。
- 验证：若故意引入失败用例，对应平台 job 应变红并使整个 workflow 失败。

## 实际结果
[Blocked] 缺少前置物料：未提供「可访问且启用 Actions（含运行额度）的 GitHub 仓库」（数据需求表 B-5），无法推送测试分支触发三平台 CI matrix。需补充确认仓库中是否存在 `.github/workflows/ci.yml`。

## 测试结论
Blocked（阻塞原因：环境物料缺失 B-5 GitHub Actions 仓库；解除后转人工触发执行）
