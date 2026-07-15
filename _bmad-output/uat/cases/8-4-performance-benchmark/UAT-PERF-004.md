---
用例编号: UAT-PERF-004
测试模块: 性能基准
story_key: 8-4-performance-benchmark
version_anchor: 4f44008
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 全新安装的 EgoSync
      state: { 已安装: true, 未完成onboarding: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证首次体验时间。
---

# UAT-PERF-004 新用户首次体验时间≤5分钟

## 业务场景
新用户首次打开应用，希望从打开到创建第一个角色不超过 5 分钟，Onboarding 流程不超过 4 步，快速上手不劝退。

## 前置条件
- 全新安装，未完成 onboarding

## 测试步骤
1. 启动应用
2. 完成 Onboarding 流程
3. 创建第一个角色
4. 计时全程

## 预期结果
- Onboarding 流程 ≤ 4 步
- 从打开到创建第一个角色 ≤ 5 分钟
- 流程清晰不卡壳
- 性能报告记录首次体验时间

## 实际结果
<留空>

## 测试结论
<留空>
