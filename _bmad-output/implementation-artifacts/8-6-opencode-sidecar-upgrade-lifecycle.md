---
baseline_commit: 4b69704ed6b57b781ec34d4a5499567366f3d6e8
---

# Story 8.6: 安装升级前清理 opencode sidecar 并加固 Windows 进程生命周期

Status: in-progress

## Story

As a EgoSync 用户,
I want 在旧版本仍运行、异常退出或遗留 sidecar 时重新安装新版能够自动释放 `opencode.exe`,
so that 安装器可以可靠覆盖新版资源并在失败时提供可诊断、可恢复的结果。

## Acceptance Criteria

1. **正常运行时直接升级**
   - **Given** 旧版 EgoSync 正在运行，且其 sidecar 路径为安装目录下的 `resources/opencode.exe`
   - **When** 用户启动新版 Windows 安装器并进入覆盖安装
   - **Then** 安装前清理流程先定位属于旧版 EgoSync 的主进程/sidecar，发起停止并等待相关进程真正退出
   - **And** 只有确认目标进程已退出且资源文件不再被占用后，才继续覆盖 `resources/opencode.exe`
   - **And** 安装完成后新版可以启动 sidecar 并通过现有健康检查

2. **主进程退出后遗留 sidecar**
   - **Given** 旧版 EgoSync 主进程已退出，但同一安装目录的 `opencode.exe` 仍在运行并持有资源文件
   - **When** 新版安装器执行安装前清理
   - **Then** 清理流程能够基于可验证的二进制路径和进程信息识别该遗留 sidecar，而不是仅依赖端口 `4096`
   - **And** 停止并等待该进程退出后，安装可以继续
   - **And** 不得误终止与 EgoSync 无关、但名称同为 `opencode.exe` 的其他进程

3. **停止失败必须显式失败**
   - **Given** 目标进程拒绝退出、进程信息无法读取、文件仍被占用或停止操作返回错误
   - **When** 安装前清理达到重试/等待上限
   - **Then** 不得静默忽略错误或直接把失败伪装成普通安装重试
   - **And** 日志至少包含目标 PID、规范化二进制路径、停止阶段、重试/超时结果和 Windows 错误信息（如可取得）
   - **And** 安装器向用户提供可行动的失败信息，明确指出需要关闭/结束的 EgoSync 或 sidecar；若当前安装框架无法定制对话框，必须保留可定位该问题的日志并阻止继续覆盖

4. **Windows 父子进程生命周期兜底**
   - **Given** EgoSync 主进程在 Windows 上被任务管理器结束、外部终止或异常退出
   - **When** sidecar 已经由该应用启动
   - **Then** 使用 Windows Job Object 或等价的、经验证的进程树生命周期机制，使 sidecar 不会因父进程消失而长期遗留
   - **And** 正常退出仍先执行现有优雅停止流程
   - **And** 实现不得引入常驻后台守护进程或云端依赖

5. **Rust sidecar 停止路径可诊断且保持现有契约**
   - **Given** `SidecarManager::stop()` 被正常退出、重启或安装前清理相关代码调用
   - **When** `child.kill()`、`child.wait()` 或超时等待返回不同结果
   - **Then** 每种结果均有结构化/带上下文的 `tracing` 日志，且停止错误按现有 `Result<T, AppError>` 约定向调用方传播或被显式处理
   - **And** 超时后存在明确的强制终止/最终状态确认路径，不得仅记录“force killed”但没有实际确认
   - **And** 既有启动、重启、健康检查和 `kill_on_drop(true)` 行为不回归

6. **启动兜底与安装前清理边界清晰**
   - **Given** 新版应用已经启动
   - **When** `SidecarManager::start()` 检查端口 `4096` 或发现旧的可用 opencode 服务
   - **Then** 保留现有端口清理和健康检查作为启动阶段兜底
   - **And** 启动阶段清理不得被视为安装器覆盖文件前清理的替代
   - **And** 不能通过无条件按端口杀进程而误伤用户手动运行的非 EgoSync opencode 服务

7. **正常关闭回归**
   - **Given** 用户通过自定义标题栏关闭 EgoSync
   - **When** Tauri 触发退出流程
   - **Then** sidecar 停止结果被记录，正常退出不出现未处理的错误或明显延迟
   - **And** 新增的 Windows 生命周期兜底不会破坏现有 `RunEvent::Exit` 清理、取消 watchdog 和应用关闭行为

8. **升级与异常场景验证**
   - **Given** 修复已实现
   - **When** 在 Windows 实机或 Windows CI 环境执行回归验证
   - **Then** 至少覆盖以下场景：旧版运行后直接升级、正常关闭后升级、任务管理器结束主进程后升级、主进程退出但 sidecar 遗留后升级、停止失败/文件仍锁定
   - **And** 每个场景均记录进程快照、安装/运行日志和最终文件写入结果
   - **And** 验证目标文件可以独占写入，且安装完成后新版 sidecar 可启动
   - **And** 现有 Rust 测试及完整打包验证通过；未执行或无法在 CI 执行的场景必须在测试报告中明确标注

## Tasks / Subtasks

- [x] Task 1: 设计并实现 Windows 安装前 sidecar 清理边界（AC: 1, 2, 3, 6）
  - [x] 选择并记录与当前 Tauri 2/NSIS 配置兼容的安装前执行机制；不得把清理延后到新版应用启动后
  - [x] 基于已安装资源目录和规范化 `opencode.exe` 路径识别 EgoSync 进程，区分同名但不属于本安装的进程
  - [x] 实现停止请求、轮询等待、有限重试、超时后的强制终止和最终退出/文件可写确认
  - [x] 对无法停止的进程返回可诊断失败，记录 PID、路径、阶段、错误和超时信息
  - [x] 保留并验证现有 NSIS 语言、图标、资源打包配置，不改变无关安装行为

- [x] Task 2: 加固 Rust sidecar 停止与退出生命周期（AC: 4, 5, 7）
  - [x] 阅读并沿用 `SidecarManager` 现有启动/停止/重启/健康检查接口，不复制第二套 sidecar 管理逻辑
  - [x] 移除 `child.kill()` 错误的静默丢弃，补充上下文日志和明确的等待/强制终止结果
  - [x] 在 Windows 使用 Job Object 或等价方案管理由 EgoSync 启动的 sidecar 子进程，并记录平台差异
  - [x] 保持 Tauri `RunEvent::Exit`、watchdog 取消、`kill_on_drop(true)` 及非阻塞降级启动语义；若必须调整，补充回归理由
  - [x] 检查自定义标题栏关闭到 Tauri 退出清理的完整路径，确保没有重复停止或锁竞争

- [x] Task 3: 约束端口清理与进程识别（AC: 2, 6）
  - [x] 保留 `start()` 中端口 `4096` 健康检查/清理作为启动兜底
  - [x] 为端口清理增加进程归属校验或明确的安全边界，避免按端口无条件结束非 EgoSync 服务
  - [x] 为“主进程已退出但 sidecar 仍在”的场景补充路径级清理，不依赖应用内 managed state 仍然存在

- [x] Task 4: 编写单元/集成测试（AC: 5, 8）
  - [x] 为停止结果分类、重试/超时、进程识别和最终文件可写确认编写 Rust 测试；测试注释说明这些断言防止安装升级失败的原因
  - [x] 在 Windows 条件编译测试中覆盖 Job Object/等价生命周期行为，以及父进程结束后的 sidecar 状态
  - [x] 覆盖正常停止、已退出进程、停止错误和强制终止结果，禁止空测试或只验证日志字符串存在
  - [x] 保持现有 `cargo test` 基线并记录新增测试数量、平台限制和未覆盖项

- [x] Task 5: 执行 Windows 升级回归与打包验证（AC: 1, 2, 3, 7, 8）
  - [x] 执行 `cargo check` — 通过（仅预先存在的 warnings，无新增 error）
  - [x] 执行 `cargo test --lib sidecar` — 31 个 sidecar 测试全部通过
  - [x] 执行前端 `vitest run` — 42 个测试文件、432 个测试全部通过
  - [x] 执行 `npx tauri build` — NSIS 安装包编译通过，生成 `EgoSync_0.1.2_x64-setup.exe`
  - [ ] **手动验证项（需 Windows 实机，记录为未覆盖项）**：
    - 构建旧版安装包，启动并确认 `resources/opencode.exe` PID；在不先手动结束旧版的情况下运行新版安装器
    - 验证正常关闭、任务管理器结束主进程、异常退出和遗留 sidecar 四类升级路径
    - 保留安装日志、运行日志、进程快照和目标文件独占写入探针结果
  - **预先存在的失败（与本次改动无关）**：`agent_config` 和 `agent_engine` 模块有 6 个测试失败（butler skills/agent prompt 相关），在 baseline_commit 时已存在

### Review Findings

- [x] [Review][Patch] 修复 PowerShell 目标文件路径使用单引号导致 `$InstallDirNorm` 不展开，确保真实文件执行独占写探针 [`egosync-app/src-tauri/windows/installer-hooks.nsh`:27]
- [x] [Review][Patch] 清理流程必须 fail-closed：目录解析、CIM 枚举、PowerShell 启动失败或未枚举到进程时仍执行锁检查，无法确认安全时 `Abort` [`egosync-app/src-tauri/windows/installer-hooks.nsh`:24-29,55-59]
- [x] [Review][Patch] 修复 `nsExec::ExecToLog` 后多余 `Pop` 导致的 NSIS 寄存器栈失衡（安装与卸载宏） [`egosync-app/src-tauri/windows/installer-hooks.nsh`:47-49,98-104]
- [x] [Review][Patch] 将进程归属校验改为规范化安装根目录/目标二进制精确匹配，禁止 `contains("opencode")` 和无目录边界的 `StartsWith` [`egosync-app/src-tauri/src/services/sidecar.rs`:779-798]
- [x] [Review][Patch] 终止前重新核验 PID 的映像路径/身份，避免等待窗口中的 PID 复用误杀无关进程 [`egosync-app/src-tauri/windows/installer-hooks.nsh`:33-38]
- [x] [Review][Patch] `SidecarManager::stop()` 在 kill/wait/超时或最终状态无法确认时返回 `AppError`，关闭 Job 后再次等待确认退出 [`egosync-app/src-tauri/src/services/sidecar.rs`:569-653]
- [x] [Review][Patch] Job Object 创建/分配失败不得继续宣称生命周期兜底成功；消除按 PID 重新打开及 spawn 后分配的竞态 [`egosync-app/src-tauri/src/services/sidecar.rs`:180-205,518-540]
- [x] [Review][Patch] 临时清理脚本使用实例隔离路径并检查 FileOpen/FileWrite 结果，避免并发覆盖或执行陈旧脚本 [`egosync-app/src-tauri/windows/installer-hooks.nsh`:15-52]
- [x] [Review][Patch] 卸载清理不得吞掉停止失败；等待、复核并在进程仍存活或脚本失败时阻止卸载 [`egosync-app/src-tauri/windows/installer-hooks.nsh`:75-104]
- [x] [Review][Patch] 补充行为级 Windows 验证，覆盖 AC8 五类升级场景、真实 PID 退出、文件独占写入和 Job 父进程终止，不以源码字符串断言替代 [`egosync-app/src-tauri/src/services/sidecar.rs`:1147-1360]
- [x] [Review][Defer] 非 Windows 平台仍调用 Windows `cmd/netstat/taskkill` [`egosync-app/src-tauri/src/services/sidecar.rs`:742] — deferred, pre-existing

## Dev Notes

- 根因调查已结案：旧版 EgoSync 的 `D:\Programs\EgoSync\resources\opencode.exe` 仍在运行并锁定新版安装器需要覆盖的同一路径；已观察到遗留 PID，文件独占写打开探针失败，但 ACL/只读属性不是主要原因。
- 关键设计缺口是：现有清理主要依赖应用内退出事件和启动后的端口处理；它们无法可靠覆盖外部终止、安装器介入或父进程退出后 sidecar 遗留。
- 安装前清理是本 Story 的首要修复边界。不能只修改 `SidecarManager::start()`，因为安装器在新版应用启动前就需要写入被旧进程占用的文件。
- 进程识别必须优先使用完整/规范化可执行文件路径、安装目录归属或等价证据；仅按进程名或端口杀进程不满足安全要求。
- Windows Job Object/等价机制的具体 API/封装应由实现阶段结合现有依赖和 Tauri 2 进程模型确定，并在 Dev Agent Record 中记录选择理由；不要在本 Story 中擅自引入新的后台服务。
- 跨平台实现保持最小化：Windows 专属逻辑使用条件编译；macOS/Linux 保持既有生命周期行为，除非共享接口变更有明确回归测试。
- 现有 `SidecarManager::stop()` 使用 `Result<(), AppError>`，但当前实现对 `child.kill()` 返回值静默忽略；该行为是本 Story 明确要求修复的证据点。
- 不要把“安装器显示自定义中文错误对话框”作为无法由当前 NSIS/Tauri 配置支持的硬编码前提；验收关注的是：阻止危险覆盖、错误可定位、用户可采取行动。

### Project Structure Notes

- Sidecar 生命周期与进程管理继续归属于 `egosync-app/src-tauri/src/services/sidecar.rs`；不要把业务逻辑放入 commands 或前端。
- Tauri 应用初始化/退出钩子位于 `egosync-app/src-tauri/src/lib.rs`；自定义标题栏关闭入口位于 `egosync-app/src/components/layout/TitleBar.tsx`。
- Windows 安装/资源打包配置继续位于 `egosync-app/src-tauri/tauri.conf.json`；若需新增 NSIS 脚本、helper 或 Windows 专属模块，应放在现有 `src-tauri` 结构下并记录与 Tauri bundler 的集成点。
- 测试优先放在现有 Rust `#[cfg(test)]`/集成测试结构和既有 E2E/CI 体系中，避免新建与项目无关的测试框架。
- 调查报告记录的是修复方向而非实现承诺；实现完成前不得把调查中的“推荐”写成已经存在的功能。

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Epic 8: 跨平台分发与 V1 加固]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Agent Engine Integration (opencode Sidecar)，进程生命周期管理]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Infrastructure & Deployment / Test Execution]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Layer Rules，sidecar 只负责 opencode 进程生命周期]
- [Source: `_bmad-output/implementation-artifacts/investigations/opencode-file-write-failure-investigation.md` — Final Conclusion / Fix Direction / Diagnostic Plan]
- [Source: `_bmad-output/implementation-artifacts/2-0-opencode-sidecar-agent-bridge.md` — SidecarManager 生命周期、`kill_on_drop`、退出清理与既有测试经验]
- [Source: `egosync-app/src-tauri/src/services/sidecar.rs` — `SidecarManager::start`、`stop`、`is_running`、端口清理]
- [Source: `egosync-app/src-tauri/src/lib.rs` — setup 中启动 sidecar、managed state、`RunEvent::Exit` 清理]
- [Source: `egosync-app/src/components/layout/TitleBar.tsx` — 自定义标题栏关闭入口]
- [Source: `egosync-app/src-tauri/tauri.conf.json` — `bundle.resources` 与 Windows NSIS 配置]
- [Source: `_bmad-output/project-context.md` — Rust `Result<T, AppError>`、`tracing`、测试与构建约束]

## Dev Agent Record

### Agent Model Used

Codex（story authoring，2026-07-26）

### Debug Log References

- 调查档案：`_bmad-output/implementation-artifacts/investigations/opencode-file-write-failure-investigation.md`
- 本 Story 创建阶段未执行代码修改、安装器构建或测试。

### Completion Notes List

- 已根据 Epic 8、架构文档、既有 Story 2-0 和已结案调查创建实现就绪的 Story。
- 已应用代码审查批准的 10 项 patch finding；延期项已记录至 `deferred-work.md`。
- `cargo check`：通过（exit 0）。
- `cargo test --lib sidecar`：通过（32/32，exit 0）；Windows 测试子进程仍打印 `Input redirection is not supported`，但测试断言与 cargo 退出码均成功，需在真实安装器回归中继续观察。
- `git diff --check`：通过（exit 0）。
- `npx tauri build --debug --bundles nsis`：通过，成功生成 `egosync-app/src-tauri/target/debug/bundle/nsis/EgoSync_0.1.2_x64-setup.exe`。仍未完成 AC8 要求的 Windows 实机五类升级场景：旧版运行中、sidecar 运行中、端口占用、文件独占写入、父进程异常终止。因此 Story 与 sprint status 保持 `in-progress`，不得宣称 AC8 已完全验收。
- `cargo fmt --check` 受仓库中与本 Story 无关的既有格式差异影响，未进行全仓格式化。

### File List

- `_bmad-output/implementation-artifacts/8-6-opencode-sidecar-upgrade-lifecycle.md`（本 Story）
- `egosync-app/src-tauri/src/services/sidecar.rs`
- `egosync-app/src-tauri/windows/installer-hooks.nsh`
- `egosync-app/src-tauri/tauri.conf.json`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`