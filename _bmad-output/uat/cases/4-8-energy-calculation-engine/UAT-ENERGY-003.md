---
用例编号: UAT-ENERGY-003
测试模块: 能量值引擎
story_key: 4-8-energy-calculation-engine
version_anchor: 92a316e7f554017b93e41e54462e9e59c95c391d
exec_mode: semi
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
      ref: 角色当前能量 ≥ 40，制造场景使其降至 < 40
      state: { 当前能量: 45 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证低能量通知。
---

# UAT-ENERGY-003 能量值跨阈值降至 40 以下时发轻触通知

## 业务场景
用户某角色能量值一直还行，这次计算突然掉到 40% 以下，希望系统发一条"轻触"级通知提醒"你的XX角色能量值较低，可能需要关注"，让自己及时介入。

## 前置条件
- 角色当前能量 ≥ 40

## 测试步骤
1. 制造场景使该角色能量值降至 < 40（如增加 at_risk 任务、减少完成率）
2. 触发能量计算
3. 检查通知

## 预期结果
- 能量值从 ≥ 40 降至 < 40（跨越阈值）
- 生成"轻触"级通知"你的XX角色能量值较低，可能需要关注"
- 侧边栏铃铛显示红点
- 不重复触发（仅在跨阈值时发，而非每次计算都发）

## 实际结果
<留空>

## 测试结论
<留空>
