---
用例编号: UAT-DASHBOARD-003
测试模块: 管家仪表盘
story_key: 4-7-dashboard-real-role-status
version_anchor: 0139d7a5cea8f5df1a202b80f7ba0f5857658226
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证点击卡片跳转。
---

# UAT-DASHBOARD-003 点击角色卡片切换到该角色视图

## 业务场景
用户在仪表盘看到某角色需要关注，希望直接点击它的卡片就跳转到该角色视图，不用再回侧边栏找，操作连贯。

## 前置条件
- 仪表盘有角色卡片

## 测试步骤
1. 打开仪表盘
2. 点击某张角色卡片
3. 观察视图切换

## 预期结果
- 点击后切换到该角色视图
- 复用 Story 2.2 的导航逻辑
- 视图切换流畅，不报错

## 实际结果
<留空>

## 测试结论
<留空>
