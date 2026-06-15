---
用例编号: UAT-MEMPANEL-002
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含多类别记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 预置角色记忆
      ref: 同一角色名下覆盖偏好/事实/认知模式等多类别的记忆
      state: { 含偏好: true, 含其它类别: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只读筛选，可脚本预置不同类别记忆。
---

# UAT-MEMPANEL-002 按类别筛选记忆

## 业务场景
当某个角色记住的事情比较多时，用户希望能按类别快速过滤——比如只看"偏好"，或只看"事实"，方便聚焦查看某一类信息，而不用在一长串卡片里翻找。

## 前置条件
- EgoSync 已启动
- 该角色记忆覆盖多个类别（至少包含偏好和另一类别）

## 测试步骤
1. 进入该角色记忆档案
2. 在顶部类别筛选器中选择"偏好"
3. 查看列表变化
4. 依次切换其它类别选项（全部、任务状态、认知模式、事实）查看过滤效果

## 预期结果
- 选择"偏好"后只显示偏好类记忆，其它类别被过滤掉
- 切换各类别选项时列表相应变化，"全部"显示该角色全部可见记忆
- 角色页筛选只作用于当前角色记忆范围

## 实际结果
<留空>

## 测试结论
<留空>
