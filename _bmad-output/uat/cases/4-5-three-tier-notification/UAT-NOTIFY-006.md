---
用例编号: UAT-NOTIFY-006
测试模块: 三级通知
story_key: 4-5-three-tier-notification
version_anchor: 7a9d0e9
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
    - type: 角色配置
      ref: 角色设为 moderate
      state: { proactivity: moderate }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证主动性档位约束通知级别上限。
---

# UAT-NOTIFY-006 通知级别受角色主动性档位约束

## 业务场景
moderate 档角色最高只能发"轻触"级通知，即使内部逻辑想发"敲门"也会被降级到"轻触"；proactive 档才能发"敲门"。希望系统自动按档位约束，不用用户每次手动判断。

## 前置条件
- 角色设为 moderate

## 测试步骤
1. 确认角色为 moderate
2. 触发一条理论上应为 knock 级的通知
3. 检查实际写入的通知级别

## 预期结果
- moderate 档最高通知级别为 tap
- 超出上限的 knock 自动降级为 tap
- proactive 档可发 knock
- passive 档不发通知
- 降级行为复用 max_notification_level_for_proactivity 函数

## 实际结果
<留空>

## 测试结论
<留空>
