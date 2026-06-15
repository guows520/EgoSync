---
用例编号: UAT-ONBOARD-003
测试模块: 五步引导首角色
story_key: 1-8-onboarding-five-step-first-role
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 已完成引导的用户数据目录
      ref: onboarded_user
      state: { onboarding_completed: "true", roles: "至少1个" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 复用已完成引导的数据目录；只读式验证不重复引导，测试后无需特别清理。
---

# UAT-ONBOARD-003 已完成引导的用户重启不再被引导

## 业务场景
用户已经完成过引导并创建了角色。之后每次打开应用，都应直接进入管家主视图，而不是再次被拉去走一遍引导，否则会非常烦人。

## 前置条件
- 用户数据目录中引导已标记完成，且已有至少一个角色

## 测试步骤
1. 在已完成引导的状态下启动应用
2. 观察启动后直接进入的界面
3. 完全关闭并再次启动，重复观察

## 预期结果
- 启动后直接进入管家主视图，不出现引导界面
- 多次重启均不重复引导
- 侧栏显示已创建的真实角色

## 实际结果

## 测试结论
