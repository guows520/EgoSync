# Deferred Work

## Deferred from: code review of 2-6-conversation-memory-extraction (2026-05-30)

- **streaming 标志插入失败永久卡死会话 (HIGH, pre-existing)**：`chat.rs` 中 `streaming.insert` 被提前到 busy 检查后、两条 `insert_message` 之前；任一 DB 写失败 `?` 提前返回时清理任务尚未 spawn，会话本进程内永久返回"我还在想上一个问题"。git 取证确认 baseline 34aab28 顺序安全，此错误顺序来自工作树未提交的前序 opencode 重构，**非 Story 2.6 引入**。⚠️ 必须在该批前序工作提交前修复（恢复 baseline 的"先插 assistant 再 insert streaming"顺序）。
- **记忆去重仅精确文本匹配 (LOW)**：唯一索引 `(source_conversation_id, category, content)` 拦不住 LLM 同义重述；V1 已知边界，Dev Notes 已声明。
- **提炼无取消钩子 + 跨库悬挂记忆 TOCTOU (MED→deferred)**：会话删除/新消息无法打断已开始的提炼；存在性校验与写入间存在 TOCTOU 窗口，可能写入悬挂 source_conversation_id。V1 无消费方，待 2.7/2.8 处理。
- **insert_memories DB 层不校验 source id 归属 (LOW)**：归属白名单仅在 pipeline 层；当前调用链一致，防御性提示。

## Deferred from: code review of 2-0c-dialog-engine-switch-to-opencode (2026-05-26)

All items resolved in the same session:

- ~~streaming_state TOCTOU race~~ ✅ Fixed: merged check+insert into single lock scope
- ~~OnboardingConversations map 无清理~~ ✅ Fixed: auto-remove on step >= 5; also cleaned on conversation delete
- ~~chat_delete_conversation 不清理活跃流/session 状态~~ ✅ Fixed: cancels token, removes streaming/opencode/onboarding state
- ~~SSE parser 未处理 `\r\n\r\n`~~ ✅ Fixed: normalize `\r\n` → `\n` before buffering
- ~~ensure_success 不读取 error body~~ ✅ Fixed: new `ensure_success_with_body` reads body on error; used in create_session, send_message, abort_session
## Deferred from: code review of 2-7-memory-panel-traceability (2026-06-01)

- sourceStates/sourceRequestIds 在 role/category 切换时未清理（MemoryTab.tsx）：memory id 全局唯一不串号，仅轻微内存累积，无功能危害。
- categoryLabels 对未知/历史 category 无兜底标签（MemoryTab.tsx）：insert_memories 有 validate 护栏，仅影响潜在历史脏数据。
- 缺少部分来源缺失 / 前端展开竞态 / Tauri State 注入护栏的测试（memory_query.rs, MemoryTab.test.tsx）：测试增强项，非阻塞。
