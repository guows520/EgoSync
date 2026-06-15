---
用例编号: UAT-ENGINE-001
测试模块: 引擎切换后对话连续性
story_key: 2-0c-dialog-engine-switch-to-opencode
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已切换到新引擎的 EgoSync 应用
      state: { 已启动: true }
      auto_generatable: true
      requirement: false
    - type: 管家对话
      ref: 用于测试的管家对话
      state: { 内容: 空 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例会产生对话历史记录；建议在测试数据集运行。
---

# UAT-ENGINE-001 升级到新对话引擎后，和管家聊天体验依旧顺畅自然

## 业务场景
EgoSync 把背后的对话引擎换成了更强的版本（支持多步思考、调用工具做事）。对用户来说，最重要的是"换没换感觉不出来什么坏处"——打字机式的流式回复、思考过程展示等体验保持顺畅，甚至能做更复杂的事。

## 前置条件
- EgoSync 已正常打开（新引擎已生效）
- 管家对话可用

## 测试步骤
1. 在管家对话框输入一个稍复杂、需要分步处理的请求，如"帮我梳理一份本周工作计划，分成今天、明天、后天三块"
2. 观察回复是否以流式（逐字/逐段）方式呈现
3. 观察回复是否完整、连贯、符合请求
4. 连续再发 1～2 轮追问，确认上下文连续

## 预期结果
- 回复以流式方式自然呈现，不是长时间空等后一次性蹦出
- 回复内容完整、可用，能体现新引擎的多步处理能力
- 多轮对话上下文连续，管家记得前面聊过的内容
- 全程无技术性报错、无明显异常长的卡顿

## 实际结果
<留空>

## 测试结论
<留空>
