---
用例编号: UAT-THEME-001
测试模块: 主题切换即时生效
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 用户主题偏好
      ref: theme-pref
      state: { storage_key: "egosync-theme", initial: "light" }
      auto_generatable: true
      requirement:
    - type: 运行中的应用
      ref: app-running
      state: { running: true }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 切换会写入 localStorage 的 egosync-theme；用例前清空该键，用例后清理，避免影响其他用例的默认主题判定。
---

# UAT-THEME-001 点击切换按钮浅色/深色立即生效

## 业务场景
用户在不同光线环境下使用应用，希望随手点一下就能在浅色与深色之间切换，切换应当即时、无卡顿、不闪烁。

## 前置条件
- 应用已启动，当前为浅色主题（已清空主题偏好）。

## 测试步骤
1. 在侧边栏底部找到主题切换按钮（月亮/太阳图标）。
2. 点击按钮切换到深色主题，观察界面变化速度。
3. 再次点击切回浅色主题。

## 预期结果
- 点击后主题在 100ms 内立即切换，整界面（背景、文字、侧栏）同步变色。
- 图标在月亮/太阳之间相应切换；无明显闪烁或残留旧色块。

## 实际结果

## 测试结论
