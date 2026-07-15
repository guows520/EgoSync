---
用例编号: UAT-QUADGROUP-003
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
      ref: 某些象限为空
      state: { Q1: 0, Q2: 2, Q3: 0, Q4: 0 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证空象限鼓励文案。
---

# UAT-QUADGROUP-003 空象限显示鼓励文案而非空白

## 业务场景
用户某个象限没有任务，希望看到一句温和的鼓励（如 Q1 空："没有紧急任务，太棒了！"），而不是一片空白让自己觉得遗漏了什么。

## 前置条件
- 某角色至少一个象限无任务，但并非全部为空

## 测试步骤
1. 打开任务面板
2. 观察空象限分组的内容

## 预期结果
- 空象限只渲染标题（含配色 + (0) badge）+ 一段鼓励文案，不渲染任务列表区
- 鼓励文案按象限区分：
  - Q1：没有紧急任务，太棒了！
  - Q2：暂无重要规划，别忘了为长远目标留出时间
  - Q3：没有需要应付的杂事，很清爽
  - Q4：没有可有可无的任务，注意力很集中
- 文案为弱化样式，不制造压力

## 实际结果
<留空>

## 测试结论
<留空>
