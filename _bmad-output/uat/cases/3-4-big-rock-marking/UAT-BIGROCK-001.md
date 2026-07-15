---
用例编号: UAT-BIGROCK-001
测试模块: 大石头标记
story_key: 3-4-big-rock-marking
version_anchor: c4248c6
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条未标记大石头的任务
      state: { is_big_rock: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后可取消标记。
---

# UAT-BIGROCK-001 标记任务为大石头并视觉突出

## 业务场景
用户每周有几件不可妥协的核心任务，希望明确标记为"大石头"，让系统和自己都能一眼识别这些是最重要的事，并在列表中排在前面。

## 前置条件
- 某角色下已有至少一条任务，且该角色当前大石头数量 < 3

## 测试步骤
1. 打开某条任务的编辑表单
2. 勾选"标记为本周大石头"复选框
3. 保存任务
4. 观察任务列表中该任务的视觉与位置

## 预期结果
- 保存后任务卡片显示琥珀色"大石头"标签
- 该任务在同象限内排在非大石头任务之前
- 标签样式温和（bg-amber-50 text-amber-600 border-amber-100）

## 实际结果
<留空>

## 测试结论
<留空>
