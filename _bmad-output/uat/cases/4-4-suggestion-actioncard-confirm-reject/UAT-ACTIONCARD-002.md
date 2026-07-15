---
用例编号: UAT-ACTIONCARD-002
测试模块: 建议卡片
story_key: 4-4-suggestion-actioncard-confirm-reject
version_anchor: 1d8249ee427fd185c4828fc56d6f11dd23ab096a
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
    - type: 角色建议
      ref: 一条 pending 建议
      state: { status: pending }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 确认会创建任务，验证后可清理。
---

# UAT-ACTIONCARD-002 确认建议后自动创建对应角色任务

## 业务场景
用户看到一条有价值的建议，点"确认"希望系统自动把这条建议转成对应角色下的任务，不用自己再手动去那个角色视图新建，省事。

## 前置条件
- 至少一条 pending 建议

## 测试步骤
1. 在管家对话区找到一张建议卡片
2. 点击"确认"按钮
3. 观察卡片动画与状态变化
4. 切换到该建议来源的角色视图，检查任务列表

## 预期结果
- 卡片显示 ✓ 动画后消失
- suggestions.status 更新为 confirmed
- 自动在对应角色的 tasks 表创建新任务
- suggestions.converted_task_id 关联新任务 ID
- 切到该角色视图，任务列表出现新任务

## 实际结果
<留空>

## 测试结论
<留空>
