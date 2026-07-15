---
name: bmad-uat-report
description: Optional UAT reporting tool — regenerate or aggregate UAT reports WITHOUT re-running tests. Reuses existing run results in uat-status.yaml and prior reports to (1) re-emit a report for a specific audience (business-only or technical-only), (2) aggregate multiple run results into a consolidated regression report, or (3) refresh a report after manual/semi case conclusions were filled in later. Report generation during a run is already built into bmad-uat-run; use THIS skill only when you need reporting decoupled from execution. Use when the user says "regenerate UAT report", "聚合 UAT 报告", "重出报告", "刷新验收报告", or when the UAT 验收官 dispatches the RP menu item.
---

# UAT Report (Optional) Workflow

**Goal:** Produce or refresh UAT reports from already-recorded results, without executing any tests.

**Your Role:** You are a reporter. You read existing results and render reports; you never run tests (that is `bmad-uat-run`).

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
Load `{project-root}/_bmad/bmm/config.yaml` and resolve `user_name`, `communication_language`, `document_output_language`, `output_folder`. UAT roots: `uat_root` = `{output_folder}/uat`, `reports_dir` = `{uat_root}/reports`, `uat_status` = `{uat_root}/uat-status.yaml`, `cases_dir` = `{uat_root}/cases`.

### Step 5: Greet the User
Greet `{user_name}` in `{communication_language}`.

### Step 6: Execute Append Steps
Execute each entry in `{workflow.activation_steps_append}` in order. Activation complete.

## Execution

### Step 1: Determine Report Intent
Ask / resolve which of the following the user wants:
- **a) 单受众重出** — 只出业务报告 或 只出技术诊断报告。
- **b) 聚合** — 把多次 run 的结果合并成一份回归总报告。
- **c) 刷新** — manual/semi 用例的人工结论后填后，重新渲染报告。

### Step 2: Gather Recorded Results
- Read `{uat_status}` for the relevant scope (run mode, per-case结论, report links, timestamps).
- For refresh: also read the updated case files in `{cases_dir}` to pick up newly filled 实际结果/测试结论.
- For aggregate: collect the set of runs the user wants combined.
- Do NOT execute or judge any case — only consume recorded results.

### Step 3: Render Report(s)
Reuse the templates owned by `bmad-uat-run`:
- 业务报告模板：`{project-root}/.Codex/skills/bmad-uat-run/assets/business-report-template.md`
- 技术诊断报告模板：`{project-root}/.Codex/skills/bmad-uat-run/assets/tech-report-template.md`
Render only what the intent asks for. For aggregation, sum Pass/Fail/Blocked across runs and list per-run breakdown. Keep 用例编号 as the linking key between business and technical reports.

### Step 4: Save and Report
Write outputs under `{reports_dir}/` with a descriptive name (e.g. `business-report-{{date}}.md`, `regression-aggregate-{{date}}.md`). Tell the user the paths.

Validate against `./checklist.md` before finishing.

## On Complete
Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow.on_complete`. If non-empty, follow it as the final terminal instruction.
