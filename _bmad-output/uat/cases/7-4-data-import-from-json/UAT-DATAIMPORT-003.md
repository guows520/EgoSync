---
用例编号: UAT-DATAIMPORT-003
测试模块: 数据导入
story_key: 7-4-data-import-from-json
version_anchor: 7bde381
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
  isolation: read-only
  notes: 验证导入前自动备份。
---

# UAT-DATAIMPORT-003 导入前自动备份当前数据

## 业务场景
用户导入存档会覆盖当前数据，希望系统在导入前自动备份当前数据到临时目录，万一导入出问题还能找回原数据，7 天后自动清理避免占空间。

## 前置条件
- 当前有数据

## 测试步骤
1. 记录当前数据
2. 执行导入
3. 检查临时目录是否生成备份文件

## 预期结果
- 导入前自动创建备份到临时目录
- 备份文件名含时间戳
- 7 天后自动清理过期备份
- 导入失败时可从备份恢复

## 实际结果
<留空>

## 测试结论
<留空>
