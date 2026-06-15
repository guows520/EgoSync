---
用例编号: UAT-NAV-004
测试模块: 历史对话列表过滤
story_key: 2-2-role-view-switch-butler
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 目标角色
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: 对话
      ref: 角色对话
      state: { role_scope: 产品教练, count: ">=1" }
      auto_generatable: true
      requirement: ""
    - type: 对话
      ref: 管家对话
      state: { role_scope: butler, count: ">=1" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 预置该角色至少 1 条对话和管家至少 1 条对话。跑完后清理专属角色及对话。
---

# UAT-NAV-004 历史对话下拉按当前视角过滤

## 业务场景
用户打开"历史对话"列表时，应只看到与当前所处视角相关的对话：在角色视图看该角色的对话，在管家视图看管家的对话，避免混在一起难以查找。

## 前置条件
- 专属角色"产品教练"已有至少 1 条历史对话。
- 管家已有至少 1 条历史对话。

## 测试步骤
1. 进入"产品教练"角色视图，打开顶部"历史对话"下拉。
2. 查看列表内容。
3. 切回管家视角，打开"历史对话"下拉。
4. 查看列表内容。

## 预期结果
- 角色视图的历史对话列表仅显示属于"产品教练"的对话。
- 管家视图的历史对话列表仅显示属于管家的对话。
- 切换视角后，列表自动刷新为对应范围，不混入对方对话。

## 实际结果

## 测试结论
