---
用例编号: UAT-MISSION-003
测试模块: 使命宣言
story_key: 5-1-mission-statement-setting
version_anchor: d396c3bc1ee58050f6d0f5dc6a1e5cc701d3aa02
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
    - type: 使命宣言
      ref: 已有一条使命宣言
      state: { content: "旧宣言" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证编辑覆盖。
---

# UAT-MISSION-003 编辑已有使命宣言覆盖旧内容

## 业务场景
用户的人生阶段变了，想修改使命宣言，希望直接在文本框改了保存就覆盖旧内容，不会出现多条使命宣言混乱。

## 前置条件
- 已有一条使命宣言

## 测试步骤
1. 打开管家设置，确认显示旧宣言
2. 修改文本内容
3. 保存
4. 重启应用确认

## 预期结果
- 保存后旧内容被覆盖（mission 表仅一行，INSERT OR REPLACE）
- 重启后显示新内容
- 不会出现多条使命宣言

## 实际结果
<留空>

## 测试结论
<留空>
