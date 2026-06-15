---
用例编号: UAT-TONE-002
测试模块: 语调模板参考
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 任一角色
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
  isolation: read-only
  notes: 仅查看设置页模板参考文案，不改数据。可用共享基线角色。
---

# UAT-TONE-002 设置页展示预设语调模板参考

## 业务场景
用户在填写角色个性描述时，希望有几个现成的语调模板作为参考，帮助自己快速找到方向，但又能自由覆盖。

## 前置条件
- 进入任一角色视图的设置页。

## 测试步骤
1. 在"角色个性描述"输入框下方查看模板参考区域。
2. 阅读其中列出的预设语调模板。

## 预期结果
- 显示预设语调模板参考，至少包含：产品经理=简洁专业、家庭=温暖关怀、学习者=好奇探索（探索/启发式）三类方向。
- 模板仅作参考，不会自动覆盖用户已输入的个性描述。

## 实际结果

## 测试结论
