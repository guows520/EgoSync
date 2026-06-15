---
用例编号: UAT-DISCOVER-004
测试模块: opencode Skill 导入与角色启用
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_find_on
      state: { name: "UAT-发现角色", skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
    - type: opencode skills 目录预置 Skill
      ref: uat_opencode_skill
      state: { name: "uat-discover-skill", description: "用于导入测试的 opencode skill" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 导入会复制到 EgoSync 受控目录并写 registry；执行后删除 registry 条目、角色绑定与受控副本。
---

# UAT-DISCOVER-004 导入 opencode Skill 并立即启用、重启保留

## 业务场景
用户从扫描结果中选择一个第三方 opencode Skill 导入，期望它进入 EgoSync Skill 库（sourceType=opencode），可立即在当前角色启用，重启后导入与启用状态都保留。

## 前置条件
- 已通过「发现 opencode Skill」扫描出 uat-discover-skill 且未导入。
- 测试角色已启用 find-skills。

## 测试步骤
1. 在扫描结果中点击 uat-discover-skill 的「导入」。
2. 查看导入结果反馈与列表中的已导入状态。
3. 在当前角色启用该 Skill。
4. 完全退出并重启应用。
5. 再次查看该 Skill 导入状态与角色启用状态。

## 预期结果
- 导入成功，sourceType=opencode，文件复制到受控目录（源文件删改后仍可用）。
- 导入后可立即在当前角色启用。
- 若导入后同步暂时失败，应提示「已导入，但同步暂时失败，将在下次同步自动生效」而非谎称已启用。
- 重启后导入状态与角色启用状态均保留。

## 实际结果

## 测试结论
