---
用例编号: UAT-EMERG-004
测试模块: 角色涌现建议
story_key: 2-5-role-emergence-suggestion
version_anchor: 5268797
exec_mode: manual
destructive: read-only
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、至少有一个 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效的 LLM Provider
  isolation: read-only
  notes: 本用例只观察管家是否打断/弹窗，不接受创建建议，不产生角色或冷却记录，故为只读。
---

# UAT-EMERG-004 涌现建议不打断正常对话且可被忽略（业务异常：不阻塞对话）

## 业务场景
用户在和管家聊天的过程中，即便管家有了建角色的想法，也不希望被弹窗硬性打断。用户应当能选择无视这个建议，继续聊别的话题，整个对话体验是顺畅、不被强迫的。

## 前置条件
- EgoSync 已启动并有至少一个角色
- 默认大模型可用

## 测试步骤
1. 与管家就某新领域连续聊几轮，诱导出涌现建议
2. 当管家提出创建角色建议时，不回应该建议，直接转而问一个完全不同话题的问题
3. 观察对话是否能继续，以及是否出现强制弹窗

## 预期结果
- 管家提建议时不弹出强制模态窗，建议只以对话文字形式出现
- 用户忽略建议、转聊新话题时，管家正常回答新话题，不卡住、不反复追问是否要建角色
- 在用户未明确接受前，不会自动创建角色

## 实际结果
<留空>

## 测试结论
<留空>
