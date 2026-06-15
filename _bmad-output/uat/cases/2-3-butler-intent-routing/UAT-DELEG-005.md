---
用例编号: UAT-DELEG-005
测试模块: 委派目标无效兜底
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 已归档角色
      state: { name: "已归档专家", status: archived }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 构造委派目标不可用场景（角色归档/不存在）。命中归档角色需要诱导，属业务异常验证，需人工判读管家降级表现，故 manual。跑完后清理。
---

# UAT-DELEG-005 委派目标不可用时显式失败并自然降级（业务异常）

## 业务场景
当管家试图委派的角色不存在或已被归档时，系统不能崩溃，也不能把错误传染到管家对话；管家应当自然降级，用文字直接回应用户。

## 前置条件
- 存在一个已归档角色"已归档专家"（或构造一个不存在的目标）。
- LLM Provider 可用，sidecar/bridge 就绪，处于管家视角。

## 测试步骤
1. 在管家对话中提出一个会指向该归档/不存在角色的任务请求。
2. 观察管家的回复与应用稳定性。

## 预期结果
- 应用不崩溃、不卡死；委派目标不可用不会阻断其它正常处理。
- 管家不把该次委派的处理结果当作成功转述，而是自然降级，用文字直接回应用户（如说明该角色当前不可用）。
- 不会向不可用角色写入任何对话历史。

## 实际结果

## 测试结论
