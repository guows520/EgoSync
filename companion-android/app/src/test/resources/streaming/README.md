# 流式黄金契约 fixtures（SPEC-companion-connection-chat-ux / streaming-protocol.md §4）

来源与誊写规则：以下 JSONL 逐行镜像桌面 `llm:stream` StreamPayload 的**生产发射形状**
（字段名、Option skip 语义、phase 值域），由各发射点结构体字面誊写；桌面侧值域由
`egosync-app/src-tauri/src/models/chat.rs` 的 `stream_phase_domain_is_locked` /
`stream_payload_serializes_android_contract_shape` 测试锚定（用户裁决 A+B 双轨：
手工快照 + 桌面 phase 值域锁定测试）。**更新本目录必须与桌面发射点同步，禁止凭空编造形状。**

誊写日期：2026-09-09（行号为当日锚点，随代码漂移以文件内容为准）。

| 文件 | 场景 | 桌面来源锚点 |
| --- | --- | --- |
| 01-answering-multi-token.jsonl | 正文多 token 逐字累加 | `services/agent_engine.rs` emit_stream_token（phase="answering"） |
| 02-thinking-token.jsonl | 思考 token 不落正文段 | `services/agent_engine.rs` SseEvent::Thinking（statusText="思考中..."） |
| 03-thinking-then-answering.jsonl | 思考切换正文（用户报告缺陷的真实时序） | 上述两处组合 |
| 04-tool.jsonl | 工具行（statusText/toolName、token 空） | `services/agent_engine.rs` emit_tool_status（phase="tool"） |
| 05-process.jsonl | 过程事件三型（thinking/narration/tool） | `services/agent_engine.rs` emit_process_event（phase="process"）+ `models/chat.rs` MessageProcessEvent |
| 06-done-with-phase.jsonl | 收口帧 phase="done" | `services/agent_engine.rs` emit_stream_done |
| 07-done-legacy-null.jsonl | 历史 SSE 路径（无 phase 字段） | `services/agent_engine.rs` SseEvent::Text/Done（phase=None，serde skip） |
| 08-multi-message-id.jsonl | 委派双段（messageId 切换） | 同 01（桌面按消息分桶发 token） |
| 09-multi-conversation.jsonl | 跨会话新流归属 | 01/06 组合 |
| 10-phase-null-compat.jsonl | 无 phase 字段的历史形状兼容 | 同 07 |
