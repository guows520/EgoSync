---
用例编号: UAT-UI-003
测试模块: 弹窗交互保留
story_key: 1-3-component-domain-split-visual-zero-regression
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 运行中的应用
      ref: app-after-split
      state: { running: true, has_at_least_one_role: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅打开/关闭弹窗交互，不持久化改动数据。
---

# UAT-UI-003 移除演示条后各弹窗仍可正常打开与关闭

## 业务场景
移除 Pitch Mode 后，仲裁弹窗、周复盘弹窗等不再由演示条自动触发，但它们仍是产品功能。用户期望通过正常入口能打开这些弹窗并关闭，功能未因重构而丢失。

## 前置条件
- 应用已启动，至少存在一个角色。

## 测试步骤
1. 进入管家视角，依次打开：全局设置面板、新建任务弹窗、添加角色弹窗、通知面板。
2. 打开后点击关闭按钮或遮罩，确认能正常关闭。
3. 进入角色视图，验证角色相关面板（任务/记忆/设置 Tab）切换正常。

## 预期结果
- 各弹窗均能正常打开并关闭，内容完整无报错。
- 角色视图内 Tab 切换正常，无空白或崩溃。

## 实际结果

## 测试结论
