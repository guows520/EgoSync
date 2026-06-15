---
用例编号: UAT-MEMPANEL-005
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 存在无记忆角色的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 无记忆角色
      ref: 一个尚未积累任何记忆的角色
      state: { 记忆数: 0 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只读查看空态文案，可脚本创建一个无记忆角色。
---

# UAT-MEMPANEL-005 无记忆时显示温暖空态文案（业务异常：空态）

## 业务场景
用户进入一个还没和它聊过几句、没什么记忆的角色或管家记忆面板时，希望看到的是一句有温度、鼓励继续交流的话，而不是冷冰冰的"暂无数据"，让人感觉这个助理是有人情味的。

## 前置条件
- EgoSync 已启动
- 存在一个尚无任何记忆的角色

## 测试步骤
1. 进入该无记忆角色的记忆档案
2. 查看空态显示的文案
3. 切换到管家记忆面板（在管家也无记忆的情况下）查看空态文案

## 预期结果
- 角色无记忆时显示温暖文案（如"还没有记忆，多和这个角色聊聊吧"）
- 管家无记忆时显示温暖文案（如"还没有记忆，多聊几次，我会慢慢记住重要的事"）
- 不出现"暂无数据"这类冷冰冰文案，界面不报错

## 实际结果
<留空>

## 测试结论
<留空>
