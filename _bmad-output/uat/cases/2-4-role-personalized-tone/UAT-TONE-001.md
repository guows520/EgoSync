---
用例编号: UAT-TONE-001
测试模块: 个性描述编辑
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 目标角色
      state: { name: "产品教练", status: active, personality_prompt: "" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 创建专属角色"产品教练"，初始个性描述为空。用例编辑并保存个性描述。跑完后清理专属角色。
---

# UAT-TONE-001 编辑并保存角色个性描述

## 业务场景
用户希望自定义某个角色的回复风格，在设置里填写一段个性描述并保存，下次对话即按新风格回应。

## 前置条件
- 进入专属角色"产品教练"角色视图的设置页。

## 测试步骤
1. 在"角色信息"区域找到"角色个性描述"多行输入框。
2. 输入一段个性描述（如"简洁专业，先判断优先级再给建议"）。
3. 点击底部"保存更改"。
4. 切走再回到该角色设置页查看个性描述。

## 预期结果
- 保存成功（按钮短暂显示"已保存"），无错误提示。
- 重新进入设置页时，个性描述显示为刚保存的内容（已持久化）。
- 当前角色视图立即使用更新后的角色对象（无需重启应用）。

## 实际结果

## 测试结论
