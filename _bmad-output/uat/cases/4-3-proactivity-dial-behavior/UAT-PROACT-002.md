---
用例编号: UAT-PROACT-002
测试模块: 主动性档位
story_key: 4-3-proactivity-dial-behavior
version_anchor: c9106649c6fe0a98c34d2a68b26c0162af30dff6
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
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 角色配置
      ref: 角色设为 proactive
      state: { proactivity: proactive }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证 proactive 保留全部优先级。
---

# UAT-PROACT-002 proactive 档保留所有优先级建议

## 业务场景
用户把角色设为 proactive（积极主动），希望收到所有优先级的建议（含 low），让角色尽可能多地主动提供帮助，high 优先级建议还可触发"敲门"通知。

## 前置条件
- 角色设为 proactive
- 默认 LLM 可用

## 测试步骤
1. 确认角色为 proactive
2. 触发工作循环生成建议
3. 检查写入 suggestions 表的建议优先级

## 预期结果
- high / medium / low 所有优先级建议均写入 DB
- high 优先级建议可触发"敲门"（knock）通知（实际通知行为在 4.5 实现）
- 不过滤任何优先级

## 实际结果
<留空>

## 测试结论
<留空>
