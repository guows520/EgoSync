---
用例编号: UAT-REASON-001
测试模块: 推理透明与不确定性
story_key: 2-9-reasoning-transparency-uncertainty
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含可引用记忆且大模型可用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider 以驱动管家/角色基于记忆作答与解释
    - type: 预置可见记忆
      ref: 管家或角色名下若干条会被用于建议的可见记忆
      state: { 条数: ">=1", 可见: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只发起对话与追问、查看回复，不改记忆，故为只读。
---

# UAT-REASON-001 追问"为什么"得到带真实记忆依据的回复

## 业务场景
管家基于自己记住的事给出了一条建议，用户想知道"你凭什么这么说"。用户希望追问"为什么/你怎么知道的"后，AI 能老实说出依据的是哪几条记忆，而不是空泛敷衍，从而判断这个建议是否靠谱。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 管家或某角色名下已有可被引用的可见记忆

## 测试步骤
1. 引导管家/角色基于已有记忆给出一条建议
2. 追问"为什么""你怎么知道的"或"依据是什么"
3. 查看回复中是否给出引用的记忆条目
4. 核对回复中引用的记忆是否与记忆面板中真实存在的记忆一致

## 预期结果
- 回复包含引用的记忆条目（以可识别的记忆引用形式标注，如 [记忆#…]）
- 引用指向的是真实存在的记忆，内容摘要与该记忆实际内容一致，未编造
- 若确实没有可引用记忆，AI 明确说明"没有可溯源的记忆依据"，不伪造来源

## 实际结果
<留空>

## 测试结论
<留空>
