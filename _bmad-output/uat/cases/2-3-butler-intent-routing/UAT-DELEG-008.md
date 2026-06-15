---
用例编号: UAT-DELEG-008
测试模块: 禁止嵌套委派
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 角色集
      state: { count: ">=2", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 验证转述阶段（follow-up）不会再次触发委派（V1 边界）。需人工判读，故 manual。跑完后清理。
---

# UAT-DELEG-008 转述阶段不再二次委派（V1 边界，业务异常防护）

## 业务场景
管家在拿到角色结果做转述时，即使话里提到"再让另一个角色看看"，本轮也不应真的再发起新的委派，必须等用户下一条消息。这是 V1 的安全边界，防止失控连环委派。

## 前置条件
- 存在至少 2 个 active 角色。
- LLM Provider 可用，sidecar/bridge 就绪，处于管家视角。

## 测试步骤
1. 在管家对话输入一个会触发委派、且转述时容易牵扯其它角色的请求（如"帮我跟进产品计划，顺便看看要不要健康教练配合"）。
2. 观察管家委派与转述全过程，特别是转述阶段是否又发起新的委派。

## 预期结果
- 本轮内管家可以委派一次（或并行多个），但进入转述阶段后不会再发起新的委派。
- 即便转述文字提到"再让 X 看看"，也不会真正触发新委派，需用户下一条消息才会启动新一轮。

## 实际结果

## 测试结论
