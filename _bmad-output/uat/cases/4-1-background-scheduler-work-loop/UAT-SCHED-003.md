---
用例编号: UAT-SCHED-003
测试模块: 后台调度器
story_key: 4-1-background-scheduler-work-loop
version_anchor: c83d6dd
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证角色间失败隔离。
---

# UAT-SCHED-003 单个角色工作循环失败不影响其他角色

## 业务场景
某个角色的工作循环出错了（比如 LLM 调用失败），希望其他角色不受影响，继续正常运转，不会一个角色崩了全部停摆。

## 前置条件
- 至少 2 个 active 角色均设为 moderate 或 proactive

## 测试步骤
1. 制造一个角色工作循环失败的场景（如该角色无目标/无任务/LLM 不可用）
2. 等待调度触发
3. 观察其他角色是否仍正常触发

## 预期结果
- 失败角色的错误只记录 tracing::warn 日志，不 panic
- 其他角色的工作循环照常执行，互不阻塞
- 应用整体不崩溃

## 实际结果
<留空>

## 测试结论
<留空>
