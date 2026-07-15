---
用例编号: UAT-OVERVIEW-003
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
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
    - type: 角色任务
      ref: 跨多个象限与大石头的任务
      state: { 含大石头: true, 跨象限: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证象限与大石头筛选的服务端过滤。
---

# UAT-OVERVIEW-003 按象限与仅大石头筛选任务概览

## 业务场景
用户想聚焦"重要且紧急"或"只看大石头"，希望用象限 chips 和"仅大石头"开关快速过滤，概览只显示符合条件的任务。

## 前置条件
- 概览中有跨多个象限、含大石头与非大石头的任务

## 测试步骤
1. 打开任务概览
2. 点击象限 chips 选择"Q1"
3. 观察概览变化
4. 切回"全部"
5. 打开"仅大石头"开关
6. 观察概览变化

## 预期结果
- 选 Q1 后只显示 Q1 任务（服务端过滤）
- 切回全部后恢复所有象限
- 打开"仅大石头"后只显示大石头任务
- 象限 + 仅大石头可叠加生效

## 实际结果
<留空>

## 测试结论
<留空>
