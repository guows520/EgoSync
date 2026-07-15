---
用例编号: UAT-DATAIMPORT-001
测试模块: 数据导入
story_key: 7-4-data-import-from-json
version_anchor: 7bde381
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 导出存档
      ref: 之前导出的 .db 或 .json 文件
      state: { 完整: true }
      auto_generatable: true
      requirement: 需用户提供之前导出的存档文件路径
  isolation: write-isolated
  notes: 导入会覆盖当前数据，必须在隔离环境执行，导入前自动备份。
---

# UAT-DATAIMPORT-001 从导出存档一键恢复全部数据

## 业务场景
用户换了设备或销毁数据后，希望从之前导出的存档文件一键恢复全部数据，回到完整工作状态，不用重新建角色、录任务。

## 前置条件
- 有之前导出的 .db 或 .json 存档文件
- 在隔离环境执行（导入会覆盖当前数据）

## 测试步骤
1. 打开全局设置"数据与主权"Tab
2. 点击"导入存档"按钮
3. 选择存档文件（.db 或 .json）
4. 确认导入
5. 等待导入完成
6. 检查恢复的数据

## 预期结果
- 自动识别文件格式（SQLite .db 或 JSON .json）
- 导入前自动备份当前数据到临时目录
- SQLite 导入通过 ATTACH DATABASE 逐表复制
- JSON 导入反序列化后事务内清空+逐表写入
- 导入完成后数据完整恢复（角色、任务、记忆、对话等）
- 返回导入统计
- 不需要关闭/重连数据库连接池

## 实际结果
<留空>

## 测试结论
<留空>
