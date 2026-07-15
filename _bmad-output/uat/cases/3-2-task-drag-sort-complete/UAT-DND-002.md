---
用例编号: UAT-DND-002
测试模块: 任务完成
story_key: 3-2-task-drag-sort-complete
version_anchor: 5585768dcab685f56ddec0d45c4d2a0d0a3bd0f2
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
      ref: 一条未完成任务 task_incomplete
      state: { 已完成: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 完成会写入 completed_at 时间戳，验证后可重置为未完成。
---

# UAT-DND-002 勾选任务完成并折叠到已完成区

## 业务场景
用户做完一件事，想一键标记完成，希望看到清晰的"已完成"反馈（绿勾、灰显、删除线），且已完成任务不要淹没未完成任务。

## 前置条件
- 某角色下有至少一条未完成任务

## 测试步骤
1. 进入角色工作台任务Tab
2. 点击某条未完成任务卡片左侧的空心圆圈
3. 观察圆圈与卡片视觉变化
4. 观察该任务在列表中的位置变化

## 预期结果
- 圆圈变为绿色对勾，卡片切换为已完成态（灰显 + 标题删除线 + 沉底）
- 颜色/透明度有 ~200ms 过渡动画
- 完成态任务默认折叠为"已完成 (N)"摘要，不直接占用未完成任务区域
- 点击"已完成 (N)"可展开灰显列表

## 实际结果
<留空>

## 测试结论
<留空>
