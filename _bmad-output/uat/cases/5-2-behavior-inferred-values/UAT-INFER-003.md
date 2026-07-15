---
用例编号: UAT-INFER-003
测试模块: 价值观推断
story_key: 5-2-behavior-inferred-values
version_anchor: 2b77a1b
exec_mode: semi
destructive: read-only
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
  isolation: read-only
  notes: 验证拒绝推断。
---

# UAT-INFER-003 用户认为推断不准时关闭不采纳

## 业务场景
用户看到推断结果觉得不准（如系统说自己"工作效率优先"但其实自己更看重家庭），希望点"不准确"直接关闭，不被强加一个不认同的使命宣言。

## 前置条件
- 已生成推断结果

## 测试步骤
1. 查看推断结果
2. 点击"不准确"按钮
3. 确认弹窗关闭
4. 确认使命宣言仍未设定

## 预期结果
- 弹窗关闭，不写入 mission 表
- 使命宣言保持未设定状态
- 用户不被强加不认同的价值观
- 推断提示框可再次手动触发

## 实际结果
<留空>

## 测试结论
<留空>
