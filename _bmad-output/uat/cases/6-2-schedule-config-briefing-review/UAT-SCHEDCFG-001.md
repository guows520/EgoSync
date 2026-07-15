---
用例编号: UAT-SCHEDCFG-001
测试模块: 节奏化时间配置
story_key: 6-2-schedule-config-briefing-review
version_anchor: 1d51afb
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后可恢复默认值。
---

# UAT-SCHEDCFG-001 配置晨间简报/周复盘/大石头规划触发时间

## 业务场景
用户希望自定义各节奏化功能的触发时间，适配自己作息——比如晨间简报改 07:30、周复盘改周日 20:00、大石头规划提醒改周一 09:00，保存后调度器按新时间触发。

## 前置条件
- 管家设置面板可访问

## 测试步骤
1. 打开管家设置节奏化配置区域
2. 修改晨间简报时间为 07:30
3. 修改周复盘为周日 20:00
4. 修改大石头规划提醒为周一 09:00
5. 保存
6. 重启应用，确认配置保留

## 预期结果
- 三项时间配置 UI 接通真实数据（非 mock defaultValue）
- 保存后写入 app_settings 表
- 重启后配置保留，表单显示已保存的值
- 调度器下次按新时间触发
- 默认值：晨间简报 08:00 / 周复盘 周日 20:00 / 大石头补触发 周一 09:00

## 实际结果
<留空>

## 测试结论
<留空>
