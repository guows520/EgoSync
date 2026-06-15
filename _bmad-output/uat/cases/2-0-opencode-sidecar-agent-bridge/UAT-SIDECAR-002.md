---
用例编号: UAT-SIDECAR-002
测试模块: 对话引擎自愈恢复
story_key: 2-0-opencode-sidecar-agent-bridge
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 正在运行的 EgoSync 应用
      state: { 已启动: true, 引擎进程: 运行中 }
      auto_generatable: true
      requirement: false
    - type: 后台引擎进程
      ref: EgoSync 拉起的对话引擎进程
      state: { 可被外部强制结束: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例需要人为强制结束后台引擎进程模拟崩溃（如通过任务管理器结束 opencode 进程）；这是隔离的进程级操作，验证后应用应自行恢复，不破坏对话历史。
---

# UAT-SIDECAR-002 对话引擎在后台意外崩溃后能自动恢复，用户重试即可继续对话

## 业务场景
长时间使用过程中，后台负责思考的引擎有可能因为意外（内存、系统资源等）崩溃。用户不应该因此被卡死——应用应当在很短时间内自动把它重新拉起来，用户重新发一句话就能继续，而不需要重启整个应用。

## 前置条件
- EgoSync 已正常打开并能正常对话（先发一条消息确认管家可回复）
- 测试者具备结束后台进程的能力（任务管理器 / 命令行）

## 测试步骤
1. 在管家对话中先正常完成一轮对话，确认引擎工作正常
2. 模拟后台引擎崩溃：通过任务管理器强制结束 EgoSync 拉起的对话引擎后台进程
3. 等待约 5～10 秒（应用进行后台自愈）
4. 在对话框再次输入一句话并发送
5. 观察这一轮对话是否能正常得到回复

## 预期结果
- 强制结束引擎进程后，应用主窗口不崩溃、不卡死，依旧可操作
- 后台在很短时间内自动重新拉起引擎进程（无需用户手动干预、无需重启应用）
- 用户重试发送的消息能够正常得到管家回复，对话恢复正常
- 此前的对话历史依然完整保留

## 实际结果
<留空>

## 测试结论
<留空>
