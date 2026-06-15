---
用例编号: UAT-MEMPANEL-003
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含可溯源记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 预置记忆与来源对话
      ref: 一条记忆及其对应的、仍存在的来源对话与来源消息
      state: { 来源对话存在: true, 来源消息存在: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 需保证记忆的 source_conversation 与 source_message 在 conversations.db 中真实存在。只读查看。
---

# UAT-MEMPANEL-003 展开记忆查看并高亮原始来源对话

## 业务场景
用户看到一条记忆，想知道"这是我哪次对话说的"，于是点开查看原文。用户希望系统能把当时对话的相关原话调出来，并高亮标出究竟是哪几句话让系统记下了这条记忆，从而判断这条记忆是否可信。

## 前置条件
- EgoSync 已启动
- 存在一条记忆，其来源对话与来源消息仍完整存在

## 测试步骤
1. 进入含该记忆的记忆档案
2. 点击该记忆卡片的"查看原文"
3. 观察展开区显示的原始对话片段
4. 检查作为来源依据的消息是否被高亮

## 预期结果
- 展开区显示该记忆来源对话中的相关原始消息，按原对话顺序排列
- 作为来源依据的消息以左边框/浅色背景等方式高亮
- 不暴露思考内容、内部路由信息或系统脚手架消息等非用户内容

## 实际结果
<留空>

## 测试结论
<留空>
