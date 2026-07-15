---
用例编号: UAT-NOTIFY-002
测试模块: 三级通知
story_key: 4-5-three-tier-notification
version_anchor: 7a9d0e9
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
    - type: 角色通知
      ref: 一条轻触级通知
      state: { level: tap }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证轻触级仅红点提示。
---

# UAT-NOTIFY-002 轻触级通知仅显示铃铛红点不弹窗

## 业务场景
角色生成"轻触"级通知（中等），希望侧边栏通知铃铛显示红点提示有新消息，但不弹窗打断当前操作，用户有空再去看。

## 前置条件
- 存在一条 tap 级通知

## 测试步骤
1. 触发一条轻触级通知生成
2. 观察侧边栏铃铛
3. 确认无弹窗、无 ActionCard 嵌入对话区

## 预期结果
- 侧边栏通知铃铛显示红点 badge
- 不弹窗、不嵌入 ActionCard 到管家对话区
- 不打断用户当前操作
- 红点 badge 有 aria-label="有新通知" 无障碍标签

## 实际结果
<留空>

## 测试结论
<留空>
