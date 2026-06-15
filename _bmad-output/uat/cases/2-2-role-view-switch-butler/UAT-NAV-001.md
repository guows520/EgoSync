---
用例编号: UAT-NAV-001
测试模块: 角色视图切换
story_key: 2-2-role-view-switch-butler
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 目标角色
      state: { name: "产品教练", status: active, color: "#4F46E5" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 创建专属测试角色"产品教练"。点击切换进入角色视图属可观察判定，但色温过渡/淡入需人眼确认，故 semi。跑完后清理专属角色。
---

# UAT-NAV-001 点击角色图标进入角色视图（色温过渡+淡入）

## 业务场景
用户在管家视角，想直接和某个角色对话。点击侧边栏该角色图标后，界面应平滑过渡到该角色的专属视图，呈现该角色的身份氛围。

## 前置条件
- 应用已启动，当前处于管家视角。
- 侧边栏存在 active 角色"产品教练"。

## 测试步骤
1. 点击侧边栏中的"产品教练"角色图标。
2. 观察主区颜色与内容切换过程。
3. 查看角色视图顶部头部信息。

## 预期结果
- 主区主题色在约 300ms 内平滑过渡到该角色的颜色，背景出现淡淡的同色底纹。
- 内容区以淡入方式显示该角色视图（约 250-300ms）。
- 角色视图头部显示来自后端的真实"角色名 + 图标 + 目标 + 能量值"，不是占位/mock 信息。

## 实际结果

## 测试结论
