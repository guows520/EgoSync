---
用例编号: UAT-BRIEF-001
测试模块: 晨间简报
story_key: 6-1-daily-morning-briefing
version_anchor: 0ec5563a43b677de604bc4dab7567b857afce626
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
      requirement: 需用户提供有效 LLM Provider，否则简报生成降级
  isolation: write-isolated
  notes: 简报写入 briefings 表与管家对话，验证后可清理。
---

# UAT-BRIEF-001 每日按配置时间生成晨间简报

## 业务场景
用户每天打开应用，希望管家已经为自己汇总好今日概览——各角色状态、今天最重要的事、待处理建议，以自然语言段落呈现，不用自己到处翻找。

## 前置条件
- 默认 LLM 可用
- 有 active 角色与任务

## 测试步骤
1. 确认晨间简报时间配置（默认 08:00）
2. 等待到配置时间（或手动触发生成）
3. 打开管家对话区观察简报

## 预期结果
- 按配置时间自动生成自然语言段落简报
- 简报内容含各角色状态、今日最重要的事、待处理建议
- 简报以管家对话消息形式呈现在 ButlerView 对话区
- 写入 briefings 表，重启后可查
- 每日只生成一次（去重）

## 实际结果
<留空>

## 测试结论
<留空>
