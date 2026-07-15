---
用例编号: UAT-DND-001
测试模块: 任务拖拽排序
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
      ref: 同一象限内至少 3 条未完成任务
      state: { 数量: 3, 同象限: true, 已完成: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 在专属测试角色下进行，拖拽后顺序会持久化，验证后可重置。
---

# UAT-DND-001 拖拽调整任务顺序并持久化

## 业务场景
用户希望按自己的优先级排列任务，把更重要的事拖到前面，松手后顺序就固定下来，下次打开还是这个顺序。

## 前置条件
- 某角色同一象限分组内至少有 2 条未完成任务
- 任务面板已打开

## 测试步骤
1. 进入角色工作台任务Tab
2. 按住某条任务卡片左侧的拖拽手柄（竖排小圆点图标）
3. 拖动到同象限内另一位置后松手
4. 观察列表顺序变化
5. 关闭任务面板后重新打开，再次观察顺序

## 预期结果
- 拖拽过程中显示半透明占位符，拖拽体验流畅
- 松手后任务在新位置实时排列
- 重新打开任务面板，顺序保持一致（已持久化）
- 拖拽手柄的点击不会触发任务编辑

## 实际结果
<留空>

## 测试结论
<留空>
