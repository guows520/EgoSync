---
用例编号: UAT-NOTIFY-001
测试模块: 三级通知
story_key: 4-5-three-tier-notification
version_anchor: 7a9d0e9
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色通知
      ref: 一条耳语级通知
      state: { level: whisper }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证耳语级静默积累。
---

# UAT-NOTIFY-001 耳语级通知静默积累不打扰

## 业务场景
角色生成"耳语"级通知（最轻微），希望它静默积累到通知列表，不显示任何前台提示、不弹窗、不响铃，等下次晨间简报再汇总，给用户最大专注空间。

## 前置条件
- 存在一条 whisper 级通知

## 测试步骤
1. 触发一条耳语级通知生成
2. 观察前台是否有提示
3. 打开通知面板，确认通知已记录

## 预期结果
- 不显示任何前台提示（无弹窗、无 ActionCard、无声音）
- 侧边栏铃铛不显示红点 badge
- 通知已写入 DB，通知面板可查
- 留待下次晨间简报汇总

## 实际结果
<留空>

## 测试结论
<留空>
