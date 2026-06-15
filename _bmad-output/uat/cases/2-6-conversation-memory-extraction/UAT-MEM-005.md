---
用例编号: UAT-MEM-005
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 管家对话
      ref: 已提炼过一次记忆的同一会话
      state: { 已提炼一次: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 重复触发同一会话提炼，预期不产生重复记忆。建议测试身份/测试 DB 下执行，验证后清理新增记忆。
---

# UAT-MEM-005 同一对话重复收尾不产生重复记忆

## 业务场景
用户可能在同一段对话里来回切换、停顿多次，导致系统多次触发对这段对话的记忆提炼。用户希望最终记忆面板里不要出现一堆内容几乎一样的重复条目，保持记忆库干净、好读。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 已有一段已完成首次提炼的管家对话，记忆面板已出现对应记忆

## 测试步骤
1. 回到该已提炼过的对话，再补一两句无新增有价值信息的话
2. 再次等待提炼收尾，或多次切换离开/回到该会话触发重复收尾
3. 打开记忆面板，检查是否出现与已有记忆内容重复的新条目

## 预期结果
- 记忆面板不出现与已有记忆来源、类别、内容相同的重复条目
- 同源信息即使被模型换种说法表达，也不会重复写入
- 记忆数量不因重复触发而异常膨胀

## 实际结果
<留空>

## 测试结论
<留空>
