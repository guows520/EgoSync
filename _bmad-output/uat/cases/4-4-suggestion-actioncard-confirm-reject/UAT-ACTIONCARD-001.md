---
用例编号: UAT-ACTIONCARD-001
测试模块: 建议卡片
story_key: 4-4-suggestion-actioncard-confirm-reject
version_anchor: 1d8249ee427fd185c4828fc56d6f11dd23ab096a
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
    - type: 角色建议
      ref: 至少一条 pending 状态建议
      state: { status: pending }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察卡片展示。
---

# UAT-ACTIONCARD-001 管家对话区展示待确认建议卡片

## 业务场景
角色生成了待确认的建议，用户希望打开管家视角就能看到这些待处理建议以卡片形式展示在对话区，包含图标、标题、来源角色、时间，一目了然知道自己该处理哪些。

## 前置条件
- 至少有一条 pending 状态的建议

## 测试步骤
1. 切换到管家视角
2. 观察管家对话区
3. 查看建议卡片内容

## 预期结果
- 管家对话区嵌入 ActionCard 展示 pending 建议
- 每张卡片含图标 + 标题 + 来源角色 + 时间
- 卡片有"确认"和"拒绝"按钮
- 已处理（confirmed/rejected）的建议不再显示

## 实际结果
<留空>

## 测试结论
<留空>
