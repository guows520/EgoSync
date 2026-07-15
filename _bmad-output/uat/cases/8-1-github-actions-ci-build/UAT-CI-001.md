---
用例编号: UAT-CI-001
测试模块: CI构建
story_key: 8-1-github-actions-ci-build
version_anchor: ad076308d48e02b7489ca154bd466d1655bc4263
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 代码仓库
      ref: EgoSync GitHub 仓库
      state: { 可访问: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: CI 只读验证，不修改源码。
---

# UAT-CI-001 三平台并行CI构建并生成安装包

## 业务场景
开发者希望每次代码推送自动在 Windows/macOS/Linux 三平台构建并生成安装包，确保跨平台兼容性且随时可发布，不用手动逐平台构建。

## 前置条件
- 代码推送到 GitHub 触发 CI

## 测试步骤
1. 推送代码到仓库
2. 等待 CI workflow 触发
3. 观察三平台 matrix 并行构建
4. 检查产物上传

## 预期结果
- 三平台（ubuntu/macos/windows）并行构建
- 构建步骤执行 npm run tauri build
- 生成各平台安装包
- 产物命名 egosync-{version}-{platform}.{ext}
- 使用 actions/upload-artifact@v4 上传到 GitHub Actions Artifacts
- 构建失败时 CI 报错

## 实际结果
<留空>

## 测试结论
<留空>
