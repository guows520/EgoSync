---
用例编号: UAT-FORGET-002
测试模块: 选择性遗忘
story_key: 2-8-selective-memory-forget
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 预置记忆
      ref: 记忆面板中至少一条记忆
      state: { 记忆数: ">=1" }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 取消遗忘不产生删除，全程不改数据，故为只读。
---

# UAT-FORGET-002 取消遗忘时无任何副作用（业务异常：取消保留）

## 业务场景
用户点了"遗忘"后又反悔了，点击"再想想"取消。用户希望这一取消是彻底干净的——记忆还在、数量不变、之前展开看的来源也不会莫名其妙被收起，就当什么都没发生过。

## 前置条件
- EgoSync 已启动
- 记忆面板中至少有一条记忆，记录当前可见记忆数

## 测试步骤
1. 进入记忆面板，记录当前记忆数量徽标
2. 若有已展开来源的记忆，保持其展开状态
3. 点击某记忆的"遗忘"，弹出确认后点击"再想想"/取消
4. 检查该记忆是否仍在、数量徽标是否变化、来源展开状态是否被影响

## 预期结果
- 该记忆仍保留在列表中，未被删除
- 记忆数量徽标保持不变
- 来源展开状态不被错误清空，界面状态与点击遗忘前一致

## 实际结果
<留空>

## 测试结论
<留空>
