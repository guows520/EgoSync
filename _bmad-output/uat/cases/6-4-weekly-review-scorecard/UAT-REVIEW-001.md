---
用例编号: UAT-REVIEW-001
测试模块: 周复盘成绩单
story_key: 6-4-weekly-review-scorecard
version_anchor: fbbc751
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
      requirement: 需用户提供有效 LLM Provider，否则复盘降级
    - type: 角色任务
      ref: 本周有完成的大石头与任务
      state: { 本周有完成: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 复盘写入 weekly_reviews 表，验证后可清理。
---

# UAT-REVIEW-001 周末自动生成正向叙事周复盘成绩单

## 业务场景
用户希望每周末收到一份温暖的成绩单，回顾本周进展——各角色能量值、大石头完成情况、新沉淀的记忆、新启用的 Skill，用正向叙事让自己感受到进步的满足感，为下周做准备。

## 前置条件
- 默认 LLM 可用
- 本周有完成的任务与活动

## 测试步骤
1. 确认周复盘时间配置（默认周日 20:00）
2. 等待到配置时间（或手动触发）
3. 打开管家对话区观察复盘

## 预期结果
- 按配置时间自动生成正向叙事的复盘摘要
- 内容含各角色能量值、大石头完成情况、新记忆条数、新启用 Skill
- 叙事风格温暖正向（非冷冰冰的数据罗列）
- 写入 weekly_reviews 表 + 管家对话消息
- 发送"轻触"级通知
- 每周只生成一次（去重）

## 实际结果
<留空>

## 测试结论
<留空>
