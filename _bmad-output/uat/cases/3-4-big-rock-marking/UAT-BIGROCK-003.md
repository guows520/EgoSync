---
用例编号: UAT-BIGROCK-003
测试模块: 大石头标记
story_key: 3-4-big-rock-marking
version_anchor: c4248c6
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条已标记大石头的任务
      state: { is_big_rock: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证取消标记的幂等语义。
---

# UAT-BIGROCK-003 取消大石头标记后标签消失

## 业务场景
用户改判了，想把某条任务从大石头中移除，希望取消勾选保存后琥珀色标签消失，任务回到普通排序位置。

## 前置条件
- 某角色下有至少一条已标记大石头的任务

## 测试步骤
1. 打开该大石头任务的编辑表单
2. 取消勾选"标记为本周大石头"
3. 保存
4. 观察任务列表

## 预期结果
- 保存后琥珀色"大石头"标签消失
- 该任务不再享受大石头优先排序，回到按 sortOrder 的位置
- 即使角色已有 3 个大石头，取消标记也不触发数量校验（取消不报错）

## 实际结果
<留空>

## 测试结论
<留空>
