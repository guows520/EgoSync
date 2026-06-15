---
用例编号: UAT-NAV-002
测试模块: 角色对话历史隔离
story_key: 2-2-role-view-switch-butler
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 角色A
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 角色B
      state: { name: "健康教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置一个可用的 LLM Provider（opencode 可连通的模型），用于角色对话产生真实回复。
  isolation: write-isolated
  notes: 创建 2 个专属测试角色并在各自视图发消息，验证历史互不污染。跑完后清理专属角色及其对话。
---

# UAT-NAV-002 不同角色对话历史互不污染

## 业务场景
用户分别和两个角色对话，每个角色应只保留属于自己的对话历史，互相看不到对方的消息，避免身份串台。

## 前置条件
- 存在 2 个 active 专属角色"产品教练"和"健康教练"。
- 已配置可用 LLM Provider。

## 测试步骤
1. 进入"产品教练"角色视图，发送一条消息（如"帮我梳理本周产品重点"）并等待回复。
2. 切换到"健康教练"角色视图，发送另一条不同的消息（如"给我一个跑步计划"）并等待回复。
3. 再切回"产品教练"角色视图，查看其对话流。
4. 再次切到"健康教练"角色视图，查看其对话流。

## 预期结果
- "产品教练"视图只显示与产品教练的对话，看不到"跑步计划"相关消息。
- "健康教练"视图只显示与健康教练的对话，看不到"产品重点"相关消息。
- 切换角色时旧消息被清空，仅加载属于当前角色的历史。

## 实际结果

## 测试结论
