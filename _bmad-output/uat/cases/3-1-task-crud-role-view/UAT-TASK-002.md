---
用例编号: UAT-TASK-002
测试模块: 任务管理
story_key: 3-1-task-crud-role-view
version_anchor: dccfdc609cfa7410f35b9721304d6e311c0307a9
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条已存在的任务 task_existing
      state: { 标题: "旧任务标题", 已完成: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 在专属测试角色下进行，验证后清理任务。
---

# UAT-TASK-002 编辑已有任务并实时更新

## 业务场景
用户想修改一条已记录任务的内容或截止时间，希望点开就能改，保存后列表立刻显示新内容，不用刷新。

## 前置条件
- 某角色下已存在至少一条任务
- 任务面板可正常打开

## 测试步骤
1. 进入角色工作台的"任务清单"Tab
2. 点击一条已有任务卡片，进入编辑态
3. 修改任务标题为"更新后的标题"，调整截止时间和四象限分类
4. 点击"保存任务"按钮

## 预期结果
- 弹窗标题显示"编辑任务"，表单预填原内容
- 保存后弹窗关闭，任务卡片立即显示更新后的标题、截止时间、象限
- 不需要手动刷新页面

## 实际结果
<留空>

## 测试结论
<留空>
