---
用例编号: UAT-SKILL-002
测试模块: 角色主动性配置
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_proactivity
      state: { name: "UAT-主动性角色", proactivity_level: "moderate" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例会修改角色主动性级别，需使用专属测试角色；执行结束后删除该角色。
---

# UAT-SKILL-002 主动性级别切换并在重启后保持

## 业务场景
用户希望为角色选择三档协作主动性（静默执行 / 适度建议 / 积极主动），表达自己希望角色多大程度上主动提建议，并在重启后保持选择。

## 前置条件
- 应用已正常启动，存在测试角色「UAT-主动性角色」。
- 该角色当前主动性为默认的「适度建议」。

## 测试步骤
1. 进入「UAT-主动性角色」的角色设置页，找到主动性配置区。
2. 确认提供三档选项：静默执行、适度建议、积极主动，且当前选中「适度建议」。
3. 将主动性切换为「积极主动」。
4. 完全退出并重新启动应用。
5. 重新进入该角色设置页查看主动性选项。

## 预期结果
- 三档主动性文案准确显示为：静默执行、适度建议、积极主动。
- 切换后界面给出保存成功反馈。
- 重启后主动性仍保持「积极主动」。
- 该故事仅存储与展示主动性，不会出现任何后台主动弹出的建议卡片或通知（V1 范围边界）。

## 实际结果

## 测试结论
