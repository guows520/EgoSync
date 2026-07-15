---
用例编号: UAT-TASK-001
测试模块: 任务管理
story_key: 3-1-task-crud-role-view
version_anchor: dccfdc609cfa7410f35b9721304d6e311c0307a9
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、至少有一个 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务列表
      ref: 该角色当前任务为空或少量
      state: { 任务数: 0 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 在专属测试角色下进行，验证后清理新建任务，避免污染用户真实任务体系。
---

# UAT-TASK-001 在角色视图创建任务并立即看到

## 业务场景
用户在某个角色的工作台中，想快速记下一件要做的事，希望点一下"+"就能弹出表单，填完保存后任务立刻出现在列表里，不用刷新页面。

## 前置条件
- EgoSync 已正常启动并完成首次设置
- 侧边栏已有至少一个 active 角色
- 该角色当前任务面板可正常打开

## 测试步骤
1. 在侧边栏点击一个角色，进入角色工作台
2. 切换到"任务清单"Tab
3. 点击任务面板右上角的"+"按钮
4. 在弹出的新建任务表单中填写任务内容（如"整理本周读书笔记"），选择四象限分类、填写截止时间
5. 点击"保存任务"按钮

## 预期结果
- 表单弹出时标题为"新建任务"
- 保存后弹窗关闭，任务列表立即出现刚创建的任务，无需手动刷新
- 任务卡片显示标题、截止时间、四象限标识
- 该任务归属于当前角色，切换到其他角色看不到这条任务

## 实际结果
<留空>

## 测试结论
<留空>
