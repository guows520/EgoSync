---
用例编号: UAT-SIDEBAR-003
测试模块: 角色侧栏呼吸动画
story_key: 1-9-role-sidebar-breathing-animation
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 含真实角色的用户数据目录
      ref: user_with_roles
      state: { roles: "至少2个，含一个高能量与一个低能量" }
      auto_generatable: true
      requirement: ""
    - type: 屏幕阅读器环境
      ref: screen_reader
      state: { tool: "NVDA / VoiceOver" }
      auto_generatable: false
      requirement: 需安装并开启屏幕阅读器（Windows NVDA 或 macOS VoiceOver）。
  isolation: read-only
  notes: 仅验证无障碍朗读与键盘导航，不改数据。
---

# UAT-SIDEBAR-003 键盘导航与屏幕阅读器朗读角色与能量

## 业务场景
依赖键盘或屏幕阅读器的用户也要能使用侧栏。用键盘上下键应能在角色图标间切换，回车进入角色视图；屏幕阅读器聚焦图标时应朗读角色名与能量值（如"产品经理 - 能量值 85%"），保障无障碍可用性。

## 前置条件
- 已创建至少 2 个角色（含不同能量值）
- 开启屏幕阅读器

## 测试步骤
1. 进入管家主视图，用 Tab 键将焦点移到侧栏
2. 用上下方向键在角色图标间切换焦点
3. 监听屏幕阅读器对当前聚焦角色的朗读内容
4. 在某个角色上按回车

## 预期结果
- 上下键可在角色图标间切换焦点，焦点有可见提示
- 屏幕阅读器朗读出角色名 + 能量值（如"产品经理 - 能量值 85%"）
- 回车进入对应角色视图

## 实际结果

## 测试结论
