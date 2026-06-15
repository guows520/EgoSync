---
用例编号: UAT-ONBOARD-004
测试模块: 五步引导首角色
story_key: 1-8-onboarding-five-step-first-role
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 全新用户数据目录
      ref: fresh_user
      state: { onboarding_completed: "不存在", roles: "空" }
      auto_generatable: true
      requirement: ""
    - type: 已配置默认 LLM
      ref: default_llm
      state: { is_default: true, connection: "正常" }
      auto_generatable: false
      requirement: 需真实有效默认 LLM 配置以驱动角色提议。
  isolation: write-isolated
  notes: 全新目录；测试后清理。
---

# UAT-ONBOARD-004 角色确认弹窗中编辑后再创建

## 业务场景
管家提议的角色名称、图标、颜色或目标，用户不一定完全满意。在最终确认创建前，用户应能在确认弹窗中自由修改这些信息，按自己的喜好定制后再落地，而不是被迫接受管家的提议。

## 前置条件
- 全新用户进入引导并推进到角色提议步骤
- 默认 LLM 正常

## 测试步骤
1. 完成引导前几步，等管家提议角色并弹出确认弹窗
2. 在弹窗中修改角色名称
3. 更换图标和颜色
4. 修改角色目标描述
5. 点击确认创建
6. 进入主视图后查看侧栏角色

## 预期结果
- 确认弹窗允许编辑名称/图标/颜色/目标
- 创建后的角色信息为用户编辑后的最终值（而非管家原提议值）
- 角色出现在侧栏，显示正确

## 实际结果

## 测试结论
