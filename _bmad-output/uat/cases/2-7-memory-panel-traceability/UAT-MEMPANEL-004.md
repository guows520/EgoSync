---
用例编号: UAT-MEMPANEL-004
测试模块: 记忆面板与来源追溯
story_key: 2-7-memory-panel-traceability
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 角色与管家均有记忆的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 预置记忆
      ref: 某角色 12 条记忆，管家总览 18 条（含全局与角色），含历史重复来源
      state: { 角色记忆数: 12, 管家总览数: 18 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 只读核对 badge 数量。可脚本预置精确条数与重复来源以验证去重计数。
---

# UAT-MEMPANEL-004 记忆数量徽标与列表去重后数量一致

## 业务场景
用户希望 Tab 上显示的记忆数量（小徽标）和实际能看到的记忆条数对得上，不会因为历史上同一来源重复提炼过而把数字虚高，让人误以为记了很多。

## 前置条件
- EgoSync 已启动
- 某角色有 12 条可见记忆；管家总览含全局 + 角色共 18 条；数据中存在历史重复来源

## 测试步骤
1. 进入该角色视图，查看"记忆档案"Tab 上的数量徽标
2. 对比徽标数字与列表实际可见卡片数
3. 切换到管家视图，查看"管家记忆"Tab 徽标
4. 对比管家徽标数字与总览列表实际可见数

## 预期结果
- 角色"记忆档案"徽标显示 12，与列表去重后可见条数一致
- 管家"管家记忆"徽标显示总览总数（18），与去重后可见条数一致
- 历史重复来源记忆不被重复计数

## 实际结果
<留空>

## 测试结论
<留空>
