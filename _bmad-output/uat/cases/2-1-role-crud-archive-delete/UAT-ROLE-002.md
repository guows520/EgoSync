---
用例编号: UAT-ROLE-002
测试模块: 角色管理-归档
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 待归档角色
      state: { name: "健康教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 保底角色
      state: { name: "写作顾问", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 需至少 2 个专属 active 测试角色，保证归档其一时不触发"至少保留一个角色"限制。跑完后清理这些专属角色。
---

# UAT-ROLE-002 归档角色后从侧边栏消失且历史保留

## 业务场景
用户有个暂时不常用的角色，想把它收起来让角色列表更整洁，但不希望丢失之前的对话历史，以后还能找回来。

## 前置条件
- 存在至少 2 个 active 角色（含专属测试角色"健康教练"）。
- "健康教练"此前已有过若干条对话记录。
- 当前进入"健康教练"角色视图的设置页。

## 测试步骤
1. 在设置页下方"危险区域"点击"归档角色"。
2. 在弹出的"确认归档角色"对话框中点击"确认归档"。

## 预期结果
- "健康教练"从左侧边栏角色列表中消失。
- 视图自动从"健康教练"切回管家视角。
- 该角色的对话历史数据仍被保留（未删除），可在全局设置的归档列表中看到它。

## 实际结果

## 测试结论
