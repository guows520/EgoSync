---
用例编号: UAT-MEM-002
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已有至少一个角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 角色对话
      ref: 与某具体角色的、含 3 条以上领域相关用户消息的会话
      state: { 用户消息数: ">=3", 领域相关: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 提炼会向 memories 表写入该角色专属记忆（role_id=该角色）。建议在测试身份/测试 DB 下执行，验证后删除新增记忆。
---

# UAT-MEM-002 与角色对话后记忆归到该角色名下（记忆分域）

## 业务场景
用户和某个专属角色（如健身教练）聊了具体领域的事，希望这些专业相关的信息被记在这个角色名下，而不是和别的角色混在一起——这样每个角色只记住与它相关的事，互不串味。

## 前置条件
- EgoSync 已启动并存在至少一个角色
- 默认大模型可用

## 测试步骤
1. 进入某个具体角色（如健身教练）的对话
2. 围绕该角色领域完成一段有内容的对话（至少 3 条有价值用户消息）
3. 等待约 5 分钟或切换会话触发收尾
4. 分别查看该角色的记忆档案与其它角色/管家记忆，确认归属

## 预期结果
- 提炼出的记忆出现在该角色的记忆档案中，内容与对话领域相符
- 这些角色专属记忆不会错误出现在其它无关角色的记忆里
- 全局总览（管家记忆）按既定规则展示，不破坏归属边界

## 实际结果
<留空>

## 测试结论
<留空>
