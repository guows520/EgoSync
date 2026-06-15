---
用例编号: UAT-REASON-004
测试模块: 推理透明与不确定性
story_key: 2-9-reasoning-transparency-uncertainty
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含可引用记忆且大模型可用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider，使回复中产生真实记忆引用
    - type: 预置可见记忆
      ref: 管家或角色名下可被引用的记忆
      state: { 条数: ">=1" }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 点击引用只打开记忆面板并定位，不改数据，故为只读。
---

# UAT-REASON-004 点击对话中的记忆引用跳转并定位高亮记忆卡片

## 业务场景
用户在对话里看到 AI 引用了某条记忆，想顺手点一下就直接看到那条记忆的全貌。用户希望点击引用后，右侧记忆面板自动打开并滚动定位到那条记忆，短暂高亮，省去自己去翻找的麻烦。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 对话回复中已包含指向真实记忆的可点击引用

## 测试步骤
1. 在管家或角色对话中得到一条含记忆引用的回复
2. 点击回复中的记忆引用
3. 观察右侧工作台是否打开记忆面板并定位到目标记忆
4. 在角色视图与管家视图分别验证一次

## 预期结果
- 点击记忆引用后打开对应工作台的记忆面板
- 必要时自动清空/调整类别筛选，使目标记忆可见
- 滚动到目标记忆卡片并短暂高亮；点击不会触发页面跳转或破坏正文渲染
- 管家视图使用全局+角色总览语义，角色视图优先定位当前角色范围内记忆

## 实际结果
<留空>

## 测试结论
<留空>
