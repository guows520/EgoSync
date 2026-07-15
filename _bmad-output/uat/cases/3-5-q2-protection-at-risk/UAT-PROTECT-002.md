---
用例编号: UAT-PROTECT-002
测试模块: Q2 任务保护
story_key: 3-5-q2-protection-at-risk
version_anchor: b248df2
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
    - type: 角色任务
      ref: 一条 at_risk 的 Q2 任务
      state: { quadrant: Q2, protection_status: at_risk }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证用户处理后预警立即消失。
---

# UAT-PROTECT-002 用户编辑或完成 at_risk 任务后预警消失

## 业务场景
用户看到"被挤压"预警后去处理这条任务（编辑内容或标记完成），希望预警立即消失，不用等下一轮定时检查，给自己"已处理"的清晰反馈。

## 前置条件
- 存在一条 protection_status = at_risk 的 Q2 任务

## 测试步骤
1. 打开该 at_risk 任务的编辑表单
2. 修改任意字段（如调整标题）并保存
3. 观察预警标识是否消失
4. （可选）对另一条 at_risk 任务点击完成，观察预警消失

## 预期结果
- 编辑保存后，protection_status 恢复为 normal
- 卡片琥珀色预警图标与左竖线立即消失
- 完成任务同样使预警消失
- 无需等待下一次定时检查

## 实际结果
<留空>

## 测试结论
<留空>
