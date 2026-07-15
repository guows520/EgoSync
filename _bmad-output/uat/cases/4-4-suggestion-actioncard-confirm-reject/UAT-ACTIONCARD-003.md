---
用例编号: UAT-ACTIONCARD-003
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
  notes: 验证拒绝原因记录与反馈注入。
---

# UAT-ACTIONCARD-003 拒绝建议并选择原因后减少类似建议

## 业务场景
用户觉得某条建议不合适，点"拒绝"并选择原因（不相关/时机不对/已完成/其他），希望系统记录原因，下次生成建议时减少类似建议，避免反复被打扰。

## 前置条件
- 至少一条 pending 建议

## 测试步骤
1. 点击建议卡片的"拒绝"按钮
2. 在弹出的原因选择中选择"时机不对"
3. 确认拒绝
4. 观察卡片消失
5. 等待下一轮建议生成，观察是否减少类似建议

## 预期结果
- 弹出拒绝原因选择（不相关/时机不对/已完成/其他）
- suggestions.status 更新为 rejected，rejection_reason 写入
- 卡片消失
- 角色下次生成建议时 System Prompt 注入"用户曾拒绝以下类型建议"
- 减少类似建议的生成频率

## 实际结果
<留空>

## 测试结论
<留空>
