---
用例编号: UAT-BIGROCKPLAN-002
测试模块: 大石头规划提醒
story_key: 6-3-big-rock-planning-reminder
version_anchor: d7c489b
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 本周已有大石头任务
      state: { 本周大石头数: 2 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证已有大石头时不提醒。
---

# UAT-BIGROCKPLAN-002 已规划大石头时不触发提醒

## 业务场景
用户已经在周复盘中规划了本周大石头，希望系统识别"已规划"不再提醒，避免重复打扰。

## 前置条件
- 本周已有 is_big_rock=true 的任务

## 测试步骤
1. 确认本周已有大石头任务
2. 到达配置的提醒时间
3. 检查是否触发提醒

## 预期结果
- 不触发大石头规划提醒
- 不生成通知、不打开 Modal
- 系统识别本周已有大石头，安静跳过

## 实际结果
<留空>

## 测试结论
<留空>
