---
用例编号: UAT-OVERVIEW-006
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
exec_mode: semi
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 所有角色与管家均无任务
      state: { 全空: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证空态文案。
---

# UAT-OVERVIEW-006 概览全空时显示温和引导文案

## 业务场景
所有角色和管家都没有任务，用户打开概览希望看到一句温和引导（如"所有角色都很轻松，可以考虑添加新目标"），而不是一片空白。

## 前置条件
- 所有 active 角色与管家均无任务

## 测试步骤
1. 切换到管家视角
2. 打开任务概览
3. 观察显示内容

## 预期结果
- 显示全空文案"所有角色都很轻松，可以考虑添加新目标"
- 不渲染空的象限分区
- 筛选后无结果时显示与全空不同的筛选空文案

## 实际结果
<留空>

## 测试结论
<留空>
