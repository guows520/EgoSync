---
用例编号: UAT-DND-005
测试模块: 任务拖拽排序
story_key: 3-2-task-drag-sort-complete
version_anchor: 5585768dcab685f56ddec0d45c4d2a0d0a3bd0f2
exec_mode: manual
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证减弱动画模式的无障碍行为。
---

# UAT-DND-005 开启系统减弱动画后拖拽与完成动画减弱

## 业务场景
部分用户对动画敏感（前庭功能障碍），希望系统开启"减弱动画"后，拖拽与完成的过渡动画被简化或去除，不影响功能。

## 前置条件
- 操作系统已开启"减弱动画"或"减少动态效果"辅助功能

## 测试步骤
1. 开启系统级"减弱动画"设置
2. 打开任务面板，拖拽一条任务
3. 点击完成一条任务
4. 观察过渡动画

## 预期结果
- 拖拽与完成的 CSS 过渡动画被减弱（motion-reduce:transition-none 生效）
- 功能不受影响，仍可正常拖拽与完成
- 不会有突兀的位移或闪烁

## 实际结果
<留空>

## 测试结论
<留空>
