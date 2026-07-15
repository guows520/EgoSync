---
用例编号: UAT-REVIEWMODAL-001
测试模块: 周复盘Modal
story_key: 6-5-weekly-review-modal-real-data
version_anchor: da49e65a2f3c078fe0ccd355b82d0519f8cbe8ae
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有本周复盘记录的 EgoSync
      state: { 已安装: true, 有本周复盘: true }
      auto_generatable: true
      requirement: false
    - type: 周复盘记录
      ref: 本周已生成的 weekly_reviews 记录
      state: { 已存在: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察 Modal 展示。
---

# UAT-REVIEWMODAL-001 周复盘Modal展示真实复盘数据

## 业务场景
用户在可视化界面中回顾本周，希望看到真实复盘摘要、能量趋势柱状图、大石头完成列表，而不是 mock 假数据，让自己有仪式感地完成每周节奏闭环。

## 前置条件
- 本周已生成 weekly_reviews 记录

## 测试步骤
1. 打开 WeeklyReviewModal（复盘阶段）
2. 观察复盘摘要、能量趋势、大石头完成列表
3. 检查日期范围显示

## 预期结果
- 复盘摘要从后端 weekly_reviews 加载真实内容
- 能量趋势柱状图（SVG 自绘）展示各角色能量值
- 大石头完成列表展示真实完成情况
- 日期范围自动计算本周一到本周日，格式"YYYY年M月D日 - D日"
- 不显示 mock 假数据

## 实际结果
<留空>

## 测试结论
<留空>
