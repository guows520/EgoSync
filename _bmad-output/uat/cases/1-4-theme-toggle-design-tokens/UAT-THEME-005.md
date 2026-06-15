---
用例编号: UAT-THEME-005
测试模块: 无障碍-对比度
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 运行中的应用
      ref: app-running
      state: { running: true, has_at_least_one_role: true }
      auto_generatable: true
      requirement:
    - type: 对比度检测工具
      ref: axe-devtools
      state: { available: true }
      auto_generatable: false
      requirement: 需要 axe DevTools（或等效对比度检测工具）来客观测量文本/背景对比度。
  isolation: read-only
  notes: 仅检测渲染结果，不改动数据。两种主题各测一遍。
---

# UAT-THEME-005 浅色与深色主题文本对比度均达标

## 业务场景
无论用户选浅色还是深色，界面文字都应清晰可读。视力较弱的用户尤其依赖足够的文本对比度。

## 前置条件
- 应用已启动，至少有一个角色，可进入主要页面。
- 已准备对比度检测工具。

## 测试步骤
1. 在浅色主题下，用对比度工具检测管家视角、角色视图、侧边栏上下文菜单的文本/背景对比度。
2. 切换到深色主题，重复检测相同页面。
3. 记录任何低于阈值的组合。

## 预期结果
- 浅色与深色主题下，所有关键文本/背景对比度 ≥ 4.5:1。
- 深色主题下状态小点、上下文菜单等无异常高亮或不可读区域。

## 实际结果
[Blocked] 缺少前置物料：未提供「axe DevTools 或等效对比度检测工具」（数据需求表 B-7），无法客观测量浅色/深色主题文本对比度是否 ≥ 4.5:1。主观目测不足以作为无障碍合规判定依据。

## 测试结论
Blocked（阻塞原因：环境物料缺失 B-7 对比度检测工具；解除后转人工测量执行）
