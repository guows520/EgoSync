---
用例编号: UAT-NOTIFY-004
测试模块: 三级通知
story_key: 4-5-three-tier-notification
version_anchor: 7a9d0e9
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色通知
      ref: 当天已有 3 条敲门通知
      state: { 今日knock数: 3 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证敲门降级。
---

# UAT-NOTIFY-004 同一天第 4 次敲门自动降级为轻触

## 业务场景
用户今天已经被"敲门"3 次了，第 4 次敲门触发时希望系统自动降级为"轻触"，避免过度打扰，tracing 日志记录降级原因。

## 前置条件
- 当天已有 3 条 knock 级通知

## 测试步骤
1. 确认当天已有 3 条 knock 通知
2. 触发第 4 条 knock 级通知
3. 检查实际写入的通知级别

## 预期结果
- 第 4 条 knock 自动降级为 tap
- 侧边栏铃铛红点更新，但不嵌入 ActionCard 到对话区
- tracing 日志记录降级原因
- 不向用户暴露降级细节

## 实际结果
<留空>

## 测试结论
<留空>
