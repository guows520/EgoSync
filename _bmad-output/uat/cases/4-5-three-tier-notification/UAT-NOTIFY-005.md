---
用例编号: UAT-NOTIFY-005
测试模块: 三级通知
story_key: 4-5-three-tier-notification
version_anchor: 7a9d0e9
exec_mode: semi
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
      ref: 多条未读通知
      state: { 未读数: 3 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证已读标记与红点联动。
---

# UAT-NOTIFY-005 通知面板按时间倒序展示并支持已读标记

## 业务场景
用户点击铃铛打开通知面板，希望按时间倒序看到所有通知（每条含角色色标+角色名+级别标签+内容+时间），点击条目标记已读，红点 badge 根据未读数显示/隐藏。

## 前置条件
- 存在多条未读通知

## 测试步骤
1. 点击侧边栏通知铃铛
2. 观察通知面板内容与排序
3. 点击某条通知条目
4. 观察该条视觉变化与铃铛红点

## 预期结果
- 通知面板按时间倒序显示所有通知
- 每条显示：角色色标 + 角色名 + 级别标签（耳语/轻触/敲门）+ 内容 + 相对时间
- 敲门级通知标签带 animate-pulse
- 点击条目后 is_read 更新为 true，该条视觉降低 opacity
- 铃铛红点根据未读数显示/隐藏（全部已读后红点消失）
- 空状态显示"暂时没有新通知"

## 实际结果
<留空>

## 测试结论
<留空>
