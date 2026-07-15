---
用例编号: UAT-DATAIMPORT-002
测试模块: 数据导入
story_key: 7-4-data-import-from-json
version_anchor: 7bde381
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 导出存档
      ref: 之前导出的 .json 文件
      state: { 完整: true }
      auto_generatable: true
      requirement: 需用户提供之前导出的 JSON 存档文件路径
  isolation: write-isolated
  notes: 验证 JSON 格式导入数据保真。
---

# UAT-DATAIMPORT-002 JSON存档导入后数据100%保真

## 业务场景
用户从 JSON 存档导入，希望所有数据 100% 保真——角色、任务、记忆、对话、使命宣言、建议、通知、简报、周复盘全部完整恢复，不丢字段不丢记录。

## 前置条件
- 有完整的 JSON 存档文件

## 测试步骤
1. 记录存档中各表的记录数与关键字段
2. 执行 JSON 导入
3. 逐表检查恢复后的记录数与字段

## 预期结果
- 所有表数据完整恢复
- 记录数与存档一致
- 关键字段（角色名、任务标题、记忆内容等）不丢失
- 索引/触发器由 migration 创建，不受影响
- 数据 100% 保真

## 实际结果
<留空>

## 测试结论
<留空>
