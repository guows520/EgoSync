---
用例编号: UAT-AGENT-001
测试模块: 角色身份同步
story_key: 2-0b-role-agent-mapping-permissions
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 正常运行的 EgoSync 应用
      state: { 已启动: true }
      auto_generatable: true
      requirement: false
    - type: 新角色
      ref: 一个待创建的测试角色
      state: { 名称: "产品经理", 目标: "帮我管理产品规划与需求梳理" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例会新建角色并写入用户数据库及后台配置；建议在测试数据集上运行，验证后可归档/删除该角色还原。
---

# UAT-AGENT-001 新建一个角色后，它立刻拥有独立身份并能以自己的方式对话

## 业务场景
用户在 EgoSync 里创建一个"产品经理"角色，期望它不是一个空壳，而是一个有自己目标、自己说话风格的独立分身——创建完就能切过去和它对话，并且它的回答符合"产品经理"这个设定。

## 前置条件
- EgoSync 已正常打开并能对话
- 当前没有同名的"产品经理"角色

## 测试步骤
1. 进入角色管理，新建角色：名称填"产品经理"，目标填"帮我管理产品规划与需求梳理"
2. 保存角色
3. 切换到"产品经理"角色的对话区
4. 发送一句话，如"我想做一个记账 App，你帮我从产品角度提几个关键问题"
5. 观察回复是否体现出"产品经理"的身份与目标导向

## 预期结果
- 角色创建成功，出现在角色列表中
- 切换到该角色后可以正常对话
- 回复明显带有产品经理视角（围绕需求、用户、规划等），而非泛泛的通用回答
- 整个过程无需用户手动去配置任何后台"agent"或"引擎参数"

## 实际结果
<留空>

## 测试结论
<留空>
