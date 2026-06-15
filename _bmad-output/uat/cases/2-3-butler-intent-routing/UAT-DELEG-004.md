---
用例编号: UAT-DELEG-004
测试模块: 模糊意图追问
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 任一角色
      state: { name: "产品经理", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 验证意图模糊时管家追问而非硬委派。需人工判读管家是否追问，故 manual。跑完后清理新建对话。
---

# UAT-DELEG-004 意图模糊时管家追问而非强制委派（业务异常）

## 业务场景
当用户表达含糊、无法明确判断该由哪个角色处理时，管家应当主动追问而不是硬猜着委派，避免把任务派错角色。

## 前置条件
- 存在至少一个 active 角色。
- LLM Provider 可用，sidecar/bridge 就绪，处于管家视角。

## 测试步骤
1. 在管家对话中输入模糊请求"帮我想想"并发送。
2. 观察管家回复。

## 预期结果
- 管家不发起任何委派（不调用委派、不出现"我让 X 处理"的执行）。
- 管家用文字向用户追问，例如询问希望由哪个角色帮忙，或列出几个相关角色供选择。

## 实际结果

## 测试结论
