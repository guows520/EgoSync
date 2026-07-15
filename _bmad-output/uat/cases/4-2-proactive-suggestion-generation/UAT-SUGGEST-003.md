---
用例编号: UAT-SUGGEST-003
测试模块: 主动建议生成
story_key: 4-2-proactive-suggestion-generation
version_anchor: c83d6dd
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色配置
      ref: 一个无目标、无任务、无近期记忆的角色
      state: { 有目标: false, 有任务: false, 有记忆: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证空输出允许。
---

# UAT-SUGGEST-003 角色无目标无任务时不生成建议且不报错

## 业务场景
用户刚建了一个新角色但还没填目标和任务，希望系统识别"这个角色没什么可建议的"，安静地跳过，不报错也不生成无意义的建议。

## 前置条件
- 存在一个无目标、无任务、无近期记忆的角色

## 测试步骤
1. 确认该角色无任何上下文
2. 触发工作循环
3. 检查 suggestions 表与日志

## 预期结果
- 不生成任何建议（0 条输出）
- 工作循环正常返回 Ok(())，不报错
- tracing::info! 记录"本次无建议生成"
- 不写入任何半成品建议

## 实际结果
<留空>

## 测试结论
<留空>
