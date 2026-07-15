---
用例编号: UAT-PERF-002
测试模块: 性能基准
story_key: 8-4-performance-benchmark
version_anchor: 4f44008
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有多个角色的 EgoSync
      state: { 已安装: true, 角色数: 3 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证呼吸动效 60fps。
---

# UAT-PERF-002 角色卡片呼吸动效60fps无掉帧

## 业务场景
用户希望角色卡片的呼吸动效流畅——60fps 无掉帧，hover/过渡动画顺滑，不因动画卡顿影响体验。

## 前置条件
- 有多个角色卡片显示

## 测试步骤
1. 打开显示角色卡片的界面
2. 用 Chrome DevTools Performance 录制呼吸动效
3. 检查帧率
4. hover 角色卡片观察过渡

## 预期结果
- 呼吸动效 ≥ 60fps
- hover/过渡动画无掉帧
- 使用 CSS transform/opacity + GPU 加速
- 不使用 JS 动画（无 requestAnimationFrame 循环、无 setInterval 动画）

## 实际结果
<留空>

## 测试结论
<留空>
