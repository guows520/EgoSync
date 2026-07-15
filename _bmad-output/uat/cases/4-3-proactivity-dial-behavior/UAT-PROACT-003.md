---
用例编号: UAT-PROACT-003
测试模块: 主动性档位
story_key: 4-3-proactivity-dial-behavior
version_anchor: c9106649c6fe0a98c34d2a68b26c0162af30dff6
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
    - type: 角色配置
      ref: 角色设为 passive
      state: { proactivity: passive }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证 passive 静默。
---

# UAT-PROACT-003 passive 档角色完全静默不主动打扰

## 业务场景
用户希望某个角色完全安静，只在自己主动找它对话时才响应，不主动生成建议、不主动发通知，给自己最大专注空间。

## 前置条件
- 角色设为 passive

## 测试步骤
1. 确认角色为 passive
2. 等待多轮调度 tick
3. 检查 suggestions 表与通知

## 预期结果
- 调度器跳过该角色，不 spawn 工作循环
- 不生成任何建议
- 不发送任何通知
- 仅在用户主动对话时响应

## 实际结果
<留空>

## 测试结论
<留空>
