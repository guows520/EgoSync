---
用例编号: UAT-THEME-007
测试模块: 字体加载与路径别名/质量门禁
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, gui_deps_installed: true }
      auto_generatable: true
      requirement:
    - type: 运行中的应用
      ref: app-running
      state: { running: true, network_available: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 运行只读类型检查/测试命令并目视字体加载，不改动数据。需网络以加载 Google Fonts。
---

# UAT-THEME-007 字体完整加载、路径别名可用、测试与类型检查通过

## 业务场景
设计要求 Inter、Noto Sans SC、JetBrains Mono 均以 swap 方式加载避免文字闪烁，同时引入 @ 路径别名提升开发体验。质量门禁需保持绿色。

## 前置条件
- 仓库依赖已安装，有网络可访问 Google Fonts。

## 测试步骤
1. 启动应用，观察首屏文字是否出现长时间空白（FOIT）后才显示。
2. 检查中文、英文及等宽场景下字体是否正确呈现。
3. 在 `egosync-app/` 运行 TypeScript 类型检查（tsc --noEmit）。
4. 运行前端测试命令。

## 预期结果
- 字体以 swap 方式加载，无 FOIT（无长时间空白文字）。
- 三种字体（含 JetBrains Mono 等宽）均可用。
- @ 路径别名可正常解析，类型检查零错误，前端测试全部通过。

## 实际结果

## 测试结论
