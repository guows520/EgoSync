---
用例编号: UAT-MEM-003
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 记忆库初始为空或已知基线的 EgoSync
      state: { 已安装: true, 记忆基线已知: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 寒暄对话
      ref: 只包含寒暄/确认收到/空泛闲聊、无持久价值的管家会话
      state: { 内容: 仅寒暄, 无持久价值: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 预期不应新增记忆，验证前后记录记忆基线对比。建议测试身份/测试 DB 下执行。
---

# UAT-MEM-003 纯寒暄闲聊不产生记忆（业务异常：无价值对话不沉淀）

## 业务场景
用户只是随口和管家打了个招呼、说了几句"在吗""收到""今天天气不错"之类的客套话。用户希望系统有判断力，不要把这种没有长期价值的闲聊也当成"重要记忆"塞进记忆库，造成一堆无意义的噪音。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 记录当前记忆数量作为基线

## 测试步骤
1. 在管家对话中只进行寒暄式闲聊（如问候、确认收到、闲谈天气），不表达任何长期偏好或事实
2. 等待约 5 分钟或切换会话触发收尾
3. 打开记忆面板，对比提炼前后的记忆数量

## 预期结果
- 提炼后记忆数量与基线一致，没有为纯寒暄新增记忆条目
- 不向用户弹出"无记忆""提炼失败"之类的提示，体验上无感
- 应用不报错、对话仍可正常继续

## 实际结果
<留空>

## 测试结论
<留空>
