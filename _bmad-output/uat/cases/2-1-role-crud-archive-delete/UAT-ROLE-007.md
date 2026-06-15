---
用例编号: UAT-ROLE-007
测试模块: 角色管理-新建
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 新建角色
      state: { name: "新建测试角色", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 本用例通过 UI 新建角色，验证其写入真实库（回归 2026-05-25 hotfix：新建走 mock 导致后续编辑保存 NotFound）。跑完后删除新建的专属角色。
---

# UAT-ROLE-007 侧边栏新建角色写入真实库后可正常编辑保存（回归）

## 业务场景
用户从侧边栏新建一个角色后，应当能立即对它进行编辑保存。曾出现新建角色未真正入库、导致随后编辑保存报"找不到角色"、并误触发"至少保留一个角色"的问题，需回归防护。

## 前置条件
- 应用已启动并完成引导，处于管家视角。

## 测试步骤
1. 点击侧边栏的"+ 添加角色"。
2. 填写角色名称"新建测试角色"，选择图标与颜色后确认创建。
3. 新角色出现在侧边栏后，点击进入其角色视图。
4. 打开设置页，修改名称为"新建测试角色v2"并点击"保存更改"。

## 预期结果
- 新角色创建后立即出现在侧边栏。
- 进入设置页编辑保存成功，无"找不到角色 / NotFound"类错误。
- 保存后名称更新为"新建测试角色v2"，未出现"至少保留一个角色"误报。

## 实际结果

## 测试结论
