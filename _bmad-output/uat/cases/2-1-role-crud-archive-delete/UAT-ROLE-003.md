---
用例编号: UAT-ROLE-003
测试模块: 角色管理-恢复
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 已归档角色
      state: { name: "家庭助理", status: archived }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 在用角色
      state: { name: "写作顾问", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 预置一个处于 archived 状态且带历史对话的专属角色"家庭助理"，并保证至少一个 active 角色存在。跑完后清理这些专属角色。
---

# UAT-ROLE-003 从全局设置恢复归档角色且历史完整

## 业务场景
用户之前归档了一个角色，现在又需要用它了，希望能从设置里把它恢复出来，并且之前的对话历史完整还在。

## 前置条件
- 存在一个已归档的专属测试角色"家庭助理"，且归档前留有对话历史。
- 应用已启动，当前在管家视角。

## 测试步骤
1. 点击侧边栏底部"设置"，打开全局设置。
2. 进入"归档角色"区域，找到"家庭助理"，确认其显示图标、名称、目标、归档时间。
3. 点击"家庭助理"对应的"恢复"按钮。

## 预期结果
- 恢复成功后，"家庭助理"重新出现在左侧边栏角色列表。
- 不会自动切换到该角色视图（除非用户主动点击）。
- 进入"家庭助理"角色视图，归档前的对话历史完整可见。

## 实际结果

## 测试结论
