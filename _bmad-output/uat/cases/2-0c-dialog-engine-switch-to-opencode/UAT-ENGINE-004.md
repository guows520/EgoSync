---
用例编号: UAT-ENGINE-004
测试模块: 角色对话路由
story_key: 2-0c-dialog-engine-switch-to-opencode
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 多角色环境
      ref: 含管家与至少一个角色的 EgoSync 环境
      state: { 角色: "产品经理可对话" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例产生对话历史；建议在测试数据集运行。
---

# UAT-ENGINE-004 在不同角色对话区说话，分别由对应角色用各自语气回应

## 业务场景
用户在"管家"对话区和"产品经理"对话区分别说话，期望各自找到对的人——管家以总管语气回应，产品经理以产品视角语气回应，不会串味、不会都变成同一个口吻。

## 前置条件
- EgoSync 中已有管家和至少一个可对话的角色（如"产品经理"）

## 测试步骤
1. 在管家对话区发送一句话，记录回复语气
2. 切换到"产品经理"对话区，发送一句同类问题
3. 对比两边回复的身份与语气是否各自正确
4. 来回切换再各发一轮，确认不会互相串台、上下文各自独立

## 预期结果
- 管家对话区的回复是管家身份/语气
- 产品经理对话区的回复体现产品经理身份/语气
- 两个对话各自保持独立上下文，互不污染
- 切换角色后无需用户做额外设置，路由自动正确

## 实际结果
<留空>

## 测试结论
<留空>
