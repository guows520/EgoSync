---
用例编号: UAT-INFER-004
测试模块: 价值观推断
story_key: 5-2-behavior-inferred-values
version_anchor: 2b77a1b
exec_mode: auto
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 刚完成首次设置的新用户 EgoSync
      state: { 已安装: true, 对话轮数: 2, 任务数: 3 }
      auto_generatable: true
      requirement: false
    - type: 使命宣言
      ref: 未设定
      state: { content: 空 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证数据不足时不推断。
---

# UAT-INFER-004 新用户数据不足时不显示推断区域

## 业务场景
新用户刚用几天，对话和任务都很少，系统没有足够依据推断价值观，希望安静地不显示推断区域，不强行给出不靠谱的推断，等数据够了再说。

## 前置条件
- 对话 < 5 轮 或 任务总数 < 5
- 未设定使命宣言

## 测试步骤
1. 打开管家设置使命宣言区域
2. 观察是否出现推断提示框
3. 尝试触发推断

## 预期结果
- 不显示"系统推断的优先级"提示框
- mission_infer 返回 None（而非错误）
- 不阻塞用户使用其他功能
- 仲裁仅基于四象限+能量值（无使命依据时）

## 实际结果
<留空>

## 测试结论
<留空>
