# Investigation: 重装时无法写入 opencode.exe

## Hand-off Brief

1. **What happened.** 已确认新版 EgoSync 安装器在写入 `D:\Programs\EgoSync\resources\opencode.exe` 时被 Windows 拒绝打开目标文件；“旧版本未关闭导致占用”目前只是用户提出的待验证假设。
2. **Where the case stands.** 案件已建立，截图是当前强证据；尚未检查进程、文件句柄、安装脚本和卸载/升级逻辑。
3. **What's needed next.** 映射证据边界并从安装器写文件路径反向追踪，优先验证目标文件是否被运行中的进程持有。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-26 |
| Status           | Concluded |
| System           | Windows（当前调查环境报告为 Microsoft Windows NT 10.0.19044.0）；EgoSync 安装目录 `D:\Programs\EgoSync` |
| Evidence sources | 用户截图、项目源代码、当前系统进程/文件状态（后两项待调查） |

## Problem Statement

用户报告：老版本 EgoSync 没有关闭，重新安装最新版本时，NSIS 安装器弹出“无法打开要写入的文件：`D:\Programs\EgoSync\resources\opencode.exe`”。用户认为旧版本未关闭可能导致报错；该因果关系尚未验证。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| `C:\Users\Admin\AppData\Local\Temp\codex-clipboard-dcf36b81-2fcf-4367-b2cc-ea96747f4370.png` | Available | 显示 NSIS 正在创建/覆盖安装目录时无法打开 `resources\opencode.exe` 进行写入 |
| 安装器/打包源代码 | Partial | 项目可访问，但尚未定位相关脚本与升级流程 |
| 失败时的进程和文件句柄快照 | Missing | 当前没有安装报错当时的句柄证据，可能需要复现或检查仍在运行的进程 |
| 安装器详细日志 | Missing | 截图未显示 Windows 错误码或占用者 PID |
| 版本控制历史 | Available | 项目 Git 历史可调查，尚未扫描 |
| `project-context.md` | Missing | CodeGraph 未发现匹配文件 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位 NSIS/安装打包脚本及 `opencode.exe` 来源 | High | Open | 查明写入行为及升级前是否关闭进程 |
| 2 | 检查 EgoSync/opencode 相关运行进程与文件路径 | High | Open | 验证旧进程占用假设 |
| 3 | 获取目标文件句柄/锁定者证据 | High | Open | 若当前未复现，需要在报错窗口存在时采集 |
| 4 | 检查近期安装器、进程生命周期相关提交 | Medium | Open | 判断是否为回归或既有缺陷 |
| 5 | 检查杀毒软件、权限、只读属性等替代原因 | Medium | Open | 用于主动反驳文件锁假设 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 2026-07-26（报告日期；实际安装时刻未知） | 用户在旧版本未关闭的情况下重新安装最新版 | 用户描述 | Deduced |
| 同一次安装 | 安装器无法打开 `D:\Programs\EgoSync\resources\opencode.exe` 进行写入 | 用户截图 | Confirmed |

## Confirmed Findings

### Finding 1: 安装失败点是覆盖 opencode.exe

**Evidence:** 用户截图 `C:\Users\Admin\AppData\Local\Temp\codex-clipboard-dcf36b81-2fcf-4367-b2cc-ea96747f4370.png`

**Detail:** NSIS 已进入安装阶段，并在目标路径 `D:\Programs\EgoSync\resources\opencode.exe` 弹出“无法打开要写入的文件”，提供中止、重试和忽略选项。截图本身不能确定底层拒绝原因。

## Deduced Conclusions

### Deduction 1: 故障发生在目标文件创建/替换阶段

**Based on:** Finding 1

**Reasoning:** 报错明确指向“打开要写入的文件”，而不是安装包读取失败或目录创建失败。

**Conclusion:** 调查应集中于目标文件的独占占用、访问控制、文件属性和安全软件拦截，而不是下载或解压来源。

## Hypothesized Paths

### Hypothesis 1: 运行中的旧版 opencode.exe 持有自身映像，阻止安装器覆盖

**Status:** Open

**Theory:** EgoSync 启动的 `opencode.exe` 在 Windows 上仍运行；安装器没有在覆盖前检测或终止它，因此写入失败。

**Supporting indicators:** 用户明确报告旧版本未关闭；报错目标恰好是可执行文件。

**Would confirm:** 报错发生时存在映像路径为目标文件的进程，关闭该进程后点击“重试”立即成功；或句柄工具显示该文件被相关 PID 占用。

**Would refute:** 报错时不存在关联进程/句柄，或关闭全部 EgoSync/opencode 进程后仍稳定失败。

**Resolution:** 待调查。

### Hypothesis 2: 目标文件权限、只读属性或安全软件阻止写入

**Status:** Open

**Theory:** ACL、文件属性或安全软件而非旧进程占用导致写入失败。

**Supporting indicators:** NSIS 的表层报错不包含底层 Windows 错误码，多种拒绝写入条件可产生相同提示。

**Would confirm:** 无占用进程但 ACL/属性检查异常，或安全日志明确拦截安装器。

**Would refute:** 文件权限正常，关闭关联进程后重试即可成功。

**Resolution:** 待调查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 报错瞬间的目标文件占用者 | 决定是否能确认旧进程占用为根因 | 保持报错窗口，检查进程映像路径或使用句柄工具 |
| NSIS 安装脚本和应用退出逻辑 | 决定产品为什么没有预防此状态 | 从项目源码定位打包入口、运行进程启动/清理代码 |
| Windows 底层错误码 | 区分 sharing violation 与 access denied | 安装日志、Process Monitor 或最小复现 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | 尚未定位；截图表明由 NSIS 文件写入阶段产生 |
| Trigger | 在已有安装目录上安装新版并尝试覆盖 `resources\opencode.exe` |
| Condition | 未确认；候选条件为旧进程占用、ACL/属性或安全软件拦截 |
| Related files | 待定位 |

## Conclusion

**Confidence:** Low

已确认安装器失败于覆盖 `opencode.exe`，但尚无直接证据确认占用者。旧版本未关闭是当前优先级最高的假设，不应在取得进程/句柄证据前视为最终根因。

## Recommended Next Steps

### Fix direction

待根因确认后确定。若文件锁假设成立，修复方向应属于“升级前进程生命周期管理”，而不是忽略文件或强行继续安装。

### Diagnostic

先定位安装脚本和进程启动/退出代码，再检查当前相关进程及目标文件状态；可复现时保留报错窗口采集占用者和错误码。

## Reproduction Plan

1. 安装并启动旧版 EgoSync，确保其可能启动的 `opencode.exe` 仍运行。
2. 不退出旧版，运行新版安装器并覆盖同一目录。
3. 记录进程 PID、映像路径和安装器报错。
4. 关闭 EgoSync 但分别验证子进程是否仍存在。
5. 终止确认的占用进程后点击“重试”；若立即成功，则构成确定性复现。

## Side Findings

- 用户截图中的安装器为 Nullsoft Install System（NSIS）；具体脚本和版本仍待源码确认。

## Follow-up: 2026-07-26

### New Evidence

#### Evidence perimeter

| Category | Status | Evidence |
| -------- | ------ | -------- |
| Diagnostic archives / installer logs | Missing | Git 跟踪文件中未发现 `.log`、`.dmp`、`.etl` 或诊断压缩包；截图没有底层 Windows 错误码 |
| Issue tracker | Missing | 当前输入未提供关联 issue/ticket |
| Version control | Available | Git 历史可用；最近提交为 `4b69704`（2026-07-25，版本提升至 0.1.2），工作区存在与本案无关的未提交修改，调查不得覆盖 |
| Tests | Partial | 清点到 51 个前端测试文件、2 个 Rust 测试路径、20 个 E2E 跟踪文件；尚未发现安装升级/残留 sidecar 的现成测试结果 |
| Static analysis | Available | CodeGraph 索引健康：299 个文件、4424 个节点、10775 条边 |
| Source code | Available | `egosync-app/src-tauri/tauri.conf.json:43` 将 `resources/opencode*` 作为安装资源；`sidecar.rs` 含 Windows `opencode.exe` 路径处理 |
| Runtime process state | Available | 2026-07-26 08:57:24 +08:00，PID 82700 正在从目标路径运行 `opencode.exe serve --pure --port 4096`；其记录的父 PID 103396 已不存在 |
| Target file state | Available | 目标文件存在、非只读；ACL 允许 Authenticated Users 修改。无写入数据的独占写打开探针失败，系统明确报告文件正被另一进程使用 |

### Additional Findings

#### Finding 2: 当前确有目标路径上的残留 opencode 进程

**Evidence:** 2026-07-26 08:57:24 +08:00 的 `Win32_Process` / `Get-Process` 查询。

**Detail:** PID 82700 的映像路径正是 `D:\Programs\EgoSync\resources\opencode.exe`，启动时间为 2026-07-25 21:52:47，命令行为 `serve --pure --port 4096`。其父 PID 103396 在取证时已不存在，符合子进程脱离原父进程后继续运行的状态。

#### Finding 3: 当前进程状态可直接阻止安装器式写打开

**Evidence:** 2026-07-26 对目标文件执行 `FileMode.Open + FileAccess.Write + FileShare.None`，未写入任何数据；Windows 返回“文件正由另一进程使用”。

**Detail:** 这确认当前机器上的同一残留进程状态足以阻止对安装目标的独占写打开。尚缺安装报错瞬间的 PID/句柄快照，因此它还不能单独证明截图中的那一次失败由 PID 82700 造成。

#### Finding 4: 权限和只读属性目前不支持替代假设

**Evidence:** 目标文件属性与 ACL 查询。

**Detail:** 文件未标记为 ReadOnly；Authenticated Users 具有 Modify 权限，Administrators/SYSTEM 具有 FullControl。当前证据不支持普通 ACL 或只读属性导致失败。

### Updated Hypotheses

- **Hypothesis 1（残留 opencode 进程占用）：Open，证据显著增强。** 当前已确认完全相同的进程/路径组合会导致写打开失败；仍需源码生命周期追踪及最好一次关闭进程后重试成功的反事实验证。
- **Hypothesis 2（权限/只读）：趋向 Refuted，但暂不正式关闭。** 当前 ACL 与属性正常；安全软件拦截仍未有日志可判定。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位 NSIS/安装打包脚本及 `opencode.exe` 来源 | High | In Progress | 已确认 Tauri 将其作为资源打包，下一步追踪安装前置行为 |
| 2 | 追踪 sidecar 启动、保存句柄和退出清理路径 | High | Open | 当前残留进程的父进程已退出，是关键源码调查方向 |
| 3 | 对“进程占用”做反驳测试 | High | Open | 关闭 PID 82700 后验证独占写打开恢复；涉及终止进程，尚未执行 |
| 4 | 检查近期生命周期/安装器相关提交 | Medium | Open | 当前仅完成历史可用性清点 |
| 5 | 获取安装器/Process Monitor 错误码 | Medium | Blocked | 需要复现报错或用户提供日志 |

## Follow-up: 2026-07-26 #2

### New Evidence

- `Get-Process` 再次确认 PID 82700 仍存活，启动时间仍为 2026-07-25 21:52:47；当前受限环境不能读取其 `Path` 字段，但上一轮非受限 WMI 快照已确认其映像来自安装目标路径。
- 目标文件仍存在，属性与 ACL 未变化；未发现 ReadOnly 或普通用户缺少 Modify 权限的证据。
- 本轮尝试通过 Windows Restart Manager 精确枚举锁定者失败：受限执行环境中的 `python` 不在 PATH；WMI 查询也返回“拒绝访问”。这些失败属于缺失证据，不能被解释为“没有锁定者”。
- 本轮受限环境中的写打开探针返回 `Access denied`，而上一轮同一探针在非受限环境中明确返回“文件正由另一进程使用”。由于执行权限配置发生变化，以上两条不能强行融合；根因判断优先采用权限更完整、错误更具体的上一轮结果，本轮结果仅证明文件依然不可写开。

### Additional Findings

#### Causal chain

1. Tauri/NSIS 安装包将 `resources/opencode*` 写入安装目录（Confirmed）。
2. PID 82700 从该安装目标的 `opencode.exe` 启动并长期运行，原父 PID 已不存在（Confirmed）。
3. Windows 对该目标执行独占写打开失败；权限完整环境明确报告“被另一进程使用”（Confirmed）。
4. NSIS 在覆盖同一路径时报告“无法打开要写入的文件”（Confirmed）。
5. 因此，残留 sidecar 是能够产生截图症状的充分条件（Deduced）。
6. 由于缺少报错瞬间的句柄快照和安装时间，尚不能把“截图中的具体一次失败由 PID 82700 引发”提升为直接确认（Missing evidence）。

#### Refutation pass

| Alternative | Search performed | Result |
| ----------- | ---------------- | ------ |
| ReadOnly 属性 | 两轮文件属性检查 | Refuted：未设置 ReadOnly |
| 普通 ACL 禁止修改 | 两轮 ACL 检查 | Refuted：Authenticated Users 有 Modify |
| 目标文件不存在/路径错误 | `Test-Path` 与文件元数据 | Refuted：目标文件存在且路径与截图完全一致 |
| 没有残留进程 | 两轮独立进程检查 | Refuted：PID 82700 持续存在 |
| 安全软件拦截 | 查找诊断日志/安全事件 | Open：没有可用日志，无法确认或排除 |
| 安装器自身损坏 | 配置与症状对照 | Open 但优先级低：安装已进行到特定资源覆盖，且现存锁定机制足以解释故障 |

### Updated Hypotheses

#### Hypothesis 1: 残留 opencode sidecar 阻止覆盖

**Status:** Open（机制已确认，具体事故归因仍为 Deduced）

**Resolution:** 反证检查已排除只读、普通 ACL、路径不存在和“没有残留进程”。要正式标记 Confirmed，需要报错时句柄证据，或在保持安装报错窗口时终止该 PID 后点击“重试”并立即成功。

#### Hypothesis 2: 权限或只读属性阻止写入

**Status:** Refuted（普通文件属性/ACL 范围）

**Resolution:** 文件非只读，Authenticated Users 具备 Modify。注意：安全软件或受控文件夹访问属于不同机制，仍因无日志保持 Open，不纳入本假设。

### Updated Conclusion

**Confidence:** Medium

现有证据足以确认“父进程退出后仍存活的安装目录内 `opencode.exe` 会使新版安装器无法覆盖该资源”这一故障机制。对用户截图中的具体安装失败，残留 sidecar 是最强且唯一已有正向证据支持的原因；但事故时间和当时句柄快照缺失，因此严格等级仍为 Deduced，而不是 Confirmed。

### Backlog Changes

- 因果推理与反证：Done。
- Restart Manager 锁定者枚举：Blocked（当前受限执行环境缺少可用 Python 且 WMI 被拒绝）。
- 源码生命周期追踪：下一优先事项，需进入 Outcome 4。
- 终止 PID 后重试：未执行；调查阶段不主动中断用户进程。

## Follow-up: 2026-07-26 #3

### New Evidence

#### Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | NSIS/Tauri bundler attempts to replace the bundled resource `resources/opencode.exe`; `egosync-app/src-tauri/tauri.conf.json:39-57` enables Windows NSIS and `egosync-app/src-tauri/tauri.conf.json:43` includes `resources/opencode*`. The exact NSIS error string is not generated by project source. |
| Trigger | Application startup creates `SidecarManager` and calls `sidecar.start()` at `egosync-app/src-tauri/src/lib.rs:195-219`; `SidecarManager::start` launches the resolved executable at `egosync-app/src-tauri/src/services/sidecar.rs:204-267`. |
| Process ownership | `SidecarManager` stores the spawned `Child` at `egosync-app/src-tauri/src/services/sidecar.rs:269-300` and requests `kill_on_drop(true)` at `sidecar.rs:245-248`. This is an in-process lifecycle guarantee, not an installer-level lock release mechanism. |
| Normal close path | The custom title-bar close button only calls `getCurrentWindow().close()` at `egosync-app/src/components/layout/TitleBar.tsx:20-22`. Tauri handles final cleanup only on `RunEvent::Exit` at `egosync-app/src-tauri/src/lib.rs:414-425`, where it calls `SidecarManager::stop()`. |
| Cleanup action | `stop()` takes the child, calls `child.kill().await`, then waits up to the graceful timeout at `egosync-app/src-tauri/src/services/sidecar.rs:313-337`. The kill result is discarded (`let _ =`), so a kill failure is not surfaced or retried by this function. |
| Startup stale-process handling | Existing stale-process cleanup runs only after the new EgoSync process has started: `start()` health-checks port 4096 and calls `kill_process_on_port` at `egosync-app/src-tauri/src/services/sidecar.rs:189-202`; the helper uses `netstat` + `taskkill /F` at `sidecar.rs:447-468`. This cannot help an installer that fails while replacing the binary before the new process launches. |
| Runtime timeline | The log shows a clean prior exit at `C:\Users\Admin\AppData\Roaming\com.egosync.app\egosync.log:33113-33116`, then the sidecar starts from the target binary at `:33124-33126` (`2026-07-25T13:52:45Z`–`:13:52:50Z`, equivalent to 21:52 local). No later `Tauri exiting`/`Stopping opencode` event appears before the log ends at `:36030` (`2026-07-26T00:45:50Z`). The target `opencode.exe` was still alive at 08:57 local. |

### Additional Findings

#### Finding 5: The code has normal-exit cleanup, but no cleanup boundary for forced/external termination

**Evidence:** `lib.rs:414-425`, `sidecar.rs:245-248`, `sidecar.rs:313-337`; runtime log `egosync.log:33113-33116` shows the normal path can stop the sidecar.

**Detail:** The implementation handles the ordinary Tauri exit path and historical logs show it working. However, if the old EgoSync process is terminated by an installer/uninstaller, crashes, or is forcibly killed, Rust destructors and `RunEvent::Exit` are not guaranteed to execute. The child can then remain alive and keep the exact executable image locked. The current implementation has no Windows Job Object/process-tree ownership and no pre-install NSIS step that stops the running sidecar.

#### Finding 6: Startup recovery is too late for this installation failure

**Evidence:** `sidecar.rs:189-202` and `sidecar.rs:447-468`.

**Detail:** The application can kill a stale listener on port 4096 only after the new application process has launched. In the reported sequence, NSIS cannot write `opencode.exe`, so the new application cannot reach this recovery code. Port cleanup therefore does not prevent the installer failure.

#### Finding 7: The runtime log matches an abnormal parent termination window

**Evidence:** `egosync.log:33124-33126` followed by no exit cleanup through `egosync.log:36030`; PID 82700 started at 21:52:47 local and remained alive after the parent PID disappeared.

**Detail:** This is consistent with the old EgoSync process being externally terminated or otherwise exiting without running its Tauri exit handler, while the child survived. The exact external terminator and installer timestamp are not present, so that part remains unconfirmed.

### Updated Hypotheses

#### Hypothesis 1: Residual opencode sidecar blocks replacement

**Status:** Confirmed as the failure mechanism; incident-level cause is Medium confidence.

**Resolution:** The exact target binary was running, the same file could not be opened for exclusive writing, normal file permissions were valid, and the source/log trace shows the child can outlive the parent when cleanup is bypassed. The only remaining gap is the missing installer timestamp/handle snapshot tying PID 82700 to the exact screenshot moment.

#### Hypothesis 2: Missing external-termination/install boundary handling is the product defect

**Status:** Confirmed at source-design level; incident trigger remains Open.

**Theory:** Cleanup exists only inside the running application; installer packaging does not stop or wait for the sidecar before replacing its resource.

**Supporting indicators:** `RunEvent::Exit`-only cleanup, no NSIS preinstall hook in the visible Tauri config, a live orphaned child, and the installer failing before new startup recovery can run.

**Would confirm:** A reproduction where the old app is terminated/closed in the same way as the upgrade, the child remains, and the new installer fails until the child is killed.

**Would refute:** An installer log proving the old sidecar was stopped before the write attempt, or a different process/security product owning the lock.

**Resolution:** Source-design portion confirmed; exact external termination path requires reproduction or installer logs.

### Backlog Changes

- Exact error text scan: Done; no project code emits the NSIS dialog text.
- Source lifecycle trace: Done to diagnosis point.
- Installer timestamp/NSIS verbose log: Open, needed only to upgrade Medium incident confidence to High.
- Windows Job Object or installer preinstall fix: Out of investigation scope; hand off to implementation workflow after user approval.

## Finalization: 2026-07-26

### Hand-off Brief (final)

1. **What happened.** 新版 NSIS 安装器需要覆盖 `D:\Programs\EgoSync\resources\opencode.exe`，但旧版 EgoSync 退出/被外部终止后，sidecar 仍存活并锁定该文件。
2. **Where the case stands.** 根因机制已由进程、文件写打开探针、权限、运行日志和源码交叉确认；具体安装瞬间的 PID/NSIS 句柄日志缺失，但不影响修复方向。
3. **What's needed next.** 进入实现规划，优先增加安装前停止并等待 sidecar 的机制，同时为 Windows 子进程生命周期增加兜底，再用“运行旧版后直接升级”的场景回归验证。

### Final Conclusion

**Confidence:** Medium

**Status:** Concluded

已确认的根因机制是：EgoSync 将 `opencode.exe` 作为安装目录内的运行中 sidecar；正常退出路径会在 `RunEvent::Exit` 中尝试停止它，但外部终止、安装器介入或异常退出时没有可靠的安装前清理和 Windows 进程树兜底，导致 sidecar 遗留并锁住新版安装器要覆盖的同一文件。当前目标文件确实被 `D:\Programs\EgoSync\resources\opencode.exe` 进程持有，ACL/只读属性不能解释现象，运行日志也显示 sidecar 启动后缺少对应的退出清理事件。

具体截图发生瞬间的 PID 和 NSIS 底层错误码未保存，因此不宣称拥有 High 级别的事故时间点证据；但故障机制、源码缺口和现场状态已形成一致证据链。

### Fix Direction

1. **安装边界（优先）：** 在 NSIS 安装/升级前检测并停止旧版 EgoSync 及其 `opencode.exe`，等待进程真正退出后再替换资源；失败时明确提示 PID/路径，而不是直接进入通用 Retry/Ignore 对话框。
2. **运行时兜底：** 在 Windows 上用 Job Object 或等价的进程树生命周期管理，保证父进程被外部终止时 sidecar 不会遗留。
3. **退出错误处理：** 不再静默丢弃 `child.kill()` 错误；增加重试、结果日志和最终强制终止路径。现有启动阶段的端口清理保留为启动兜底，但不能替代安装前清理。

### Diagnostic / Verification Plan

- 在旧版启动并确认 `opencode.exe` 运行后，直接运行新版安装器；预期安装器先停止旧 sidecar，再成功覆盖 `resources/opencode.exe`。
- 覆盖正常关闭、点击安装器升级、任务管理器结束 EgoSync、异常退出四种场景；每种场景验证 `opencode.exe` 不残留、目标文件可独占写入、日志包含停止结果。
- 失败场景保留 NSIS 日志和进程快照，确认错误对话框不再出现；若仍出现，记录具体 PID、路径和 Windows 错误码。

### Reproduction Plan

1. 安装当前版本并启动 EgoSync，确认目标路径的 `opencode.exe` 正在运行。
2. 不退出应用，直接运行新版安装器；记录目标文件写入失败。
3. 关闭/终止旧版 EgoSync，分别观察 sidecar 是否仍存活。
4. 终止遗留 `opencode.exe` 后点击 NSIS 的“重试”；预期立即成功。
5. 应用修复后重复步骤 1–4，预期安装器自动完成清理并成功升级。

### Evidence Gaps

- 原始安装操作的精确时间、NSIS verbose log 和当时的锁定 PID 未保留。
- 当前日志能证明 sidecar 启动后缺少退出清理记录，但不能单独证明是哪个外部组件终止了 EgoSync 主进程。
- 安全软件拦截没有日志证据；不过现有正向锁定证据和源码链已足以确定首要修复方向。

### Recommended Next Step

推荐转入 `bmad-create-story`，将“安装前 sidecar 清理 + Windows 子进程生命周期兜底 + 升级回归测试”拆成可验收的实现故事；不建议先用 `bmad-quick-dev` 直接改动，因为该修复横跨 NSIS 打包边界、Rust 进程管理和 Windows 场景测试。
