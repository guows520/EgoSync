---
用例编号: UAT-APP-003
测试模块: 桌面应用启动性能
story_key: 1-1-tauri-desktop-app-existing-ui
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 已安装应用
      ref: egosync-installed
      state: { installed: true, first_compile_done: true }
      auto_generatable: true
      requirement:
    - type: 基准测试机
      ref: mid-range-host
      state: { tier: "中端 Windows 或 M1 Mac" }
      auto_generatable: false
      requirement: 需在符合性能基线的真实硬件（中端 Windows / M1 Mac）上测量冷启动，云虚机或低配机不能代表真实用户体验。
  isolation: read-only
  notes: 仅启动应用并计时，不改动数据；为模拟冷启动可在测量前关闭应用并清理内存缓存。
---

# UAT-APP-003 冷启动时窗口在 3 秒内出现

## 业务场景
真实用户双击图标后，期望应用能很快出现，不会长时间盯着空白屏幕，从而获得"开箱即用"的流畅感受。

## 前置条件
- EgoSync 已安装完成（已完成首次编译，处于可双击运行状态）。
- 测量机为中端 Windows 或 M1 Mac，应用处于完全关闭的冷启动状态。

## 测试步骤
1. 确保 EgoSync 当前未运行（结束所有相关进程）。
2. 双击应用图标，同时开始计时。
3. 当应用窗口首次可见时停止计时。
4. 记录从双击到窗口出现的耗时，重复 3 次取代表值。

## 预期结果
- 冷启动窗口出现时间 < 3 秒。
- 窗口出现后界面正常渲染，无长时间白屏。

## 实际结果
[Blocked] 缺少前置物料：未提供「符合性能基线的真实硬件（中端 Windows / M1 Mac）」（数据需求表 B-2）。冷启动 3 秒判定必须在基准硬件上测量，当前开发机/云虚机不能代表真实用户体验，故不在本轮给出结论以免误导。

## 测试结论
Blocked（阻塞原因：环境物料缺失 B-2 基准硬件；解除后转人工计时执行）
