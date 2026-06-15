---
用例编号: UAT-THEME-003
测试模块: 首次打开主题默认值
story_key: 1-4-theme-toggle-design-tokens
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 全新用户数据状态
      ref: fresh-profile
      state: { storage_key: "egosync-theme", value: "absent", system_prefers_dark: false }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 需在无主题偏好（已清空 localStorage 键）状态下启动；用例后清理。系统偏好可通过 OS 设置或 matchMedia mock 控制。
---

# UAT-THEME-003 无偏好时按系统设置决定，否则默认浅色

## 业务场景
首次使用的新用户没有任何主题偏好，应用应智能地跟随系统的深浅色设置；若系统也未指明，则给出舒适的默认（浅色），避免一上来就刺眼。

## 前置条件
- 全新状态：无主题偏好记录。

## 测试步骤
1. 在系统设置为浅色、无主题偏好的状态下首次打开应用，观察初始主题。
2. 将系统切换为深色偏好、再次清空偏好后打开应用，观察初始主题。

## 预期结果
- 系统为浅色（或未指明）时，应用首启为浅色。
- 系统偏好为深色时，应用首启自动为深色。

## 实际结果

## 测试结论
