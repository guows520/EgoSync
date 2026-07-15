---
用例编号: UAT-SUGGEST-002
测试模块: 主动建议生成
story_key: 4-2-proactive-suggestion-generation
version_anchor: c83d6dd
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 角色建议
      ref: 近 7 天已有建议
      state: { 近7天有建议: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证去重逻辑。
---

# UAT-SUGGEST-002 新建议与近 7 天已有建议不重复

## 业务场景
用户不希望每天收到重复或高度相似的建议，希望系统生成新建议时自动规避最近 7 天已说过的内容，保持建议的新鲜度。

## 前置条件
- 角色近 7 天已有建议记录
- 默认 LLM 可用

## 测试步骤
1. 记录角色近 7 天已有建议的标题
2. 触发新一轮工作循环
3. 检查新生成的建议标题

## 预期结果
- 新建议标题与近 7 天已有建议不重复（LLM 判重 + 确定性兜底）
- 归一化后 title 与近 7 天某条完全相同的候选不写入
- 不使用 embedding/余弦相似度

## 实际结果
<留空>

## 测试结论
<留空>
