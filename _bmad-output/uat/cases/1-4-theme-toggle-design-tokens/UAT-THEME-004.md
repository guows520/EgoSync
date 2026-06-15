---
用例编号: UAT-THEME-004
测试模块: 无障碍-减少动效
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 系统无障碍设置
      ref: os-reduced-motion
      state: { prefers_reduced_motion: "reduce" }
      auto_generatable: false
      requirement: 需在操作系统层开启"减少动态效果/reduce motion"无障碍选项（Windows: 显示动画效果关闭；macOS: 减弱动态效果），无法纯脚本生成。
    - type: 运行中的应用
      ref: app-running
      state: { running: true }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 修改的是 OS 无障碍开关（环境状态），用例后恢复原系统设置。
---

# UAT-THEME-004 开启系统"减少动效"后呼吸/过渡动画基本停止

## 业务场景
对动效敏感或有前庭障碍的用户会在系统开启"减少动态效果"。应用应尊重该设置，让呼吸动画、淡入淡出、过渡等几乎瞬时完成，避免不适。

## 前置条件
- 在操作系统开启"减少动态效果/prefers-reduced-motion: reduce"。
- 应用已启动。

## 测试步骤
1. 确认系统已开启减少动效。
2. 打开应用，观察侧边栏角色图标的呼吸动画是否还在循环。
3. 切换主题、打开弹窗，观察过渡/淡入动画是否被显著缩短。

## 预期结果
- 呼吸动画不再持续循环。
- 主题切换、弹窗淡入等动效时长降至近似瞬时（≤ 10ms），界面仍可正常使用。

## 实际结果

## 测试结论
