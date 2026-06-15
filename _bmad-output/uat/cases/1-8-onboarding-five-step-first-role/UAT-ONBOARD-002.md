---
用例编号: UAT-ONBOARD-002
测试模块: 五步引导首角色
story_key: 1-8-onboarding-five-step-first-role
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 全新且未配置 LLM 的数据目录
      ref: fresh_no_llm
      state: { onboarding_completed: "不存在", llm_configs: "空" }
      auto_generatable: true
      requirement: ""
    - type: 有效 LLM 配置物料
      ref: valid_llm_material
      state: {}
      auto_generatable: false
      requirement: 需一组真实有效的 LLM 服务地址、模型名、API Key，用于引导途中现场配置并继续。
  isolation: write-isolated
  notes: 全新目录；测试后清理配置、keyring 条目、角色与 onboarding 标记。
---

# UAT-ONBOARD-002 引导开始时未配置 LLM 引导用户先配置再继续

## 业务场景
全新用户打开应用但还没配置任何模型。引导无法在没有模型的情况下进行，因此应先把用户带去配置模型，等用户配好后自动回到引导继续，做到无缝衔接。

## 前置条件
- 全新用户数据目录，且尚未配置任何 LLM
- 准备一组真实有效的 LLM 配置物料

## 测试步骤
1. 启动应用（全新且无 LLM 配置）
2. 观察引导是否引导用户先去配置模型（跳到 LLM 设置）
3. 在设置中填入有效配置并测试连接成功、保存
4. 观察配置完成后是否自动返回引导继续

## 预期结果
- 引导检测到未配置 LLM，引导用户进入 LLM 设置页
- 用户配置并保存成功后，自动返回引导流程继续
- 不会卡死在引导无法推进

## 实际结果

## 测试结论
