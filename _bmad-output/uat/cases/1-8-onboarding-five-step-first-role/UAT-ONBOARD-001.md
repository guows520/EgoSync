---
用例编号: UAT-ONBOARD-001
测试模块: 五步引导首角色
story_key: 1-8-onboarding-five-step-first-role
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
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
      requirement: 需真实有效默认 LLM 配置以驱动真实引导对话与角色提议。
  isolation: write-isolated
  notes: 使用全新数据目录；测试后清理数据目录中创建的角色与 onboarding 标记。
---

# UAT-ONBOARD-001 新用户首次打开被引导五步对话创建首个角色

## 业务场景
全新用户第一次打开应用且已配置好模型。管家应主动出来打招呼，通过五步友好对话（欢迎→自我介绍→了解痛点→提议角色→确认创建）引导用户，在 5 分钟内创建出第一个角色并理解 EgoSync 的核心玩法。

## 前置条件
- 全新用户数据目录（从未完成过引导，无任何角色）
- 已配置默认 LLM 且连接正常

## 测试步骤
1. 启动应用（全新状态）
2. 确认应用自动进入引导界面，管家主动开场打招呼
3. 按引导依次回应：告诉管家称呼、最近在忙什么、关注的痛点
4. 等待管家基于对话提议一个角色（含名称/图标/颜色/目标）
5. 在角色确认弹窗中确认创建
6. 观察是否跳转进入管家主视图，且侧栏出现刚创建的角色

## 预期结果
- 应用自动进入引导，管家主动流式开场
- 五步对话流畅推进，管家提议的角色与用户描述相关
- 确认后成功创建角色，进入管家主视图
- 整个流程可在 5 分钟内完成

## 实际结果

## 测试结论
