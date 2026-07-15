---
用例编号: UAT-QUADGROUP-004
测试模块: 四象限分组显示
story_key: 3-6-quadrant-grouped-display
version_anchor: 068c84b
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 某象限有 3+ 条任务
      state: { Q2: 3 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证折叠/展开交互与动画。
---

# UAT-QUADGROUP-004 点击象限标题折叠/展开整组

## 业务场景
用户想聚焦某个象限，希望点击标题就能把其他象限折叠起来，减少视觉干扰；再点一次展开，动画平滑不突兀。

## 前置条件
- 某角色有非空象限

## 测试步骤
1. 打开任务面板，确认所有非空象限默认展开
2. 点击某非空象限的标题
3. 观察任务区的折叠动画
4. 再次点击标题，观察展开

## 预期结果
- 非空象限默认展开
- 标题左侧显示折叠箭头（展开 ChevronDown / 折叠 ChevronRight）
- 点击标题后任务区以 ~300ms 动画收起/展开
- 折叠状态不持久化：切换 tab 或关闭面板后重新打开，默认全展开
- 标题按钮有 aria-expanded 属性，键盘可达

## 实际结果
<留空>

## 测试结论
<留空>
