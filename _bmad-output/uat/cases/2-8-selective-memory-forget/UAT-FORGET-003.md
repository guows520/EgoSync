---
用例编号: UAT-FORGET-003
测试模块: 选择性遗忘
story_key: 2-8-selective-memory-forget
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含可删除记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 可删除测试记忆
      ref: 已知所属范围（角色或管家）与类别的一条测试记忆
      state: { 可删除: true, 有来源对话: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 会真实删除 memories 记录；必须用专门预置的测试记忆/测试身份执行，验证后无需还原（属删除验证）。需确认 conversations.db 原始对话未受影响。
---

# UAT-FORGET-003 确认遗忘后记忆移除、徽标减一、历史对话不受影响

## 业务场景
用户确认遗忘一条记忆后，希望它立刻从列表里消失、数量也相应减一，但当初产生这条记忆的原始对话记录依然完整保留在历史里——遗忘的只是"被记住的结论"，不是"聊过的事实"。

## 前置条件
- EgoSync 已启动
- 存在一条已知所属范围与来源对话的测试记忆，记录当前可见记忆数

## 测试步骤
1. 进入含该测试记忆的记忆面板，记录数量徽标
2. 点击该记忆"遗忘"并确认
3. 查看该卡片是否从列表移除、徽标是否减一
4. 前往历史对话/该记忆的原始来源对话，确认原始消息是否仍在且未被改动

## 预期结果
- 确认后该记忆卡片从列表移除，数量徽标减一（按当前 Tab/筛选口径重算）
- 角色页只刷新当前角色范围，管家页刷新全局+角色总览范围
- 原始历史对话与消息内容仍完整存在、未被删除或篡改
- 若删除后列表为空，显示温暖空态而非"暂无数据"

## 实际结果
<留空>

## 测试结论
<留空>
