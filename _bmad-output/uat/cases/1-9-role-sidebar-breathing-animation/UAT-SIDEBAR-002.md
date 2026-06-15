---
用例编号: UAT-SIDEBAR-002
测试模块: 角色侧栏呼吸动画
story_key: 1-9-role-sidebar-breathing-animation
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 开启减弱动效的系统环境
      ref: reduced_motion_env
      state: { prefers_reduced_motion: "reduce" }
      auto_generatable: false
      requirement: 需在操作系统层面开启"减弱动态效果/reduce motion"无障碍设置。
    - type: 含真实角色的用户数据目录
      ref: user_with_roles
      state: { roles: "至少1个" }
      auto_generatable: true
      requirement: ""
  isolation: read-only
  notes: 仅查看渲染表现，不修改数据。
---

# UAT-SIDEBAR-002 系统开启减弱动效时呼吸动画停止

## 业务场景
部分用户因眩晕敏感或偏好，在系统中开启了"减弱动态效果"。应用应尊重该设置，停止侧栏图标的呼吸动画，让图标保持静态清晰，体现无障碍关怀。

## 前置条件
- 操作系统已开启"减弱动态效果"
- 至少已创建一个角色

## 测试步骤
1. 在系统设置中开启"减弱动态效果"
2. 启动应用进入管家主视图
3. 观察侧栏角色图标是否还有呼吸动画

## 预期结果
- 呼吸动画停止，图标保持静态、清晰可见（完全不透明）
- 图标仍正确显示，无异常

## 实际结果

## 测试结论
