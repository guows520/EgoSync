---
用例编号: UAT-THEME-006
测试模块: 浅色主题视觉零回归
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: Story1.3 完成态基线截图
      ref: story13-baseline
      state: { theme: light, views: ["管家视角","角色视图","全局设置"] }
      auto_generatable: false
      requirement: 需要 Story 1.3 完成后（引入 design token 之前）的浅色主题页面截图，作为引入 CSS 变量后视觉零回归的对比基准。
    - type: 运行中的应用
      ref: app-running
      state: { running: true, theme: light }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅视觉对比，不改动数据。
---

# UAT-THEME-006 引入设计 token 后浅色主题外观保持不变

## 业务场景
本次引入 CSS 变量设计 token 系统，但浅色主题是当前用户已习惯的外观。用户期望升级后浅色界面与之前完全一样，token 化是"幕后"改动。

## 前置条件
- 已有 Story 1.3 完成态的浅色主题基线截图。
- 应用以浅色主题启动。

## 测试步骤
1. 以浅色主题打开应用，进入管家视角、角色视图、全局设置。
2. 截取相同区域截图。
3. 与 Story 1.3 基线逐一对比配色、间距、圆角、字体。

## 预期结果
- 浅色主题下界面与 Story 1.3 基线视觉一致，无配色漂移、无圆角/间距变化。
- accent、sidebar 等颜色与原硬编码值保持一致。

## 实际结果
[Blocked] 缺少前置物料：未提供「Story 1.3 完成态（token 化前）浅色主题基线截图」（数据需求表 B-4）。无基线无法做引入设计 token 后的视觉零回归像素对比。

## 测试结论
Blocked（阻塞原因：基线物料缺失 B-4 Story1.3 浅色截图；解除后转人工对比执行）
