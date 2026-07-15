---
用例编号: UAT-PERF-003
测试模块: 性能基准
story_key: 8-4-performance-benchmark
version_anchor: 4f44008
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
  isolation: read-only
  notes: 验证流式渲染延迟。
---

# UAT-PERF-003 流式token渲染延迟三平台差异≤50ms

## 业务场景
用户希望无论在 Windows/macOS/Linux 哪个平台，LLM 流式 token 的渲染延迟都差不多（差异 ≤ 50ms），不会某平台明显卡顿。

## 前置条件
- 默认 LLM 可用
- 三平台均可测试（或至少 Windows + Linux）

## 测试步骤
1. 在 Windows 上发送对话消息，测量流式 token 从到达到 DOM 渲染的延迟
2. 在 Linux 上重复
3. 在 macOS 上重复（本地手动）
4. 对比三平台延迟差异

## 预期结果
- 三平台流式 token 渲染延迟差异 ≤ 50ms
- 流式渲染层无平台分支（agent_bridge.rs SSE 解析 + event_router.rs demux + useTauriEvent 链路统一）
- 渲染顺滑无卡顿

## 实际结果
<留空>

## 测试结论
<留空>
