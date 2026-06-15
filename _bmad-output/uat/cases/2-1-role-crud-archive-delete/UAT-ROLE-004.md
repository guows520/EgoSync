---
用例编号: UAT-ROLE-004
测试模块: 角色管理-永久删除
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 待删除角色
      state: { name: "临时顾问", status: active }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 保底角色
      state: { name: "写作顾问", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 创建专属测试角色"临时顾问"并预置若干对话；本用例会永久删除它及其关联对话。保留至少一个其它 active 角色以避免触发保护限制。
---

# UAT-ROLE-004 输入角色名二次确认后永久删除角色及其对话

## 业务场景
用户确定某个角色再也用不到了，想彻底删除它，包括它的所有对话记录，并且清楚这个操作不可撤销。

## 前置条件
- 存在专属测试角色"临时顾问"，且已有关联对话和消息。
- 存在至少一个其它 active 角色。
- 当前进入"临时顾问"角色视图的设置页。

## 测试步骤
1. 在"危险区域"点击"永久删除"。
2. 在弹出的"永久删除角色"对话框中，按提示在输入框输入角色名"临时顾问"。
3. 点击"永久删除"按钮。

## 预期结果
- 删除成功后"临时顾问"从侧边栏消失，且不出现在归档列表中。
- 该角色及其关联的对话、消息被一并删除，无法找回。
- 若当前停留在被删角色视图，视图自动切回管家视角。

## 实际结果

## 测试结论
