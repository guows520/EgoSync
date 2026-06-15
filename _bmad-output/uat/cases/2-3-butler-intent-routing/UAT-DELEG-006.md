---
用例编号: UAT-DELEG-006
测试模块: 跨角色全局同步
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 私聊角色
      state: { name: "产品经理", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 先在角色视图私聊，再回管家询问近况，验证管家能引用各角色近况摘要。需人工判读管家是否掌握近况，故 manual。跑完后清理。
---

# UAT-DELEG-006 管家自动掌握各角色近况

## 业务场景
用户主动切到某角色私聊后回到管家，希望管家无需手动同步就能知道刚才与该角色聊了什么，体现"一个稳定助理掌握全局"的体验。

## 前置条件
- 存在 active 专属角色"产品经理"。
- LLM Provider 可用，sidecar/bridge 就绪。

## 测试步骤
1. 进入"产品经理"角色视图，私聊一个明确话题（如"我们决定下周做用户访谈"）并等待回复。
2. 切回管家视角。
3. 在管家对话中问"我刚才跟产品经理聊了什么"并发送。

## 预期结果
- 管家能在回复中引用刚才与"产品经理"私聊的近况要点（如"用户访谈"相关内容）。
- 管家未主动复述无关角色的近况摘要（仅在相关时引用）。

## 实际结果

## 测试结论
