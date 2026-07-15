---
用例编号: UAT-INFER-001
测试模块: 价值观推断
story_key: 5-2-behavior-inferred-values
version_anchor: 2b77a1b
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有充足历史数据的 EgoSync
      state: { 已安装: true, 对话轮数: 10, 任务数: 10 }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider，否则推断降级为 None
    - type: 使命宣言
      ref: 未设定
      state: { content: 空 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 推断结果不持久化，仅展示。
---

# UAT-INFER-001 未设定使命时基于行为推断隐含价值观

## 业务场景
用户没显式写使命宣言，但希望系统通过分析自己最近 30 天的对话、任务、记忆模式，推断出 2-3 条隐含价值观（如"家庭陪伴 > 工作效率 > 个人学习"），让自己看到"原来系统是这样理解我的"。

## 前置条件
- 未设定使命宣言
- 有充足历史数据（对话 ≥ 5 轮，任务 ≥ 5 条）
- 默认 LLM 可用

## 测试步骤
1. 打开管家设置使命宣言区域
2. 观察是否出现"系统推断的优先级"提示框
3. 点击"推断使命宣言"按钮
4. 查看推断结果

## 预期结果
- 出现"系统推断的优先级"灰色提示框
- 推断结果为 2-3 条价值观摘要
- 含置信度提示
- 推断内容可追溯到近期行为模式（如频繁处理家庭任务→家庭优先）

## 实际结果
<留空>

## 测试结论
<留空>
