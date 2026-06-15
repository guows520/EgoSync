---
用例编号: UAT-MEM-004
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 失效的 LLM Provider 环境
      ref: 默认大模型不可用（如 Key 失效/网络不通/无默认 Provider）
      state: { 可用: false }
      auto_generatable: false
      requirement: 需用户构造一个会导致后台提炼失败的环境（例如临时禁用网络、配置无效 Key、或移除默认 Provider）
    - type: 管家对话
      ref: 含有 3 条以上有价值用户消息的会话
      state: { 用户消息数: ">=3" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后需恢复有效 Provider/网络。建议测试身份/测试 DB 下执行。
---

# UAT-MEM-004 提炼失败时静默不打扰用户（业务异常：提炼失败不影响对话）

## 业务场景
用户在大模型临时不可用（网络断开、Key 失效）的情况下和管家聊了天。用户希望即使后台记忆提炼这件"幕后小事"失败了，也不要跳出错误提示来吓自己，更不能影响当下正常的聊天体验——提炼成不成功是系统自己的事，不该让用户买单。

## 前置条件
- EgoSync 已启动
- 默认大模型处于不可用状态（无效 Key / 断网 / 无默认 Provider）

## 测试步骤
1. 在大模型不可用的状态下，与管家完成一段有价值的对话（注意：发送对话本身可能也受影响，可在对话完成后再切断 Provider 以专门验证提炼失败路径）
2. 等待约 5 分钟或切换会话触发记忆提炼收尾
3. 观察界面是否弹出与"记忆提炼失败"相关的错误提示
4. 确认聊天界面仍可正常使用

## 预期结果
- 后台提炼失败不向用户弹出错误提示或 toast
- 对话完成状态不受影响，用户可继续发消息、切换会话
- 应用不崩溃、不卡死；记忆面板保持原有内容（不写入错误数据）

## 实际结果
<留空>

## 测试结论
<留空>
