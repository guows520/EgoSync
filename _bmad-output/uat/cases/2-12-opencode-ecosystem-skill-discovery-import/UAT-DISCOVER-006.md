---
用例编号: UAT-DISCOVER-006
测试模块: V1 不做远程市场
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_find_on
      state: { name: "UAT-发现角色", skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察 UI 范围边界，不修改持久状态，可基于共享基线只读检查。
---

# UAT-DISCOVER-006 V1 不提供远程市场/URL 下载

## 业务场景
确认 V1 的「发现」仅限本机 opencode 可见 Skill 目录，不提供从 URL/远程市场下载、不需要账号登录、无付费/评分功能。用户必须先把远程 Skill 保存为本地 SKILL.md 再走自定义导入。

## 前置条件
- 测试角色已启用 find-skills，可进入发现界面。

## 测试步骤
1. 打开「发现 opencode Skill」与自定义 Skill 导入相关界面，检查是否存在 URL 输入、远程市场、登录、评分、付费等入口。
2. 检查发现结果是否只来自本机 opencode skills 目录。

## 预期结果
- 界面无远程市场/URL 下载/登录/评分/付费入口。
- 发现结果仅来自本机 opencode skills 目录。
- 文档/提示明确远程 Skill 需先保存为本地 SKILL.md 再导入。
- 该范围边界需人工确认。

## 实际结果

## 测试结论
