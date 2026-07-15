---
用例编号: UAT-DESTROY-001
测试模块: 数据销毁
story_key: 7-2-data-destroy-initial-state
version_anchor: fbdbc830f0744ea453e037157f8f398438225a95
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有完整数据的 EgoSync
      state: { 已安装: true, 有角色: true, 有任务: true }
      auto_generatable: true
      requirement: false
    - type: 备份文件
      ref: 销毁前自动创建的隐藏备份
      state: { 临时目录: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 此用例会销毁全部数据，必须在隔离环境执行，销毁后需用备份恢复。
---

# UAT-DESTROY-001 输入确认文字后销毁全部数据回到初始状态

## 业务场景
用户不想继续使用应用，希望彻底删除所有数据确保隐私不留残余，需要输入"确认销毁"四个字才能继续，避免误操作，销毁后应用回到全新安装状态。

## 前置条件
- 应用有完整数据
- 在隔离环境执行（销毁后需用备份恢复）

## 测试步骤
1. 打开全局设置"数据与主权"Tab
2. 点击"销毁所有数据"按钮
3. 阅读警告文案
4. 输入"确认销毁"
5. 点击"确认销毁"按钮
6. 等待销毁完成
7. 观察应用状态

## 预期结果
- 显示警告"此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。"
- 需输入"确认销毁"四个字才能启用确认按钮
- 销毁中显示 spinner + "销毁中..."
- 销毁完成后自动跳转到 Onboarding 引导页面
- 数据库为空（仅保留 schema），app_settings 为空表
- Sidebar 无角色，等同全新安装
- 销毁前自动创建隐藏备份（临时目录，7天自动清理）
- 同步删除系统钥匙串中所有 LLM API Key

## 实际结果
<留空>

## 测试结论
<留空>
