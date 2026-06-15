---
用例编号: UAT-FORGET-001
测试模块: 选择性遗忘
story_key: 2-8-selective-memory-forget
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 含可删除记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 可删除测试记忆
      ref: 某角色或管家名下一条专门用于删除验证的记忆
      state: { 内容: 测试记忆, 可删除: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 删除会真实移除 memories 记录并写入同源屏蔽。必须使用专门预置的测试记忆/测试身份，禁止删除用户真实记忆。
---

# UAT-FORGET-001 点击遗忘弹出管家风格确认（非系统弹窗）

## 业务场景
用户发现一条记忆不想再被记住，点击"遗忘"时，希望看到的是一句有温度、像管家在和自己确认的话，而不是浏览器那种生硬的系统弹窗，让删除这件事也保持产品一贯的体贴感。

## 前置条件
- EgoSync 已启动
- 存在一条专门用于测试的可删除记忆

## 测试步骤
1. 进入含该测试记忆的记忆面板
2. 在该记忆卡片上点击"遗忘"
3. 观察弹出的确认交互样式与文案
4. 检查是否提供"确认遗忘"与"再想想"/取消两个明确选项

## 预期结果
- 显示 EgoSync 内的管家风格确认文案（如"确定要忘记这条吗？忘了就真忘了哦。原始对话还会留在历史里。"）
- 不使用浏览器系统弹窗/alert/toast
- 提供明确的"确认遗忘"与"再想想"/取消操作，遗忘按钮呈现红色等 destructive 视觉层级

## 实际结果
<留空>

## 测试结论
<留空>
