---
用例编号: UAT-UI-002
测试模块: 移除 Pitch Mode 演示条
story_key: 1-3-component-domain-split-visual-zero-regression
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 生产构建产物
      ref: prod-build
      state: { built: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅观察界面，不改动数据。
---

# UAT-UI-002 生产界面不再出现顶部演示用 Pitch Mode 条

## 业务场景
顶部黑色 Pitch Mode 条仅用于路演演示，不应出现在交付给用户的正式版本里。用户打开应用时不应看到任何与演示相关的切换条。

## 前置条件
- 已生成生产构建并能启动。

## 测试步骤
1. 启动生产构建版本的应用。
2. 观察界面顶部是否存在黑色的 Pitch Mode 切换条。
3. 切换不同视图，确认没有"场景切换"(daily/conflict/review) 演示控件残留。

## 预期结果
- 界面顶部不再有黑色 Pitch Mode 条。
- 没有任何演示用场景切换控件；主内容区相比演示版整体上移。

## 实际结果

## 测试结论
