---
用例编号: UAT-SIDEBAR-001
测试模块: 角色侧栏呼吸动画
story_key: 1-9-role-sidebar-breathing-animation
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 含真实角色的用户数据目录
      ref: user_with_roles
      state: { roles: "至少2个不同 energy 值", onboarding_completed: "true" }
      auto_generatable: true
      requirement: ""
  isolation: read-only
  notes: 仅查看侧栏渲染，不修改数据；共享基线即可。
---

# UAT-SIDEBAR-001 侧栏显示真实角色图标并带呼吸动画

## 业务场景
用户创建过角色后，侧栏应显示真实的角色（图标用 emoji），而不是写死的示例角色。每个角色图标缓缓地"呼吸"（明暗渐变），让用户感觉角色是"活的"实体，而非冷冰冰的静态图标。

## 前置条件
- 用户已创建至少 2 个角色（emoji 图标，不同能量值）
- 已完成引导，进入管家主视图

## 测试步骤
1. 打开应用进入管家主视图
2. 查看左侧角色侧栏显示的角色图标
3. 核对图标与名称是否与已创建的真实角色一致（emoji 正确渲染）
4. 静观角色图标的明暗变化（呼吸效果）

## 预期结果
- 侧栏显示真实创建的角色（emoji 图标正确显示，非示例数据）
- 未选中的角色图标呈现缓慢的呼吸动画（明暗渐变循环，约 3 秒一轮）
- 动画流畅、不卡顿

## 实际结果

## 测试结论
