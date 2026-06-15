---
用例编号: UAT-THEME-002
测试模块: 主题偏好持久化
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 用户主题偏好
      ref: theme-pref
      state: { storage_key: "egosync-theme" }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 用例写入并读取 localStorage 主题偏好；使用独立用户数据目录或测试前后清理该键，确保可重复执行。
---

# UAT-THEME-002 切换主题后重启应用仍保留偏好

## 业务场景
用户选定深色主题后，期望下次打开应用还是深色，不必每次重新设置——偏好应被记住。

## 前置条件
- 应用已启动；已清空既有主题偏好。

## 测试步骤
1. 将主题切换为深色。
2. 完全关闭应用（或刷新/重启 WebView）。
3. 重新打开应用，观察初始主题。
4. 再切回浅色并重启，确认浅色同样被记住。

## 预期结果
- 重启后应用以上次选择的主题（深色）打开。
- 切回浅色重启后同样保留浅色。
- 若从未设置过偏好，则按系统偏好决定，默认浅色。

## 实际结果

## 测试结论
