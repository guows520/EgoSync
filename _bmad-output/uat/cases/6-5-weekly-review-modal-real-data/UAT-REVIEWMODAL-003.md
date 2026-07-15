---
用例编号: UAT-REVIEWMODAL-003
测试模块: 周复盘Modal
story_key: 6-5-weekly-review-modal-real-data
version_anchor: da49e65a2f3c078fe0ccd355b82d0519f8cbe8ae
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
  isolation: read-only
  notes: 验证阶段切换。
---

# UAT-REVIEWMODAL-003 复盘与规划两阶段可切换

## 业务场景
用户希望周复盘 Modal 支持从复盘阶段切换到规划阶段，先回顾本周再规划下周，有仪式感地完成闭环。

## 前置条件
- WeeklyReviewModal 可打开

## 测试步骤
1. 打开 Modal，默认进入复盘阶段
2. 查看复盘内容
3. 切换到规划阶段
4. 规划后切回复盘查看

## 预期结果
- 默认进入复盘阶段（除非 initialPhase='plan'）
- 可切换到规划阶段
- 阶段切换流畅，数据不丢失
- 大石头规划提醒触发时自动进入规划阶段

## 实际结果
<留空>

## 测试结论
<留空>
