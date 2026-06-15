---
用例编号: UAT-ROLE-006
测试模块: 角色管理-删除确认
story_key: 2-1-role-crud-archive-delete
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 待删除角色
      state: { name: "测试角色A", status: active }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 保底角色
      state: { name: "测试角色B", status: active }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 创建 2 个专属测试角色；本用例验证删除确认拦截，不应真正删除。跑完后清理专属角色。
---

# UAT-ROLE-006 删除确认输入名称不匹配时拦截删除（业务异常）

## 业务场景
永久删除是不可逆操作，系统要求用户手动输入角色名做二次确认。若用户输错或留空，删除按钮必须保持不可用，防止误删。

## 前置条件
- 存在专属测试角色"测试角色A"，且存在至少一个其它 active 角色。
- 当前进入"测试角色A"角色视图的设置页。

## 测试步骤
1. 在"危险区域"点击"永久删除"，打开删除确认对话框。
2. 不输入任何内容，观察"永久删除"按钮状态。
3. 在输入框输入一个错误的名称（如"测试角色X"），再次观察按钮状态。
4. 点击"取消"关闭对话框。

## 预期结果
- 输入框为空时，"永久删除"按钮为禁用状态。
- 输入的名称与角色名不一致时，"永久删除"按钮仍为禁用状态。
- 点击"取消"后对话框关闭，"测试角色A"未被删除，仍在侧边栏中。

## 实际结果

## 测试结论
