---
用例编号: UAT-CLASSIFY-002
测试模块: 任务自动分类
story_key: 3-3-auto-quadrant-classification
version_anchor: 0871096146e95899b732ed7902c8c7706896c066
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
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 角色任务
      ref: 一条低置信度分类任务
      state: { confidence: 0.6 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证低置信度分类的象限标识。
---

# UAT-CLASSIFY-002 低置信度分类的象限显示

## 业务场景
系统对某条任务的紧急/重要程度判断不太确定时，希望坦诚地告诉用户"这条我不太确定"，让用户决定是否手动调整，而不是装作很笃定。

## 前置条件
- 默认大模型已配置
- 存在一条置信度低于 80% 的任务

## 测试步骤
1. 创建一条语义模糊的任务（如"想想以后的事"，无明确截止时间）
2. 等待自动分类完成
3. 观察该任务卡片的视觉标识

## 预期结果
- 任务自动分类为 Q2（重要不紧急）
- 任务卡片象限标识显示为 Q2
- 低置信度时后端记录 confidence 值，但前端不显示特殊标记

## 实际结果
<留空>

## 测试结论
<留空>
