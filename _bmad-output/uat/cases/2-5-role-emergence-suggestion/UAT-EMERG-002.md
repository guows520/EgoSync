---
用例编号: UAT-EMERG-002
测试模块: 角色涌现建议
story_key: 2-5-role-emergence-suggestion
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 管家已发出涌现建议的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效的 LLM Provider 以驱动管家发出建议并执行创建提议
    - type: 涌现建议上下文
      ref: 管家已在对话中建议创建某新领域角色（可承接 UAT-EMERG-001）
      state: { 建议已出现: true }
      auto_generatable: false
      requirement: 需先通过真实对话让管家产生一次涌现建议
  isolation: write-isolated
  notes: 本用例会真实创建一个新角色，必须在专属测试环境/测试身份下执行，验证后删除该新建角色。
---

# UAT-EMERG-002 接受管家建议后引导确认并成功创建角色

## 业务场景
当管家建议创建某个新领域角色、用户也确实需要时，用户希望"答应一声"就能进入一个可编辑确认的创建界面，调整好名称、图标后确认，角色随即出现在侧边栏，立刻可以使用。

## 前置条件
- 管家已在对话中提议创建某新领域角色（如健身教练）
- 默认大模型可用

## 测试步骤
1. 在管家的创建建议下，回复表示接受（如"好啊，建一个吧"）
2. 观察界面是否弹出角色确认编辑窗
3. 在确认窗中查看/微调角色名称、图标、颜色、目标等信息后点击确认创建
4. 查看侧边栏角色列表

## 预期结果
- 用户接受后弹出角色编辑确认窗，可编辑名称、图标、颜色、目标
- 点击确认后窗口关闭，新角色出现在侧边栏角色列表中
- 新角色可被正常选中并进入对话

## 实际结果
<留空>

## 测试结论
<留空>
