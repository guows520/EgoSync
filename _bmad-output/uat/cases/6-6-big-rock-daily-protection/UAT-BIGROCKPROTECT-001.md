---
用例编号: UAT-BIGROCKPROTECT-001
测试模块: 大石头保护
story_key: 6-6-big-rock-daily-protection
version_anchor: f20301b58d138f47419e57488a7f11a6c04338de
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
    - type: 角色任务
      ref: 一条未完成大石头任务，本周无进展（updated_at 早于本周一）
      state: { is_big_rock: true, is_completed: false, 本周无进展: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后清理提醒记录。
---

# UAT-BIGROCKPROTECT-001 工作日大石头无进展时管家温和提醒

## 业务场景
用户本周标记了大石头但一直没动它，希望管家在工作日温和提醒"你的大石头'XX'这周还没动，要不要今天安排一点时间？"，帮自己守住本周最重要的事，不被琐事淹没。

## 前置条件
- 存在未完成大石头任务，本周无任何进展（updated_at 早于本周一）

## 测试步骤
1. 确认有大石头任务本周未动
2. 等待调度器 tick 触发大石头保护检查
3. 观察管家对话区

## 预期结果
- 管家在对话中温和提醒，包含大石头标题
- 提醒文案如"你的大石头'XX'这周还没动，要不要今天安排一点时间？"
- 生成"轻触"级通知
- 不使用 toast/红框打断

## 实际结果
<留空>

## 测试结论
<留空>
