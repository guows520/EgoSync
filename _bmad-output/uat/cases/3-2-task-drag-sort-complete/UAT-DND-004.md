---
用例编号: UAT-DND-004
测试模块: 任务完成
story_key: 3-2-task-drag-sort-complete
version_anchor: 5585768dcab685f56ddec0d45c4d2a0d0a3bd0f2
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条已完成任务 task_done
      state: { 已完成: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证幂等语义：重复点完成不刷新 completed_at。
---

# UAT-DND-004 重复点击完成不刷新首次完成时刻

## 业务场景
用户已完成的任务再点一次完成，希望系统识别"已经是完成态"不做任何改动，保护"首次完成时刻"的语义，不让它被刷新成最新时间。

## 前置条件
- 某角色下有至少一条已完成任务，记录其 completed_at

## 测试步骤
1. 记录该已完成任务的 completed_at 时间戳
2. 再次对该任务触发"标记完成"操作（已是完成态）
3. 检查 completed_at 是否变化

## 预期结果
- 后端识别当前 is_completed 已等于目标值，直接返回当前任务
- completed_at 与 updated_at 不被刷新
- "首次完成时刻"语义得到保护

## 实际结果
<留空>

## 测试结论
<留空>
