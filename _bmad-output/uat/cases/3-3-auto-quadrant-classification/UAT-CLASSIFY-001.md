---
用例编号: UAT-CLASSIFY-001
测试模块: 任务自动分类
story_key: 3-3-auto-quadrant-classification
version_anchor: 0871096146e95899b732ed7902c8c7706896c066
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供一个有效的 LLM Provider（API Key 已配置并通过连接测试），否则自动分类会降级为 Q2
    - type: 角色任务
      ref: 新建任务，含标题与截止时间
      state: { 标题: "明天要交的季度汇报", 截止: 明天 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 自动分类异步执行，需等待分类完成事件。验证后清理任务。
---

# UAT-CLASSIFY-001 新任务由系统自动分配四象限

## 业务场景
用户创建任务时不想每次都手动判断"这事紧急还是重要"，希望系统根据任务内容、截止时间和角色目标自动分析并分到合适的象限。

## 前置条件
- 默认大模型已配置并能正常对话
- 某角色已存在，角色目标已填写

## 测试步骤
1. 在角色任务面板新建一条任务，填写明显紧急的标题（如"明天要交的季度汇报"）与截止时间（明天）
2. 不手动选择四象限分类，直接保存
3. 等待几秒，观察任务卡片上的象限标识变化

## 预期结果
- 保存后任务先以默认象限（Q2）出现
- 后台 LLM 分析完成后，象限自动更新（如明显临期+重要应升入 Q1）
- 分类完成后任务卡片象限标识实时更新，无需刷新

## 实际结果
<留空>

## 测试结论
<留空>
