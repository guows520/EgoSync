---
用例编号: UAT-EMERG-001
测试模块: 角色涌现建议
story_key: 2-5-role-emergence-suggestion
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、至少有一个 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true, 无健身类角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供一个有效的 LLM Provider（API Key 已配置并通过连接测试），否则管家无法基于对话识别涌现信号
    - type: 管家对话
      ref: 一个用于多轮聊天的管家会话
      state: { 内容: 空 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 若验证中接受建议会创建新角色，需在专属测试环境或测试身份下进行，验证后删除新建角色与冷却记录，避免污染用户真实角色体系。
---

# UAT-EMERG-001 连续聊同一新领域后管家主动建议创建角色

## 业务场景
用户最近频繁和管家聊一个还没有专属角色覆盖的新领域（如健身），希望管家像一个细心的助理那样，注意到这个习惯后自然地提议"要不要专门弄一个这方面的角色"，让角色体系跟着自己的需求自然生长，而不是自己手动想着去建。

## 前置条件
- EgoSync 已正常启动并完成首次设置，侧边栏已有至少一个角色
- 当前不存在与"健身/运动"相关的角色
- 默认大模型已配置并能正常对话

## 测试步骤
1. 打开管家对话，连续 3 轮以上围绕"健身/运动"主题聊天（如请教训练计划、饮食搭配、运动恢复等）
2. 观察管家在前一两轮是否克制、没有第一句就提议建角色
3. 继续聊到第 3 轮及以后，观察管家的回复内容

## 预期结果
- 前 1-2 轮管家正常回答健身话题，不急于建议创建角色
- 聊到一定轮次后，管家以自然对话口吻主动提议创建健身相关角色（类似"我注意到你最近常聊健身，要不要创建一个健身教练角色？"）
- 该建议融入正常对话，不是弹窗、不打断当前回答

## 实际结果
<留空>

## 测试结论
<留空>
