---
用例编号: UAT-SIDECAR-003
测试模块: 应用退出资源回收
story_key: 2-0-opencode-sidecar-agent-bridge
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 正在运行的 EgoSync 应用
      state: { 已启动: true, 引擎进程: 运行中 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需在任务管理器中观察进程清单；关闭应用属于正常生命周期操作，不影响用户数据。
---

# UAT-SIDECAR-003 关闭应用后后台引擎彻底退出，不残留占用电脑资源

## 业务场景
用户关掉 EgoSync 窗口后，会理所当然地认为"程序已经退出了"。背后那个负责思考的引擎进程也应该一并干净退出，不能偷偷留在后台占用内存和端口，否则下次打开可能冲突，或让用户觉得电脑变慢。

## 前置条件
- EgoSync 已正常打开并能对话
- 打开任务管理器（或等效工具）准备观察进程

## 测试步骤
1. 正常使用 EgoSync 完成一轮对话
2. 在任务管理器中确认存在 EgoSync 拉起的后台引擎进程
3. 关闭 EgoSync 主窗口（或使用退出菜单完全退出应用）
4. 等待数秒后，在任务管理器中查看相关进程是否还在

## 预期结果
- 应用退出后，EgoSync 主进程消失
- 后台对话引擎进程也随之终止，没有遗留的"孤儿"引擎进程占用资源或端口
- 再次打开应用时能正常启动，不出现端口被占用导致的异常

## 实际结果
<留空>

## 测试结论
<留空>
