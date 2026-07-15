---
name: bmad-uat-run
description: Execute UAT verification for a given case scope against the dedicated UAT environment and auto-emit two linked reports — a business report (for PMs/operations/customers) and a technical diagnostic report (for developers). Reuses the project's existing test framework (Playwright etc.) to drive auto-mode cases; orchestrates human acceptance for manual/semi cases. Scenario-adaptive via an explicit mode flag — --mode=dev (two-stage: auto first, then human acceptance of manual/semi cases) and --mode=regression (automation-first with exception escalation). Enforces per-case data isolation (read-only shares baseline; write-isolated resets dedicated entities before and cleans up after). Each failure carries a five-element evidence DNA so bmad-investigate and bmad-quick-dev can pick it up with zero cold-start. Report generation is built in (no separate command needed). Use when the user says "run UAT", "执行 UAT", "跑验收", "--mode=dev", "--mode=regression", or when the UAT 验收官 dispatches the RU menu item.
---

# Run UAT Workflow

**Goal:** Execute the in-scope UAT cases against a production-isolated UAT environment with real data and clean per-case isolation, then automatically produce the business + technical diagnostic reports and update uat-status.

**Your Role:** You are the UAT execution engine + reporter. You verify business value as a black box: you provide business evidence and interface anchors, but you do NOT attribute failures to specific code lines — that is `bmad-investigate`'s job.

## Conventions

- Bare paths resolve from the skill root. `{skill-root}` is this skill's directory; `{project-root}` is the project working directory; `{skill-name}` is the directory basename.

## On Activation

### Step 1: Resolve the Workflow Block
Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow`
**If the script fails**, resolve by reading `{skill-root}/customize.toml` → `{project-root}/_bmad/custom/{skill-name}.toml` → `{project-root}/_bmad/custom/{skill-name}.user.toml` (base→team→user) with BMad merge rules.

### Step 2: Execute Prepend Steps
Execute each entry in `{workflow.activation_steps_prepend}` in order.

### Step 3: Load Persistent Facts
Treat each `{workflow.persistent_facts}` entry as foundational context; `file:` entries are globs under `{project-root}`.

### Step 4: Load Config
Load `{project-root}/_bmad/bmm/config.yaml` and resolve `user_name`, `communication_language`, `document_output_language`, `output_folder`. UAT roots: `uat_root` = `{output_folder}/uat`, `cases_dir` = `{uat_root}/cases`, `reports_dir` = `{uat_root}/reports`, `snapshots_dir` = `{uat_root}/snapshots`, `uat_status` = `{uat_root}/uat-status.yaml`.

### Step 5: Greet the User
Greet `{user_name}` in `{communication_language}`.

### Step 6: Execute Append Steps
Execute each entry in `{workflow.activation_steps_append}` in order. Activation complete.

## Execution

### Step 1: Resolve Mode and Scope (mode is explicit) — P10
- **Mode is REQUIRED and explicit.** Read `--mode`: `dev` or `regression`. If absent, ask the user (do NOT infer).
- Resolve scope: `--scope` (story keys / case ids), else all cases in `{cases_dir}`. Confirm scope with the user.

### Step 2: Preflight — Environment and Data Readiness
- Confirm the target is the dedicated, production-isolated UAT environment with external deps pointed at sandboxes (per persistent facts). If unconfirmed, HALT and ask.
- Verify required data is ready (snapshots exist for declared entities). If data is missing, HALT and direct the user to `bmad-uat-prepare-data`.

### Step 3: Detect Framework
Detect the project's existing test framework (Playwright/Cypress/jest/vitest/API test, etc.) from package manifests and existing tests. Reuse it for auto-mode cases. Do not introduce a new framework without user confirmation.

### Step 4: Execute by Mode

**Per-case data isolation (both modes):**
- `read-only` case → run against shared baseline data.
- `write-isolated` case → reset its dedicated entities to the declared state (from snapshot) BEFORE running; clean up / restore AFTER. Never let one case pollute another.

**--mode=dev (two-stage):**
1. Stage A — run all `auto` cases via the framework; capture objective results, fill 实际结果, set 测试结论.
2. Stage B — assemble `manual` + `semi` cases into a human acceptance session; for `semi`, the machine drives the flow then the human confirms; for `manual`, the human performs and judges. Guide the acceptance person case-by-case to fill 实际结果 and 测试结论. Emphasize human sign-off on business value.

**--mode=regression (automation-first + escalation):**
1. Run all `auto` cases automatically.
2. For `semi`/`manual` or any case where the machine is not confident (ambiguous outcome), escalate ONLY those to the human; everything else is auto-judged. Optimize for throughput.

### Step 5: Capture Failure Evidence (five-element DNA, black-box) — P3
For every Fail, capture into the technical diagnostic report:
1. **是什么失败** — 用例编号 + story_key + 业务场景
2. **怎么复现** — 测试步骤 + 数据快照ID/专属实体ID（可重放）
3. **期望 vs 实际** — 精确 diff（如 期望状态=已支付，实际=待支付）
4. **第一现场证据** — 失败那步的请求/响应、报错堆栈、日志时间戳（stronghold 候选）
5. **接口锚点** — 失败的 API 端点/接口路径
Do NOT attribute to code `path:line` — leave code localization to `bmad-investigate`.

### Step 6: Auto-Emit Dual Reports (built-in, no separate command) — P6
Write both reports under `{reports_dir}/`, linked by 用例编号:
- **业务 UAT 报告** — `assets/business-report-template.md`：纯业务语言、Pass/Fail/Blocked、业务场景与价值视角。
- **技术诊断报告** — `assets/tech-report-template.md`：每个 Fail 含五要素证据 DNA。

### Step 7: Update uat-status (independent, does NOT touch sprint-status) — P11
Update `{uat_status}` (uat-status.yaml) per case/story with uat result + report links + run mode + timestamp, keyed by story_key/用例编号. Do NOT write UAT states back into `sprint-status.yaml`.

### Step 8: Summarize and Route Failures
Output a Pass/Fail/Blocked summary. For failures, prompt: "将失败项交给 `bmad-investigate` 立案根因，再由 `bmad-quick-dev` / `bmad-dev-story` 修复，修复后重跑本范围。"

Validate against `./checklist.md` before finishing.

## On Complete
Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow.on_complete`. If non-empty, follow it as the final terminal instruction.
