---
用例编号: UAT-SUGGEST-004
测试模块: 主动建议生成
story_key: 4-2-proactive-suggestion-generation
version_anchor: c83d6dd
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置但默认 LLM 不可用的 EgoSync
      state: { 已安装: true, 默认LLM可用: false }
      auto_generatable: true
      requirement: false
    - type: 角色配置
      ref: 有目标与任务的角色
      state: { 有目标: true, 有任务: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证 LLM 失败降级。
---

# UAT-SUGGEST-004 LLM 生成失败时不报错不写半成品

## 业务场景
LLM 暂时不可用（超时/格式错误/provider 错误），希望系统安静地跳过本轮建议生成，不向用户报错，不写入半成品建议，等待下次循环再试。

## 前置条件
- 默认 LLM 不可用或返回格式错误

## 测试步骤
1. 制造 LLM 失败场景（断网或配置错误）
2. 触发工作循环
3. 检查 suggestions 表与应用状态

## 预期结果
- tracing::warn! 记录失败原因（含 role_id、role_name、error）
- 不向用户报错，不 panic
- 工作循环返回 Ok(())，等待下次循环
- 不写入任何半成品建议

## 实际结果
<留空>

## 测试结论
<留空>
