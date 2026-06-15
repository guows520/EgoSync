---
用例编号: UAT-IMPORT-003
测试模块: 自定义 Skill 重复导入
story_key: 2-11-custom-skill-md-import-role-binding
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 合法 SKILL.md 文件
      ref: uat_dup_skill_md
      state: { name: "uat-dup-skill", description: "用于重复导入测试", content_hash_stable: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需先导入一次再重复导入；执行后删除该 Skill 并清理受控目录副本。
---

# UAT-IMPORT-003 重复导入同名/相同内容 Skill 提示已存在

## 业务场景
用户再次导入了已经存在（同名或相同内容哈希）的 Skill。系统应提示已存在，允许取消或覆盖元数据，而不是默默创建一个无法区分的重复条目。

## 前置条件
- Skill 库中已存在通过导入得到的 uat-dup-skill。
- 应用已启动，持有同一份 SKILL.md。

## 测试步骤
1. 进入角色 Skill 区域，再次点击「导入自定义 Skill」。
2. 选择与已存在 Skill 相同内容（或同名）的 SKILL.md。
3. 观察系统提示。
4. 分别尝试「取消」与「覆盖元数据」。
5. 查看 Skill 列表条目数量。

## 预期结果
- 系统检测到重复并提示「已存在」。
- 提供取消与覆盖元数据两种选择。
- 取消后不新增条目；覆盖后仍为同一条目（不产生不可区分的重复条目）。

## 实际结果

## 测试结论
