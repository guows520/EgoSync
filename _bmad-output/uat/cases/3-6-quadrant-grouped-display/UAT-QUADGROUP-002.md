---
用例编号: UAT-QUADGROUP-002
测试模块: 四象限分组显示
story_key: 3-6-quadrant-grouped-display
version_anchor: 068c84b
exec_mode: auto
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
      ref: 分布在 Q1~Q4 的任务
      state: { Q1: 2, Q2: 3, Q3: 1, Q4: 0 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证计数 badge 的显示。
---

# UAT-QUADGROUP-002 每个象限标题右侧显示任务数 badge

## 业务场景
用户希望每个象限标题旁有个小数字，告诉自己这个象限有多少任务，空象限显示 0，不用自己数。

## 前置条件
- 某角色下有任务，至少一个象限为空

## 测试步骤
1. 打开任务面板
2. 观察每个象限标题右侧的计数 badge

## 预期结果
- 每个象限标题右侧显示任务数 badge（如"重要且紧急 (2)"）
- 数值 = 该象限全部未删除任务数（未完成 + 已完成）
- 空象限同样显示 badge，值为 0
- badge 为中性药丸样式，不抢视觉焦点

## 实际结果
<留空>

## 测试结论
<留空>
