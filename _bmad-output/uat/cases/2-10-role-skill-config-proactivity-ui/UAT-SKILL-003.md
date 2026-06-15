---
用例编号: UAT-SKILL-003
测试模块: 管家 Skill 配置
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 管家设置
      ref: uat_butler_skills
      state: { butler_skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 管家为全局单例配置，用例会修改其 Skill 开关；执行后需把管家 Skill 配置恢复为初始（两项开启）。建议在隔离的 UAT 应用数据目录运行。
---

# UAT-SKILL-003 管家 Skill 配置切换并持久化

## 业务场景
用户希望像配置角色一样，单独控制「管家」可用的两个默认能力插件，并在重启后保持配置。

## 前置条件
- 应用已正常启动，可进入管家设置页。
- 管家两个 Skill 开关初始均为开启。

## 测试步骤
1. 打开管家设置页，找到与角色同名的「Skill 配置」区块。
2. 确认区块展示 find-skills 与 skill-creator 两个开关及其来源说明。
3. 关闭 find-skills 开关。
4. 完全退出并重新启动应用。
5. 再次打开管家设置页查看 Skill 配置。

## 预期结果
- 管家设置页存在「Skill 配置」区块，含两个元 Skill 开关。
- 关闭后给出内联保存成功反馈。
- 重启后管家 find-skills 仍为关闭状态。

## 实际结果

## 测试结论
