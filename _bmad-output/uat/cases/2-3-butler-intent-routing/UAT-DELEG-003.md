---
用例编号: UAT-DELEG-003
测试模块: 委派写入角色历史
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 被委派角色
      state: { name: "产品经理", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 在管家触发委派后切到角色视图核对委派痕迹。是否出现"[管家委派]"痕迹可客观判定，但回复内容需人确认，故 semi。跑完后清理。
---

# UAT-DELEG-003 委派交互写入角色对话历史，角色"记得"被委派

## 业务场景
管家委派给角色后，这次委派应当沉淀进该角色自己的对话历史，使用户主动切到该角色视图时能看到完整的委派来龙去脉，角色不是"空壳"。

## 前置条件
- 存在 active 专属角色"产品经理"。
- 已在管家对话中触发过一次对"产品经理"的委派（可先执行 UAT-DELEG-001）。

## 测试步骤
1. 点击侧边栏"产品经理"图标，进入其角色视图。
2. 查看该角色的对话历史。

## 预期结果
- 该角色对话历史中出现一条用户侧消息，内容带"[管家委派]"前缀及任务摘要（如有上下文也一并体现）。
- 紧随其后是该角色自己的回复消息。
- 委派历史完整可见，与管家转述内容相对应。

## 实际结果

## 测试结论
