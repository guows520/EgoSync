---
用例编号: UAT-OVERVIEW-001
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色 + 管家自己有任务的 EgoSync
      state: { 已安装: true, active角色数: 2, 管家有任务: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 分布在多个角色与象限的任务
      state: { 跨owner: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察概览展示，不修改数据。
---

# UAT-OVERVIEW-001 管家任务概览汇总所有角色与管家任务

## 业务场景
用户在管家视角想一站式看到所有活跃角色 + 管家自己的任务，不用逐个切换角色就能掌握全局，每条任务就近标注归属，知道自己该为哪个角色操心。

## 前置条件
- 至少 2 个 active 角色各有任务
- 管家自己也有任务

## 测试步骤
1. 切换到管家视角
2. 打开"任务概览"Tab
3. 观察任务展示结构

## 预期结果
- Tab 标签为"任务概览"（不再是"通用任务"）
- 按 Q1→Q4 四象限分区展示，组内跨 owner 混排（不按角色分组）
- 每条任务卡就近显示归属：角色任务 = 角色色点 + 角色名；管家任务 = 固定"管家"标识
- 已完成任务默认折叠
- 排除归档角色的任务

## 实际结果
<留空>

## 测试结论
<留空>
