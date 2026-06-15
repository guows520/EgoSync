---
用例编号: UAT-REASON-005
测试模块: 推理透明与不确定性
story_key: 2-9-reasoning-transparency-uncertainty
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含历史记忆引用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider 以产生含记忆引用的历史回复
    - type: 历史引用 + 已遗忘记忆
      ref: 一条历史回复引用了某条之后被遗忘的记忆
      state: { 历史引用存在: true, 目标记忆已遗忘: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 构造该状态需先产生含引用的回复并遗忘对应记忆。使用测试记忆/测试身份。验证后无需还原（记忆已遗忘）。
---

# UAT-REASON-005 点击已遗忘记忆的历史引用温和降级（业务异常：引用已遗忘记忆）

## 业务场景
一条旧回复里引用了某条记忆，但用户后来把那条记忆遗忘了。当用户再点这个旧引用时，希望系统温和地告诉自己"这条记忆现在不可用，可能已被遗忘"，既不伪造一条来源糊弄自己，也不会跳错角色或显示已删除的残留内容。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 存在一条历史回复引用了某条之后被遗忘的记忆

## 测试步骤
1. 找到该含旧记忆引用的历史回复
2. 确认该引用指向的记忆已被遗忘
3. 点击该历史引用
4. 观察界面反馈与历史回复文本

## 预期结果
- 给出温和的不可用反馈（如"这条记忆现在不可用，可能已经被遗忘了"）
- 不伪造来源、不切换到错误角色、不展示已删除记忆的残留内容
- 历史回复中既有的记忆引用文本本身不被篡改；其它仍存在记忆的引用点击仍正常工作

## 实际结果
<留空>

## 测试结论
<留空>
