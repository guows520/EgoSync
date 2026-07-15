---
用例编号: UAT-PERF-001
测试模块: 性能基准
story_key: 8-4-performance-benchmark
version_anchor: 4f44008
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证冷启动时间。
---

# UAT-PERF-001 应用冷启动时间在可接受范围

## 业务场景
用户希望应用冷启动够快——从双击图标到能交互不超过几秒，不用等半天才看到界面，体验流畅。

## 前置条件
- 应用已安装

## 测试步骤
1. 完全关闭应用
2. 双击图标启动
3. 计时从启动到 Onboarding/主界面可交互
4. 重复 3 次取平均

## 预期结果
- 本地 SSD 冷启动 ≤ 3 秒
- CI 环境冷启动 ≤ 10 秒（runner 性能波动大，宽松基线）
- 启动过程无卡死
- 性能报告记录启动时间供趋势分析

## 实际结果
<留空>

## 测试结论
<留空>
