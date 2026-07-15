---
用例编号: UAT-MISSION-001
测试模块: 使命宣言
story_key: 5-1-mission-statement-setting
version_anchor: d396c3bc1ee58050f6d0f5dc6a1e5cc701d3aa02
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 使命宣言
      ref: 当前为空
      state: { content: 空 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后可清空使命宣言。
---

# UAT-MISSION-001 设定自由文本使命宣言并重启保留

## 业务场景
用户想写下自己的人生使命宣言作为角色间优先级指南，希望在管家设置里输入一段话保存，重启应用后还在，不用反复填。

## 前置条件
- 当前未设定使命宣言

## 测试步骤
1. 打开管家设置面板，找到使命宣言区域
2. 在文本框输入自由文本（如"家庭第一，事业第二，终身学习不停"）
3. 点击保存
4. 重启应用，再次打开管家设置

## 预期结果
- 保存成功，使命宣言写入 mission 表（format = free）
- 重启后使命宣言仍在，文本框显示之前输入的内容
- 不丢失数据

## 实际结果
<留空>

## 测试结论
<留空>
