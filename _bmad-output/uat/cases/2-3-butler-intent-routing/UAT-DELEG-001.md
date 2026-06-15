---
用例编号: UAT-DELEG-001
测试模块: 管家委派
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 产品角色
      state: { name: "产品经理", status: active, goal: "跟进产品与OKR" }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 opencode sidecar 正常启动（含 delegate bridge），用于真实委派与角色回复。
  isolation: write-isolated
  notes: 创建专属"产品经理"角色；用例在管家对话中触发委派。委派语义与转述质量需人工判读，故 manual。跑完后清理专属角色及对话。
---

# UAT-DELEG-001 管家自动委派任务给匹配角色并转述结果

## 业务场景
用户只想面对一个稳定的"管家"，把需求直接说给管家，由管家自动安排合适角色处理，再把结果讲给自己，全程不需要手动切角色。

## 前置条件
- 存在 active 专属角色"产品经理"。
- LLM Provider 可用，opencode sidecar 与 delegate bridge 已就绪。
- 当前处于管家视角。

## 测试步骤
1. 在管家对话中输入"帮我跟进本周 OKR"并发送。
2. 观察管家的流式回复全过程。
3. 等待管家把角色处理结果转述完毕。

## 预期结果
- 管家在文字中显式提及正在委派（如"稍等，我让产品经理看一下"）。
- 全程停留在管家对话流中，不出现视图切换、弹窗或确认按钮。
- 管家随后以自己的话把"产品经理"的处理结果转述给用户。

## 实际结果

## 测试结论
