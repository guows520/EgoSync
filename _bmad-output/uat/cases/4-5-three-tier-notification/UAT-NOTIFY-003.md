---
用例编号: UAT-NOTIFY-003
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
      ref: 一条敲门级通知
      state: { level: knock }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证敲门级嵌入 ActionCard + 红点。
---

# UAT-NOTIFY-003 敲门级通知嵌入管家对话区并显示红点

## 业务场景
角色生成"敲门"级通知（最重要），希望管家对话区嵌入 ActionCard 展示通知（标题+来源角色+时间+"立即处理/稍后"按钮），同时侧边栏铃铛显示红点，确保重要信息不被遗漏，但不使用传统 toast/snackbar 打断。

## 前置条件
- 存在一条 knock 级通知

## 测试步骤
1. 触发一条敲门级通知生成
2. 观察管家对话区与侧边栏铃铛
3. 检查 ActionCard 内容与按钮

## 预期结果
- 管家对话区嵌入 ActionCard 展示通知（图标+标题+来源角色+时间）
- ActionCard 有"立即处理"和"稍后"按钮
- 侧边栏铃铛显示红点 badge
- 不使用 toast/snackbar
- 敲门级通知标签带 animate-pulse 动画

## 实际结果
<留空>

## 测试结论
<留空>
