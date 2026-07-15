---
用例编号: UAT-ENERGY-004
测试模块: 能量值引擎
story_key: 4-8-energy-calculation-engine
version_anchor: 92a316e7f554017b93e41e54462e9e59c95c391d
exec_mode: auto
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证不实时重算。
---

# UAT-ENERGY-004 用户手动操作后不实时重算能量值

## 业务场景
用户完成任务或对话后，希望能量值不立即重算（避免频繁计算拖慢响应），等下次调度循环再统一更新，V1 接受这种延迟。

## 前置条件
- 角色有任务

## 测试步骤
1. 记录角色当前 energy
2. 完成一条任务或发送一条对话
3. 立即检查 energy 是否变化
4. 等待下次调度循环，再检查 energy

## 预期结果
- 手动操作后 energy 不立即重算
- 等下次调度循环才更新
- V1 不追求实时性，允许延迟
- 用户操作响应不被能量计算拖慢

## 实际结果
<留空>

## 测试结论
<留空>
