---
用例编号: UAT-REASON-002
测试模块: 推理透明与不确定性
story_key: 2-9-reasoning-transparency-uncertainty
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含多条相关记忆且大模型可用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 预置多条相关记忆
      ref: 同时支撑同一建议的多条可见记忆（如时间冲突类）
      state: { 条数: ">=2", 互相关联: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只发起对话查看证据链表达，不改数据。
---

# UAT-REASON-002 复合建议输出可读的证据链

## 业务场景
当一条建议同时依赖好几条记忆时（比如"周五有家庭聚餐"加上"产品评审也在周五"得出"建议调整"），用户希望 AI 能把推理过程串成一条清楚的证据链讲明白，让自己看懂建议是怎么一步步得出来的。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 已预置同时支撑同一建议的多条相关记忆

## 测试步骤
1. 引导 AI 给出一条需要综合多条记忆的建议
2. 追问原因，或让 AI 解释该建议
3. 查看回复是否呈现串联多条记忆的证据链

## 预期结果
- 回复输出简洁、可读的证据链（如 记忆A → 记忆B → 建议）
- 证据链只暴露依据链（记忆/规则/历史模式），不暴露隐藏的内部思考过程
- 多条记忆的排列服务于解释建议，逻辑清晰

## 实际结果
<留空>

## 测试结论
<留空>
