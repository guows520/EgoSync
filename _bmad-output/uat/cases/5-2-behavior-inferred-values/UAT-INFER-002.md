---
用例编号: UAT-INFER-002
测试模块: 价值观推断
story_key: 5-2-behavior-inferred-values
version_anchor: 2b77a1b
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
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 使命宣言
      ref: 未设定，已生成推断结果
      state: { content: 空, 有推断: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 采纳会写入 mission 表，验证后可清空。
---

# UAT-INFER-002 用户确认采纳推断结果转为正式使命宣言

## 业务场景
用户看到系统推断的价值观觉得挺准，希望点击"确认采纳"把它转成自己的正式使命宣言，可先编辑再采纳，不用从零写。

## 前置条件
- 已生成推断结果

## 测试步骤
1. 在推断弹窗中查看推断的优先级列表与可编辑文本
2. 编辑其中一段文本
3. 点击"确认采纳"
4. 打开使命宣言区域确认

## 预期结果
- 弹窗显示推断的优先级列表 + 可编辑的使命宣言文本 + 置信度提示
- 编辑后点击"确认采纳"，编辑后内容写入 mission 表（format=free）
- 使命宣言区域显示采纳后的内容
- 推断提示框消失（已有正式使命宣言）

## 实际结果
<留空>

## 测试结论
<留空>
