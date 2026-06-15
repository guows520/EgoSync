---
用例编号: UAT-SIDEBAR-004
测试模块: 角色侧栏呼吸动画
story_key: 1-9-role-sidebar-breathing-animation
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 含不同能量值角色的用户数据目录
      ref: roles_varied_energy
      state: { roles: "三个，energy 分别 ≥80、40~79、<40" }
      auto_generatable: true
      requirement: ""
  isolation: read-only
  notes: 仅查看状态小点颜色，不改数据；如需精确能量值可脚本预置三个角色。
---

# UAT-SIDEBAR-004 角色状态小点按能量值呈现不同颜色

## 业务场景
用户希望一眼看出每个角色的"状态"。侧栏角色图标上的状态小点应随能量高低变色：能量充足为翠绿、中等为琥珀、偏低为暗灰，让用户快速感知哪些角色"精力旺盛"、哪些"需要关注"。

## 前置条件
- 已创建三个角色，能量分别为高（≥80）、中（40~79）、低（<40）

## 测试步骤
1. 进入管家主视图查看侧栏三个角色图标
2. 观察每个角色图标上的状态小点颜色
3. 将小点颜色与角色能量值对应核对

## 预期结果
- 能量 ≥80 的角色：小点为翠绿色
- 能量 40~79 的角色：小点为琥珀色
- 能量 <40 的角色：小点为暗灰色（不使用刺眼红色）

## 实际结果

## 测试结论
