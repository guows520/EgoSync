---
title: '修复迁移 032 校验值违约——历史开发库恢复启动（auth_sessions 列删正名）'
type: 'bugfix'
created: '2026-09-19'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '9688965dc92e863d0a1bb07f49b7d5b9874a9a2f'
context:
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 15.4 评审修复提交 3a5c85c 对**已应用**的迁移 032_auth_sessions.sql 原地删列（last_seen_at），违反 sqlx「已应用迁移不可变」契约——历史开发库（v32 记录原版 checksum 7cabf4b1…，含 12 角色/8 任务/12 会话的真实数据）启动即报 `migration 32 was previously applied but has been modified`，应用完全不可用。

**Approach:** 032 字节级还原为 3d0f3b6 原版（恢复 checksum 匹配），YAGNI 删列改由新增迁移 033 以 030 重建范式合规重做（保行、终态恰 token_hash+created_at 两列）；随后把 15.5 验证期间被嵌套误置的真历史数据目录换回原位，端到端恢复历史库启动。

## Boundaries & Constraints

**Always:**

- 032 还原必须**字节级**（sqlx checksum = sha384(文件字节)，任何空白差异即新 checksum）；源为 `git show 3a5c85c~1:crates/egosync-engine/migrations/032_auth_sessions.sql`。
- 033 用重建范式（CREATE new + INSERT SELECT + DROP + RENAME，照抄 030_llm_provider_extended.sql 结构）：对「有列历史库」与「无列窗口库」两种血统都成立，且不依赖 SQLite ≥3.35 的 DROP COLUMN。
- 迁移在事务内幂等可重放：已应用 v33 的库再次启动零变更。
- 用户数据操作先归档后替换、先副本验证后动真库。

**Never:**

- 不做代码级 checksum 改写兜底（CRLF 修复先例是空操作归一化，列删是真实 schema 语义变更——代码级改写等于永久掩盖违约）；窗口态库（3a5c85c 后新建、v32=cc552550…）的修复以 033 注释给出手工指引，不写自动逻辑。
- 不动 auth_sessions 之外的任何表、不动 conversations 库迁移（raw SQL 路径与本缺陷无关）、不动 commands.json/dispatch_gen 工件面。
- 不改 `repair_legacy_crlf_migration_checksums` 既有行为。

## I/O & Edge-Case Matrix

| 场景 | 输入 / 状态 | 期望输出 / 行为 | 错误处理 |
|------|-------------|----------------|----------|
| 历史库升级 | DB 已应用 v1..32（v32=7cabf4b1 原版、auth_sessions 含 last_seen_at、含会话行） | init_db 成功；v33 应用；终态列恰 [token_hash, created_at]；既有会话行保留 | N/A |
| 全新建库 | 空路径 | 032 原样建表（含列）→ 033 重建删列；终态两列、v33 记录在案 | N/A |
| 幂等重启 | v33 已应用的库 | 再次 init_db 零变更正常启动 | N/A |
| 窗口态库 | v32=cc552550（3a5c85c~本修复间新建） | 仍报 checksum 不匹配（预期不变绿） | 033 注释给出手工修复指引（UPDATE 回原版 checksum 或删库） |

</frozen-after-approval>

## Code Map

- `crates/egosync-engine/migrations/032_auth_sessions.sql` -- 违约现场；原版含 last_seen_at 列与三行注释差（3a5c85c 增两行评审注释并删列）。
- `crates/egosync-engine/migrations/030_llm_provider_extended.sql` -- 重建范式样板（CREATE/INSERT SELECT/DROP/RENAME）。
- `crates/egosync-engine/src/db/pool.rs` -- `MIGRATOR` 为 `sqlx::migrate!` **编译期嵌入**（改迁移必须重构建二进制）；`run_migrations` 先跑 CRLF 修复再跑 Migrator；测试区 `init_db_repairs_legacy_crlf_migration_checksum_without_losing_data` 是「篡改迁移记录构造历史态」的现成测试模式。
- `~/.local/share/com.egosync.desktop/com.egosync.desktop.bak-155/` -- 真·历史库（egosync.db v32=7cabf4b1、12 角色；conversations.db 12 会话）——15.5 收尾恢复时被 mv 嵌套误置。
- `~/.local/share/com.egosync.desktop/`（外层）-- 窗口态库（v32=cc552550、无列）+ e2e 运行时杂项——15.5 验证期间生成的临时态，归档不删。

## Tasks & Acceptance

**Execution:**
- [x] `crates/egosync-engine/migrations/032_auth_sessions.sql` -- 字节级还原为 3d0f3b6 原版（`git show 3a5c85c~1:...` 落盘后 `sha384sum` 核对前缀 7cabf4b1）-- 恢复已应用迁移的 checksum 契约
- [x] `crates/egosync-engine/migrations/033_auth_sessions_drop_last_seen.sql` -- 新增重建迁移：终态两列、INSERT SELECT 保行；头注释含 15.4 违约始末、窗口态库手工修复 SQL（完整 32 字节原版 sha384 hex，实现时计算） -- YAGNI 删列以合规方式重做
- [x] `crates/egosync-engine/src/db/pool.rs` -- 测试区新增回归：①构造历史态（全量应用后删 v33 记录 + ALTER TABLE 补回 last_seen_at 列 + 预置会话行）再 init_db ⇒ 列删、行保留、v33 记录在案；②全新库终态两列 + 二次 init_db 幂等 -- 矩阵四行中三行的执行级钉（窗口态行为靠 sqlx 语义不测）
- [x] （操作面，非代码）`~/.local/share/com.egosync.desktop/` -- 外层窗口态目录整体归档为 `com.egosync.desktop.window-bak`；嵌套 `.bak-155` 内容上提回原位 -- 用户数据复位；对副本先验证再动真库
- [x] spec `## Implementation Notes` -- 记录验证链与归档路径

**Acceptance Criteria:**
- Given 历史态 DB（v32 原版 checksum、含列、含行），when `init_db`，then 启动成功、终态列恰 [token_hash, created_at]、预置行保留
- Given 全新路径，when `init_db`，then v32+v33 全量应用、终态两列
- Given 真历史库副本，when server `EGOSYNC_DATA_DIR` 启动，then 双池迁移全绿、12 角色可查
- Given 真路径历史库复位后，when 桌面 release 重构建启动 ≥30s，then 零 panic 零迁移报错

## Implementation Notes

**验证链（按执行顺序，全部真命令实测）：**

1. **032 字节级还原**：`git show 3a5c85c~1:crates/egosync-engine/migrations/032_auth_sessions.sql` 落盘；`sha384sum` = `7cabf4b156f54e053ca7e43f87438dc57f55259854a12fe1a17173ab26a96fc605be6f0f4c79875133eeedd90e1cb15a`（前缀 7cabf4b1 与历史库 v32 记录逐字节一致）。
2. **033 新增**：照抄 030 重建范式（CREATE new + INSERT SELECT + DROP + RENAME），显式列清单 SELECT 使「有列/无列」两种血统同构成立；头注释含 15.4 违约始末与窗口态库手工修复 SQL（原版 sha384 hex 全量 96 字符，已程序化比对 git blob 校验零差）。实现期自查发现并修正一处注释内 hex 转写多字（33EEEEDD→33EEEDD）——该修正发生在真库冒烟之后，真库 v33 记录一度携带旧 checksum，已按「删 v33 记录→重构建→重冒烟重放」闭环处理（见 7）。
3. **engine 回归**：`cd crates/egosync-engine && cargo test` ⇒ **820/820 绿**（818 存量 + 新增 2：`init_db_upgrades_legacy_auth_sessions_with_last_seen_at_column` 历史态升级〔含 032 原版 sha384 前缀 7cabf4b1 执行级钉〕、`init_db_fresh_library_drops_last_seen_at_and_restarts_idempotently` 全新库终态+二次幂等）。
4. **server 回归**：`cd server && cargo test` ⇒ **40/40 绿**（lib 5 + api 24 + 2 + parity 5 + secret 4）。
5. **历史库副本验证（动真库前）**：`cp -r ~/.local/share/com.egosync.desktop/com.egosync.desktop.bak-155 /tmp/hist-verify` → `EGOSYNC_DATA_DIR=/tmp/hist-verify EGOSYNC_TOKEN=x EGOSYNC_PORT=8399 cargo run --manifest-path server/Cargo.toml --bin egosync-server`：双池迁移全绿零报错；login 后 `POST /api/cmd/role_list` 返回 **12 角色**；终态 auth_sessions 恰 [token_hash, created_at]、v33 记录在案；同副本二次启动零变更（幂等重启行）。
6. **数据复位（先归档后替换）**：`mv ~/.local/share/com.egosync.desktop ~/.local/share/com.egosync.desktop.window-bak`（窗口态整体归档，含其嵌套 .bak-155）→ `mv …window-bak/com.egosync.desktop.bak-155 ~/.local/share/com.egosync.desktop`（真历史库上提回原位）。复位后校验：v32=7CABF4B1、12 角色、含 last_seen_at 列、无嵌套 .bak-155。
7. **桌面 release 重构建 + xvfb 冒烟**：`npm run build`（tsc 零错）+ `npx tauri build --no-bundle`（4m07s，MIGRATOR 编译期嵌入）；`xvfb-run -a …/target/release/egosync` 真路径启动 **40s 存活、零 panic、双池迁移零报错**；真库 v33 应用、列恰两列、12 角色保留。033 注释修正后重构建重冒烟复验同绿（v33 以修正后 checksum 重放记录）。
8. **DoD**：`cd egosync-app && npm run test:all` 全绿——vitest **51 文件 / 713 测试**、src-tauri cargo **37+1**、engine cargo **820**。

**归档路径（归档不删）：**
- `~/.local/share/com.egosync.desktop.window-bak/`——15.5 验证期间的窗口态库（egosync.db v32=cc552550、无 last_seen_at 列）+ e2e 运行时杂项；该库按矩阵第 4 行预期仍会 checksum 不匹配，手工修复指引见 033 头注释（UPDATE 回原版 checksum 或删库）。
- `~/.config/com.egosync.app/egosync.log.pre-032fix.bak`——冒烟前旧应用日志。
- `/tmp/hist-verify/`——历史库副本验证现场（临时目录，重启即失，无保留义务）。

**实现裁量与既知边界：**
- 回归测试①补列用常量 `DEFAULT '2026-01-01T00:00:00Z'`——SQLite 的 `ALTER TABLE ADD COLUMN` 不允许括号表达式默认值；真实历史库的列来自 032 的 CREATE TABLE，不受此限制。
- 冒烟日志中的 WARN（opencode sidecar 无 binary 降级、keyring DBus 无会话、默认 LLM 未配置）均为无头环境/资源缺席下的既有降级路径，非 panic、非迁移报错。
- 033 重放对「已重建两列终态」的库亦成立（auth_sessions_new 每次重建前必不存在、INSERT SELECT 显式列清单），故删 v33 记录重放是安全操作——真库即以此路径收敛到修正后 checksum。

## Spec Change Log

## Review Triage Log

## Design Notes

- **为何重建而非 `ALTER TABLE … DROP COLUMN`**：DROP COLUMN 需 SQLite ≥3.35 且对无列血统直接报错；重建两种血统通吃（030 先例），事务内原子完成。
- **为何拒绝代码级 checksum 兜底**：`repair_legacy_crlf_migration_checksums` 的正当性在于 CRLF/LF 是零语义差；本例差的是一个真实列——代码级改写等于让「迁移历史可篡改」常态化。

## Verification

**Commands:**
- `cd crates/egosync-engine && cargo test` -- 新回归用例与全量 818+ 绿
- `cp -r ~/.local/share/com.egosync.desktop/com.egosync.desktop.bak-155 /tmp/hist-verify && EGOSYNC_DATA_DIR=/tmp/hist-verify EGOSYNC_TOKEN=x cargo run --manifest-path server/Cargo.toml` -- 历史库**副本**上双池迁移全绿（动真库前的安全验证）
- `cd egosync-app && npm run test:all` -- DoD 全绿（涉 Rust 改动）
- 数据复位后：release 重构建 + xvfb 桌面冒烟 ≥30s 零 panic（真路径端到端）

**Manual checks (if no CLI):**
- 复位后 `ls ~/.local/share/com.egosync.desktop/`：无嵌套 `.bak-155`、无 `window-bak`、egosync.db 即原历史库（12 角色可查）
