# Deferred Work

## Deferred from: code review of 3-3-auto-quadrant-classification (2026-06-18)

- **临期阈值 UTC 与 deadline 本地日期边界偏差 (LOW→deferred, 需全应用时区决策)**：`services/task_deadline_watch.rs:55-65` `compute_imminent_threshold` 基于 `SystemTime` UTC 秒推导日期，而 `deadline` 来自 `<input type="date">` 的本地墙钟日期。全应用刻意 UTC-only（`db::settings::chrono_now` 裸 SystemTime，无 chrono 依赖），单独为本功能引入本地时区会与约定不一致。影响有限且自愈：UTC+8 用户跨日边界附近阈值最多偏早一天，每小时循环随 UTC 推进会在 ≤1 个时区偏移内补上，非永久漏判。建议待全应用时区策略统一时一并处理。
- **`extract_json_object` 贪婪截取首个 `{` 到末个 `}` (LOW, robustness)**：`services/task_classifier.rs:334-341` 当 LLM 在 JSON 前后输出含散落花括号的散文时，截取区间会变成非法 JSON 而解析失败。当前失败安全降级到 Q2，非正确性破坏；可后续改为按花括号深度扫描提取首个完整 JSON 对象增强健壮性。

## Deferred from: code review of 2-12-opencode-ecosystem-skill-discovery-import (2026-06-09)

- **duplicate 分支 replace_bindings 跨角色解绑 (HIGH→deferred, pre-existing)**：`import_opencode_skill` 重复导入时调 `replace_bindings(skill_id, false, [当前角色])`，其语义为 DELETE 该 skill 全部绑定后只重插当前角色，会静默解绑该 Skill 已绑定的其它角色。2.11 `import_custom_skill` 使用完全相同模式 → 非本次引入，应作为统一 binding "merge vs replace" 语义问题单独立项处理。
- **async 命令内阻塞 std::fs I/O (MED→deferred, pre-existing)**：`scan_opencode_root` / `import_opencode_skill` 在 async fn 内同步 `read_dir`/`read_to_string`，慢盘或大目录会阻塞 tokio 工作线程。既有 skill_registry 同步 I/O 模式一致，建议统一迁移到 `tokio::fs` 或 `spawn_blocking`。
- **read_dir 权限失败静默 (LOW→deferred, pre-existing)**：目录存在但权限不可读时 `let Ok(entries) = read_dir else { return Ok(()) }`，与"目录不存在"同等静默，用户无反馈。低概率边界，可与扫描可观测性增强一并处理。
- **content_hash 全局唯一不分 source_type (MED→deferred, 用户裁决)**：内容相同的 opencode Skill 会被 `find_skill_by_content_hash`（不分 source_type）误判为某 custom Skill 的 duplicate，返回 `entry.source_type='custom'`。用户 Decision #1 选项 1 未选改 `(content_hash, source_type)`：V1 同内容跨源场景极罕见，暂保持全局唯一；若未来生态导入增多再立项加 source_type 维度。

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
## Deferred from: code review of 2-8-selective-memory-forget (2026-06-01)

- **useMemories refetch 失败覆盖删除成功状态 (LOW, pre-existing)**：`useMemories.ts:35-40` 在 list 失败时 `setMemories([]) + setError('记忆暂时加载失败…')`，会把刚刚成功的删除结果覆盖成"加载失败"空态。该 hook 未在本故事 diff 中修改，属既有行为；删除主流程已正确，仅在 refetch 阶段网络/DB 抖动时短暂误导。
- **确认对话框无障碍缺口 (LOW, a11y)**：`MemoryTab.tsx:243-276` 的自定义确认 UI 为普通 `<div>`，缺 `role="alertdialog"`、`aria-live`、打开时焦点转移与 Esc 关闭，读屏/纯键盘用户体验不佳。统一归并到 Epic 8 story 8-3（WCAG 可达性审计）处理，避免本故事局部引入与项目其它面板不一致的 a11y 模式。

## Deferred from: code review of 2-11-custom-skill-md-import-role-binding (2026-06-04)

- **command 层拼装受控目录路径 (LOW, 整洁度)**：`commands/skill.rs:35-39` 在命令层拼 `app_data_dir/opencode-workspace/.opencode/skills`，受控目录约定属业务/配置逻辑，应下沉到 service（建议新增 `skill_registry::resolve_skills_root(app_data_dir)`，command 只传 app_data_dir）。无功能影响，boss 决定 defer。
- **description 无长度上限 (LOW, prompt 膨胀)**：外部 SKILL.md 的 description 原样进 DB 并注入每个启用该 Skill 的角色 prompt 与 opencode.json，超长内容会膨胀配置与每次请求 prompt。boss 决定本次不做限长、不加 UI，遗留后续（建议届时复用 app_settings KV，key=skill_description_max_len，默认 2000，按 chars 截断）。
- **缺"日志不含内容/路径"不变量测试 (LOW, 可观测性)**：现有实现设计上合规（tracing 仅记 id/hash/状态，prompt 仅用 name/description，不含用户原始路径），但 diff 中无断言测试锁定该不变量；FS 写失败时 `AppError` 可能携带受控绝对路径进 warn 日志（非用户原始路径，风险低）。建议补一条断言测试固化"日志/prompt 不含文件全文与路径"。

## Deferred from: code review of 2-7-memory-panel-traceability (2026-06-01)

- sourceStates/sourceRequestIds 在 role/category 切换时未清理（MemoryTab.tsx）：memory id 全局唯一不串号，仅轻微内存累积，无功能危害。
- categoryLabels 对未知/历史 category 无兜底标签（MemoryTab.tsx）：insert_memories 有 validate 护栏，仅影响潜在历史脏数据。
- 缺少部分来源缺失 / 前端展开竞态 / Tauri State 注入护栏的测试（memory_query.rs, MemoryTab.test.tsx）：测试增强项，非阻塞。

## Deferred from: code review of 3-2-task-drag-sort-complete (2026-06-17)

- **`reorder_tasks` N+1 查询 (LOW, performance)**：每个 ID 执行一次 SELECT role_id + 一次 UPDATE sort_order，任务量大时往返次数线性增长。SQLite 本地 IO 影响极小，可后续合并为单条 `UPDATE ... CASE WHEN id=? THEN ?` 批量优化。
- **缺 `aria-live` 屏幕阅读器反馈 (LOW, a11y)**：拖拽排序、完成切换无 `aria-live` 区域向辅助技术通报结果。归并到 Epic 8 story 8-3（WCAG 可访问性审计）统一处理。

## Deferred from: code review of 2-13-mcp-server-list-role-access (2026-06-11)

- command 连接测试超时误报 + Windows 孤儿进程（mcp_server.rs::test_command_server）：stdio/local command 仍以 2s 内不退出作为“可长期运行”的启发式判断，慢速失败可能被误报为成功；`cmd /C` 启动的孙进程（npx→node）可能不被 child.kill 完整回收。平台特定，非阻塞。注意：remote `streamable_http`/SSE 测试连接假成功已在 2026-06-13 修复为 MCP protocol/SSE content-type 校验。
- add_to_role 不校验归档角色（mcp_server.rs::add_to_role）：可对 archived 角色建立 MCP 绑定，full_sync 会给 archived agent 加 disable 兜底，仅状态污染。
- sync_role_updated_with_skills_and_mcp 未处理 archived disable 标记（agent_config.rs）：单角色同步路径未设 disable，full_sync 与 sync_role_archived 兜底，低风险。
- 角色级 MCP 硬隔离需真实 opencode 数据流人工 UAT（2.0d 根因领域）：动态切换顶层 mcp scope 对已建 session 的即时生效性仅有单测覆盖，需人工复验。Dev Agent Record 已承认 E2E 未返回结构化结论。
