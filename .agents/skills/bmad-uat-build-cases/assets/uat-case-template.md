---
用例编号: UAT-<MODULE>-<NNN>
测试模块: <业务模块>
story_key: <如 1-2-order>
version_anchor: <baseline_commit 或 git SHA>
exec_mode: <auto | manual | semi>
destructive: <read-only | write-isolated>
优先级: <高 | 中 | 低>
data_contract:
  entities:
    - type: <实体类型>
      ref: <引用名>
      state: { <key>: <value> }
      auto_generatable: <true | false>
      requirement: <当 auto_generatable=false 时，描述需用户提供的物料要求>
  isolation: <read-only | write-isolated>
  notes: <隔离/重置说明>
---

# <用例编号> <用例标题>

## 业务场景
<该用例对应的实际业务价值 / 用户故事，业务语言>

## 前置条件
<执行前必须满足的环境/权限/数据状态，业务语言>

## 测试步骤
1. <用户操作，业务语言，如 "在搜索框输入商品名并点击搜索">
2. <...>
3. <...>

## 预期结果
<业务层面期望的系统表现、页面跳转、数据变化>

## 实际结果
<执行时填写；auto 类由 bmad-uat-run 填写，manual/semi 类由验收人填写>

## 测试结论
<Pass | Fail | Blocked>
