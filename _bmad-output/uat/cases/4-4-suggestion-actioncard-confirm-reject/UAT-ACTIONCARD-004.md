---
用例编号: UAT-ACTIONCARD-004
测试模块: 建议卡片
story_key: 4-4-suggestion-actioncard-confirm-reject
version_anchor: 1d8249ee427fd185c4828fc56d6f11dd23ab096a
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
    - type: 角色建议
      ref: 已确认与已拒绝的建议各一条
      state: { 含confirmed: true, 含rejected: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证已处理建议不再显示。
---

# UAT-ACTIONCARD-004 已处理建议不再重复显示

## 业务场景
用户已经确认或拒绝过某条建议，希望它从管家对话区消失，不再反复出现，避免重复处理。

## 前置条件
- 存在已确认和已拒绝的建议各一条

## 测试步骤
1. 切换到管家视角
2. 观察管家对话区的建议卡片
3. 确认已处理建议不在列表中

## 预期结果
- confirmed 和 rejected 状态的建议不再显示在管家对话区
- 仅 pending 状态建议显示为 ActionCard
- 不会重复出现已处理建议

## 实际结果
<留空>

## 测试结论
<留空>
