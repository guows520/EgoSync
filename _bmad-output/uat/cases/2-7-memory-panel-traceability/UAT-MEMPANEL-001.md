---
用例编号: UAT-MEMPANEL-001
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已有角色且该角色已积累若干记忆的 EgoSync
      state: { 已安装: true, 角色记忆数: ">=1" }
      auto_generatable: true
      requirement: false
    - type: 预置角色记忆
      ref: 某角色名下的多条结构化记忆（含不同类别与创建时间）
      state: { 条数: 5, 含来源对话: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只读查看记忆列表，可用脚本预置测试角色与记忆。验证仅浏览不改数据。
---

# UAT-MEMPANEL-001 角色记忆档案按时间倒序展示记忆卡片

## 业务场景
用户进入某个角色的记忆档案，希望看到这个角色记住的事情，最近记住的排在最上面，每条都清楚标明是什么类别、记了什么、什么时候记的、来自哪段对话，一目了然。

## 前置条件
- EgoSync 已启动
- 该角色名下已有若干条记忆（建议 5 条，覆盖不同类别与时间）

## 测试步骤
1. 进入该角色视图
2. 打开"记忆档案"Tab
3. 查看记忆卡片的排列顺序与每张卡片显示的信息

## 预期结果
- 记忆卡片按创建时间从新到旧排列
- 每张卡片显示类别标签、内容摘要、创建时间，以及查看来源对话的入口
- 角色页只显示该角色名下的记忆，不混入其它角色或纯全局记忆

## 实际结果
<留空>

## 测试结论
<留空>
