---
用例编号: UAT-BRIEF-002
测试模块: 晨间简报
story_key: 6-1-daily-morning-briefing
version_anchor: 0ec5563a43b677de604bc4dab7567b857afce626
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
  isolation: read-only
  notes: 验证简报基于真实数据。
---

# UAT-BRIEF-002 简报内容基于真实角色状态与任务

## 业务场景
用户希望简报不是套话，而是基于自己真实的角色状态、任务进度、待处理建议生成，让自己看了知道今天该聚焦什么。

## 前置条件
- 角色有任务、有能量值、有 pending 建议

## 测试步骤
1. 记录当前各角色状态、任务、pending 建议
2. 触发简报生成
3. 阅读简报内容，对照真实数据

## 预期结果
- 简报内容可追溯到真实角色状态（能量值、待办数）
- 提及今天最重要的事（如临期任务、大石头）
- 提及待处理建议
- 不凭空捏造不存在的内容

## 实际结果
<留空>

## 测试结论
<留空>
