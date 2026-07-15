---
用例编号: UAT-BRIEF-003
测试模块: 晨间简报
story_key: 6-1-daily-morning-briefing
version_anchor: 0ec5563a43b677de604bc4dab7567b857afce626
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置但默认 LLM 不可用的 EgoSync
      state: { 已安装: true, 默认LLM可用: false }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证 LLM 失败降级。
---

# UAT-BRIEF-003 LLM 不可用时简报降级不报错

## 业务场景
LLM 暂时不可用，希望简报生成安静地跳过，不报错、不卡死应用，等下次再试，不影响其他功能。

## 前置条件
- 默认 LLM 不可用

## 测试步骤
1. 制造 LLM 不可用场景
2. 到达简报触发时间
3. 检查应用状态

## 预期结果
- 简报生成失败时 tracing::warn 记录原因
- 不向用户报错、不 panic
- 应用其他功能正常
- 不写入半成品简报

## 实际结果
<留空>

## 测试结论
<留空>
