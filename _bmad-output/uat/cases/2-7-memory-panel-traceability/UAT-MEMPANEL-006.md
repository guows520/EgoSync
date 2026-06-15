---
用例编号: UAT-MEMPANEL-006
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含来源已缺失记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 来源缺失记忆
      ref: 一条记忆，其来源对话或来源消息已被删除/不可用
      state: { 记忆存在: true, 来源对话已删除: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需构造来源对话被删除而记忆仍在的状态（删除 conversations.db 中对应对话）。建议在测试 DB 下构造，验证后还原。
---

# UAT-MEMPANEL-006 来源对话已删除时温和提示不可用（业务异常：来源缺失）

## 业务场景
有时一条记忆对应的原始对话已经被删掉了。用户点开想看原文时，希望系统温和地告诉自己"原始对话已经不在了"，而不是崩溃、报红、弹错误框，破坏整体的稳妥感。

## 前置条件
- EgoSync 已启动
- 存在一条记忆，其来源对话或来源消息已被删除/不可用

## 测试步骤
1. 进入含该记忆的记忆档案
2. 点击该记忆的"查看原文"
3. 观察展开区的提示与应用状态

## 预期结果
- 展开区显示温和的不可用文案（如"来源对话已不可用"）
- 应用不崩溃、不弹错误 toast
- 其它来源完整的记忆仍能正常展开查看原文

## 实际结果
<留空>

## 测试结论
<留空>
