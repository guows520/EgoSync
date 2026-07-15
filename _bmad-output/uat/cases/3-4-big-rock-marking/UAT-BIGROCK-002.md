---
用例编号: UAT-BIGROCK-002
测试模块: 大石头标记
story_key: 3-4-big-rock-marking
version_anchor: c4248c6
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 该角色已有 3 条大石头任务
      state: { 大石头数: 3 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条待标记的第 4 个任务
      state: { is_big_rock: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证每角色 ≤ 3 个大石头的限制。
---

# UAT-BIGROCK-002 每角色最多 3 个大石头，超出时友好提示

## 业务场景
用户想标记第 4 件大石头，希望系统温和地拦住自己："每周最多 3 个大石头，请先取消一个再标记"，避免大石头泛滥失去聚焦意义。

## 前置条件
- 某角色已有 3 条 is_big_rock = true 的未软删除任务

## 测试步骤
1. 打开第 4 条任务的编辑表单
2. 勾选"标记为本周大石头"
3. 点击保存
4. 观察提示与表单状态

## 预期结果
- 保存失败，弹窗保持打开
- 显示中文提示"每个角色每周最多 3 个大石头，请先取消一个再标记"
- 该任务的 is_big_rock 未被写入（仍是 false）
- 不会出现技术性错误堆栈

## 实际结果
<留空>

## 测试结论
<留空>
