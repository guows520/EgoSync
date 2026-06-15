---
用例编号: UAT-NAV-003
测试模块: 切回管家
story_key: 2-2-role-view-switch-butler
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 任一角色
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
  isolation: read-only
  notes: 仅做视图切换与查看，不改数据。需存在至少一个 active 角色（可用共享基线角色）。色温恢复需人眼确认，故 semi。
---

# UAT-NAV-003 从角色视图切回管家恢复管家色与历史

## 业务场景
用户在某角色视图聊完后，想回到管家面前继续全局事务。点击管家图标应平滑切回管家视角，恢复管家的主题色和管家对话历史。

## 前置条件
- 应用已启动，当前处于某个角色视图（如"产品教练"）。
- 管家此前已有对话历史。

## 测试步骤
1. 点击侧边栏顶部的管家（Home）图标。
2. 观察主区颜色变化与内容区。

## 预期结果
- 主区主题色在约 300ms 内恢复为管家色（靛蓝 #6366F1），同色底纹恢复为透明。
- 内容区显示管家的对话历史，而非角色对话或 mock 内容。

## 实际结果

## 测试结论
