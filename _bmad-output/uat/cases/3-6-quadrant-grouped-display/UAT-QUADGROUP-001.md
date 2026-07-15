---
用例编号: UAT-QUADGROUP-001
测试模块: 四象限分组显示
story_key: 3-6-quadrant-grouped-display
version_anchor: 068c84b
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 分布在 Q1~Q4 的多条任务
      state: { Q1: 2, Q2: 3, Q3: 1, Q4: 0 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察分组渲染，不修改数据。
---

# UAT-QUADGROUP-001 任务按四象限分组显示且标题配色区分

## 业务场景
用户打开任务面板，希望一眼看清哪些紧急、哪些重要——四象限分组按固定顺序展示，每组标题颜色不同（Q1 红、Q2 蓝、Q3 灰、Q4 淡灰），帮自己快速定位。

## 前置条件
- 某角色下有分布在多个象限的任务

## 测试步骤
1. 进入角色工作台任务Tab
2. 观察任务列表的分组结构

## 预期结果
- 固定渲染 4 个象限分组，顺序恒为 Q1 → Q2 → Q3 → Q4
- 标题文案：Q1·重要且紧急 / Q2·重要不紧急 / Q3·紧急不重要 / Q4·不重要不紧急
- 标题颜色：Q1 红色、Q2 蓝色、Q3 深灰、Q4 淡灰
- 即使某象限为空也渲染（不隐藏）

## 实际结果
<留空>

## 测试结论
<留空>
