# Deferred Work

## Deferred from: code review of story-2-0b (2026-05-26)

- **opencode.json 并发写竞态** [`GUI/src-tauri/src/services/agent_config.rs`] — 多个 role command 并行执行时 load/save 之间无锁，后写覆盖先写。V1 单用户场景概率极低，未引入回归；保留至并发改造时统一处理。
- **`sync_role_archived` 对损坏 entry 静默 save** [`GUI/src-tauri/src/services/agent_config.rs:148-153`] — 若 `agent[role-x]` 不是 object（仅手工损坏文件触发），`as_object_mut()` 返回 None 但仍执行了 `save`。非本 story 引入路径，触发前提需外部破坏。