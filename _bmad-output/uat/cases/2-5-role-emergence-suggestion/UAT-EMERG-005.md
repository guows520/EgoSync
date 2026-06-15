---
用例编号: UAT-EMERG-005
测试模块: 角色涌现建议
story_key: 2-5-role-emergence-suggestion
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、至少有一个 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 通过侧边栏手动创建角色会真实新增角色，验证后需删除该测试角色；本用例不依赖 LLM。
---

# UAT-EMERG-005 手动创建角色入口与涌现建议并存不受影响

## 业务场景
用户希望即使有了"管家自动建议建角色"这个新能力，原来通过侧边栏加号手动建角色的老习惯依然能用，两条路径互不干扰，自己想主动建角色时随时可以。

## 前置条件
- EgoSync 已启动，侧边栏可见加号创建入口

## 测试步骤
1. 点击侧边栏的加号创建角色入口
2. 在弹出的手动创建角色窗口中填写角色信息并确认
3. 查看角色是否正常创建并出现在侧边栏

## 预期结果
- 加号手动创建窗口正常弹出，与涌现建议的确认窗是两条独立路径
- 手动填写信息后能正常创建角色并出现在侧边栏
- 手动创建流程不受涌现建议功能影响，行为与以往一致

## 实际结果
<留空>

## 测试结论
<留空>
