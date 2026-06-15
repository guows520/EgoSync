---
用例编号: UAT-ROLE-005
测试模块: 角色管理-保底保护
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 唯一角色
      state: { name: "唯一角色", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 需构造"仅剩 1 个 active 角色"的隔离环境（专属测试库），其余角色归档或不存在。跑完后恢复，不影响共享角色库。
---

# UAT-ROLE-005 仅剩一个角色时禁止归档和删除（业务异常）

## 业务场景
系统要求用户至少保留一个可用角色。当只剩最后一个角色时，归档和删除入口应当被拦截，避免用户把自己置于"无角色可用"的状态。

## 前置条件
- 隔离环境中仅存在 1 个 active 角色"唯一角色"。
- 当前进入"唯一角色"角色视图的设置页。

## 测试步骤
1. 查看设置页"危险区域"。
2. 尝试点击"归档角色"按钮。
3. 尝试点击"永久删除"按钮。

## 预期结果
- "归档角色"与"永久删除"按钮均为禁用状态，无法点击。
- 危险区域显示提示文案"至少保留一个角色"。
- 不会发生任何归档或删除操作。

## 实际结果

## 测试结论
