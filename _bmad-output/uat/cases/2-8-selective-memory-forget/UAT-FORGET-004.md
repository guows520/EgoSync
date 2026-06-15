---
用例编号: UAT-FORGET-004
测试模块: 选择性遗忘
story_key: 2-8-selective-memory-forget
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含可删除记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 可删除测试记忆
      ref: 一条测试记忆
      state: { 可删除: true }
      auto_generatable: true
      requirement: false
    - type: 删除失败环境
      ref: 使后端删除返回错误的条件（如目标记忆已被并发删除/后端临时不可用）
      state: { 会触发删除失败: true }
      auto_generatable: false
      requirement: 需构造一次删除失败场景（例如对同一记忆并发删除、或在删除瞬间使后端不可用），以验证失败反馈
  isolation: write-isolated
  notes: 涉及真实删除尝试，使用测试记忆/测试身份。删除失败路径较难稳定复现，可结合并发点击或测试环境构造。
---

# UAT-FORGET-004 删除失败时保留记忆并温和提示（业务异常：遗忘失败保留）

## 业务场景
用户确认遗忘后，万一后端这次没删成功，用户希望那条记忆别凭空消失留下"幽灵"，而是稳妥地留在列表里，并看到一句温和的"这条暂时没忘掉，稍后再试"，自己可以放心地再点一次重试。

## 前置条件
- EgoSync 已启动
- 存在一条测试记忆，并能构造一次删除失败的场景

## 测试步骤
1. 进入含该测试记忆的记忆面板
2. 点击"遗忘"并确认，在构造的删除失败条件下触发后端返回错误
3. 观察该记忆卡片与提示文案
4. 在删除进行中尝试重复点击，观察是否被阻止
5. 失败后再次点击"确认遗忘"重试

## 预期结果
- 删除失败时该记忆卡片仍保留在列表中
- 显示温和失败文案（如"这条记忆暂时没忘掉，稍后再试一下"），不使用 toast/snackbar、不静默失败
- 删除进行中禁止对同一记忆重复点击，避免并发删除
- 失败后允许再次确认遗忘进行重试

## 实际结果
<留空>

## 测试结论
<留空>
