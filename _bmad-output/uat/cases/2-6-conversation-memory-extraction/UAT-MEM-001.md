---
用例编号: UAT-MEM-001
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
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
      requirement: 后台记忆提炼依赖默认大模型，需用户提供有效 LLM Provider（API Key 已配置并通过连接测试）
    - type: 管家对话
      ref: 含有 3 条以上有价值用户消息的新管家会话
      state: { 用户消息数: ">=3", 含可记忆偏好: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 提炼会向 memories 表写入全局记忆。建议在专属测试身份/测试 DB 下执行，验证后删除新增记忆，避免污染真实记忆库。
---

# UAT-MEM-001 与管家深聊后系统自动沉淀全局记忆

## 业务场景
用户和管家聊了一段有内容的对话，分享了自己的总体偏好（如"我习惯晚上工作""不喜欢被频繁打扰"）。用户希望系统能像一个用心的助理一样，事后悄悄把这些重要信息记下来，下次不用重复交代，管家会越来越懂自己。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 准备一段包含至少 3 条有价值表达（偏好/事实/长期安排）的管家对话内容

## 测试步骤
1. 在管家对话中完成一段有内容的聊天，明确表达若干总体偏好或事实（至少 3 条用户消息）
2. 停止发送消息并等待约 5 分钟，或手动新建/切换到另一个对话以触发收尾
3. 打开管家记忆面板，查看是否新增了与刚才对话相符的记忆

## 预期结果
- 一段时间后（或切换会话后）管家记忆面板出现新记忆条目
- 新记忆内容与刚才表达的偏好/事实一致，归属为管家全局记忆
- 提炼过程在后台静默进行，不打断聊天、不阻塞继续发消息

## 实际结果
<留空>

## 测试结论
<留空>
