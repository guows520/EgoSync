---
用例编号: UAT-SUGGEST-001
测试模块: 主动建议生成
story_key: 4-2-proactive-suggestion-generation
version_anchor: c83d6dd
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider，否则建议生成会降级为空输出
    - type: 角色配置
      ref: 角色有目标、有任务、有近期记忆
      state: { 有目标: true, 有任务: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 建议会写入 suggestions 表，验证后清理。
---

# UAT-SUGGEST-001 角色工作循环生成基于真实上下文的建议

## 业务场景
用户希望角色不只是被动应答，而是会主动审视自己的目标和任务状态，给出有价值的建议（如"你这周的大石头还没动，要不要今天安排一下？"），且建议内容能追溯到当前的真实情况，不是凭空捏造。

## 前置条件
- 默认大模型已配置
- 某角色有目标、有任务、有近期记忆
- 角色主动性设为 moderate 或 proactive

## 测试步骤
1. 确保角色有目标与任务
2. 等待调度器触发工作循环（或手动触发）
3. 检查 suggestions 表是否生成 pending 状态建议
4. 检查建议内容是否与角色目标/任务相关

## 预期结果
- 工作循环生成 0-3 条建议
- 每条建议写入 suggestions 表，status = pending，priority ∈ {high, medium, low}
- 建议内容可追溯到角色当前 goal、任务状态、最近记忆
- 不凭空捏造与角色无关的建议

## 实际结果
<留空>

## 测试结论
<留空>
