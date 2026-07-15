---
用例编号: UAT-SCHEDCFG-003
测试模块: 节奏化时间配置
story_key: 6-2-schedule-config-briefing-review
version_anchor: 1d51afb
exec_mode: auto
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证大石头补触发星期可选范围。
---

# UAT-SCHEDCFG-003 大石头规划提醒星期仅限周一或周二

## 业务场景
大石头规划提醒只在每周初触发，希望星期选择限定在周一或周二，避免用户选了周五失去"周初规划"的意义。

## 前置条件
- 管家设置可访问

## 测试步骤
1. 打开大石头规划提醒配置
2. 查看星期可选项
3. 尝试选择周一或周二

## 预期结果
- 星期可选仅限周一或周二
- 选择其他星期被校验拒绝
- 保存后 bigrock_reminder_day 为 "1" 或 "2"

## 实际结果
<留空>

## 测试结论
<留空>
