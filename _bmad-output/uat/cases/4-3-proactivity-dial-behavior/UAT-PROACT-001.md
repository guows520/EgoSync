---
用例编号: UAT-PROACT-001
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
      ref: 角色设为 moderate
      state: { proactivity: moderate }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证 moderate 过滤 low 建议。
---

# UAT-PROACT-001 moderate 档过滤低优先级建议

## 业务场景
用户把角色设为 moderate（适度），希望只收到 high 和 medium 优先级的建议，low 优先级的琐碎建议被自动过滤掉，不打扰自己。

## 前置条件
- 角色设为 moderate
- 默认 LLM 可用

## 测试步骤
1. 确认角色为 moderate
2. 触发工作循环生成建议
3. 检查写入 suggestions 表的建议优先级

## 预期结果
- 仅 high 和 medium 优先级建议写入 DB
- low 优先级建议在写入前被过滤掉，不出现在 suggestions 表
- 通知级别上限为"轻触"（tap），不使用"敲门"（knock）

## 实际结果
<留空>

## 测试结论
<留空>
