---
用例编号: UAT-REVIEWMODAL-002
测试模块: 周复盘Modal
story_key: 6-5-weekly-review-modal-real-data
version_anchor: da49e65a2f3c078fe0ccd355b82d0519f8cbe8ae
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider，否则 AI 建议降级
  isolation: write-isolated
  notes: 规划会创建大石头任务，验证后可清理。
---

# UAT-REVIEWMODAL-002 规划阶段AI建议大石头并采纳创建任务

## 业务场景
用户在规划阶段希望系统为每个角色给出 1-2 个大石头建议，自己可采纳或手动输入，确认后批量创建大石头任务并自动分类，不用逐个去角色视图新建。

## 前置条件
- 至少 2 个 active 角色
- 默认 LLM 可用

## 测试步骤
1. 打开 WeeklyReviewModal 规划阶段
2. 查看各角色的 AI 大石头建议
3. 采纳某条建议
4. 手动输入一条大石头
5. 确认创建

## 预期结果
- 各角色显示 1-2 个 AI 大石头建议
- 可采纳建议或手动输入
- 可添加多个大石头
- 确认后批量创建 is_big_rock=true 任务
- 创建后触发四象限自动分类
- 大石头数量限制（每角色 ≤ 3）生效

## 实际结果
<留空>

## 测试结论
<留空>
