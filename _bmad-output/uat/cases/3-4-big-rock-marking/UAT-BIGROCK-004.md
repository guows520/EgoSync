---
用例编号: UAT-BIGROCK-004
测试模块: 大石头标记
story_key: 3-4-big-rock-marking
version_anchor: c4248c6
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少两个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 角色A已有 3 个大石头
      state: { 归属: 角色A, 大石头数: 3 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 角色B的一条待标记任务
      state: { 归属: 角色B, is_big_rock: false }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证角色间大石头数量互不影响。
---

# UAT-BIGROCK-004 不同角色的大石头数量互不影响

## 业务场景
用户在"读书人"角色已标记 3 个大石头，希望在"健身教练"角色仍能标记自己的大石头，两个角色的限额独立计算，不互相挤占。

## 前置条件
- 至少两个 active 角色
- 角色A已有 3 个大石头任务

## 测试步骤
1. 切换到角色B
2. 在角色B下打开一条任务，勾选大石头
3. 保存

## 预期结果
- 角色B的任务成功标记为大石头，不报错
- 角色A的 3 个大石头限制不影响角色B
- 各角色大石头数量独立计数

## 实际结果
<留空>

## 测试结论
<留空>
