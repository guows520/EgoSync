---
用例编号: UAT-QUADGROUP-005
测试模块: 四象限分组显示
story_key: 3-6-quadrant-grouped-display
version_anchor: 068c84b
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
      ref: 该角色任务总数为 0
      state: { 任务数: 0 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证全局空态优先于象限鼓励文案。
---

# UAT-QUADGROUP-005 完全没有任务时只显示全局空态文案

## 业务场景
用户某角色一条任务都没有，希望看到一句温和的引导（如"还没有任务，先添加一个小目标吧"），而不是 4 个空象限堆叠出一堆"没有紧急任务/没有重要规划"的鼓励文案，那样反而显得啰嗦。

## 前置条件
- 某角色任务总数为 0

## 测试步骤
1. 切换到一个没有任何任务的角色
2. 打开任务面板
3. 观察显示内容

## 预期结果
- 只显示全局空态文案"还没有任务，先添加一个小目标吧"
- 不渲染 4 个象限分组
- 不渲染各象限鼓励文案

## 实际结果
<留空>

## 测试结论
<留空>
