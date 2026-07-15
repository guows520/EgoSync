---
用例编号: UAT-CLASSIFY-004
测试模块: 任务自动分类
story_key: 3-3-auto-quadrant-classification
version_anchor: 0871096146e95899b732ed7902c8c7706896c066
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条已被自动分类为 Q3 的任务
      state: { quadrant: Q3, manual_override: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证手动覆盖后系统不再自动覆盖。
---

# UAT-CLASSIFY-004 用户手动选择象限后系统不再自动覆盖

## 业务场景
用户对某条任务的分类有明确判断，手动选了象限，希望系统尊重自己的选择，后续自动分类和临期升 Q1 都不再擅自改动。

## 前置条件
- 存在一条已被自动分类的任务

## 测试步骤
1. 打开该任务的编辑表单
2. 手动将四象限分类改为 Q1（与自动分类不同）
3. 保存
4. 等待下一轮自动分类或临期检查触发
5. 观察该任务象限是否被自动改动

## 预期结果
- 保存后任务象限为用户手动选择的 Q1
- 表单文案明确提示"自动建议，可手动调整"
- 后续自动分类与临期升 Q1 不再覆盖该任务的象限
- 任务被标记为 manual_override = true

## 实际结果
<留空>

## 测试结论
<留空>
