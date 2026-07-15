---
用例编号: UAT-MISSION-002
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
  isolation: write-isolated
  notes: 验证结构化模板填充。
---

# UAT-MISSION-002 点击参考模板自动填入多段文本

## 业务场景
用户不知道怎么写使命宣言，希望系统提供几个参考模板（如全人平衡型、家庭为基事业为翼型等），点击就能把多段文本填入编辑框，自己再修改成贴合实际的版本。

## 前置条件
- 使命宣言区域可访问

## 测试步骤
1. 打开管家设置使命宣言区域
2. 点击"全人平衡型"模板按钮
3. 观察文本框内容
4. 修改其中一段后保存

## 预期结果
- 点击模板后多段文本自动填入编辑框
- 模板覆盖多种风格（全人平衡型/家庭为基事业为翼型等4个）
- 填入后可继续修改
- 修改后保存成功，format = structured

## 实际结果
<留空>

## 测试结论
<留空>
