---
用例编号: UAT-NAV-005
测试模块: 角色身份呈现
story_key: 2-2-role-view-switch-butler
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 对话角色
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置一个可用的 LLM Provider，用于角色对话产生真实回复。
  isolation: write-isolated
  notes: 验证角色对话气泡身份正确（不误称管家、输入框提示含角色名）。回归 2026-05-24 体验修复。需人工判读身份措辞，故 manual。跑完后清理新建对话。
---

# UAT-NAV-005 角色对话中身份正确不误称管家（业务异常防护）

## 业务场景
用户在角色视图与角色对话时，对方应当以该角色身份回应，而不是自称"管家"或退回通用助理口吻。曾出现角色气泡仍显示管家、角色自称管家、输入框写"跟管家说点什么"的问题，需防护。

## 前置条件
- 进入专属角色"产品教练"角色视图，并新建一个全新对话（避免旧会话历史中的旧身份消息干扰）。
- 已配置可用 LLM Provider。

## 测试步骤
1. 查看角色视图输入框的占位提示文案。
2. 向"产品教练"发送一条自我介绍类问题（如"你是谁，能帮我做什么"）。
3. 阅读角色回复气泡的署名/头像与措辞。

## 预期结果
- 输入框占位提示包含当前角色名（如"跟产品教练说点什么"），不是"跟管家说点什么"。
- 回复气泡呈现为该角色身份（角色名/图标/颜色），不是管家身份。
- 回复内容以该角色身份自述，不自称管家、不退回通用助理口吻。

## 实际结果

## 测试结论
