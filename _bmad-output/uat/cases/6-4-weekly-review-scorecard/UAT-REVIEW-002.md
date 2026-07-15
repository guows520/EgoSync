---
用例编号: UAT-REVIEW-002
测试模块: 周复盘成绩单
story_key: 6-4-weekly-review-scorecard
version_anchor: fbbc751
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
  notes: 验证复盘数据可追溯。
---

# UAT-REVIEW-002 复盘内容基于本周真实数据

## 业务场景
用户希望复盘成绩单不是套话，而是基于本周真实的能量值、大石头完成、记忆沉淀、Skill 启用数据，让自己看到实实在在的进步。

## 前置条件
- 本周有可统计的活动数据

## 测试步骤
1. 记录本周各角色能量值、大石头完成数、新记忆数、新 Skill
2. 触发复盘生成
3. 对照复盘内容与真实数据

## 预期结果
- 复盘内容可追溯到真实数据
- 大石头完成列表与实际完成一致
- 新记忆条数与实际沉淀一致
- 不凭空捏造不存在的进展

## 实际结果
<留空>

## 测试结论
<留空>
