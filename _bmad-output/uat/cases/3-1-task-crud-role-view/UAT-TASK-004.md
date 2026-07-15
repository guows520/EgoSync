---
用例编号: UAT-TASK-004
测试模块: 任务管理
story_key: 3-1-task-crud-role-view
version_anchor: dccfdc609cfa7410f35b9721304d6e311c0307a9
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证空标题与非法象限的校验反馈。
---

# UAT-TASK-004 任务标题为空或象限非法时给出友好提示

## 业务场景
用户匆忙中没填任务内容就点保存，希望系统友好地提示"任务内容不能为空"，而不是默默保存一条空任务或报一堆技术错误。

## 前置条件
- 某角色任务面板可正常打开

## 测试步骤
1. 点击"+"新建任务
2. 不填写任何任务内容，直接点击"保存任务"
3. 观察保存按钮状态与提示
4. 填写标题后再次尝试保存

## 预期结果
- 标题为空时保存按钮禁用或点击后显示中文内联错误（如"任务内容不能为空"）
- 弹窗保持打开，不会创建空任务
- 填写标题后保存按钮恢复可用，可正常保存

## 实际结果
<留空>

## 测试结论
<留空>
