---
name: bmad-uat-impact-scan
description: Analyze recent code changes and produce a regression decision report for UAT — two lists, then orchestrate the next steps. List ① re-verify scope (stories whose code changed but whose cases remain valid); List ② suspected-stale cases (stories whose semantics changed so their case version anchors lag). Never silently reuses stale cases — flags them ⚠️ for human confirmation and rebuild. Acts as the regression entry orchestrator: at the end it presents an option menu and routes stale cases to bmad-uat-build-cases (incremental rebuild after human confirmation) and re-verify scope to bmad-uat-prepare-data → bmad-uat-run. Use when the user says "impact scan", "影响面分析", "哪些 story 需要重新验收", "回归范围", or when the UAT 验收官 dispatches the IS menu item.
---

# UAT Impact Scan Workflow

**Goal:** From a code-change range, decide what needs re-verification and what cases may be stale — and orchestrate the regression flow accordingly. Decision-support + entry orchestration; it does not itself run tests or rebuild cases, it routes to the skills that do.

**Your Role:** You are a regression triage officer. You produce evidence-based scope recommendations and stale-case warnings, then guide the user through the next actions. You never silently reuse a stale case.

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
Load `{project-root}/_bmad/bmm/config.yaml` and resolve `user_name`, `communication_language`, `document_output_language`, `implementation_artifacts`, `output_folder`. UAT roots: `uat_root` = `{output_folder}/uat`, `cases_dir` = `{uat_root}/cases`, `reports_dir` = `{uat_root}/reports`. `sprint_status` = `{implementation_artifacts}/sprint-status.yaml`.

### Step 5: Greet the User
Greet `{user_name}` in `{communication_language}`.

### Step 6: Execute Append Steps
Execute each entry in `{workflow.activation_steps_append}` in order. Activation complete.

## Execution

### Step 1: Determine the Change Range
- Default: `<last UAT commit>..HEAD`. Find the last UAT commit from the most recent report/uat-status; if unknown, ask the user for a base ref (e.g. last release tag).
- Accept an explicit range if the user provides one.
- Run `git diff --name-only <range>` to list changed files.

### Step 2: Map Changed Files → Affected Stories
- For each story (in `{cases_dir}` and/or `{sprint_status}`), read its File List / implementation paths.
- A story is affected if any changed file falls within its implementation paths.
- Record per affected story: changed file count, associated case count.

### Step 3: Detect Stale Cases (version anchor comparison)
- For each affected story, compare each case's `version_anchor` against the story's current baseline/commit.
- If the anchor lags AND the change plausibly altered the story's business semantics (e.g. AC change, flow change), flag the case as **suspected stale** with a reason (e.g. "AC 已变更", "流程变更").
- Anchor lags but change is non-semantic (refactor/format) → keep in re-verify scope, not stale.

### Step 4: Produce the impact-scan-report (two lists)
Write `{reports_dir}/impact-scan-{{date}}.md` using `assets/impact-scan-report-template.md`, containing:
- **清单① 建议重验范围**：| story_key | 改动文件数 | 关联用例数 | 建议 |
- **清单② 疑似过期用例**：| 用例编号 | story_key | 锚点commit | 当前commit | 过期原因 | 处置(⚠️待复核) |
- **汇总建议**：本次回归建议范围、需先复核的过期用例数、推荐下一步。

### Step 5: Orchestrate Next Steps (entry orchestrator) — present option menu, HALT
Present the report, then a numbered menu and **wait for input**:
```
扫描完成。检测到 {{stale_count}} 条疑似过期用例、{{scope_count}} 条建议重验用例。
[1] 现在逐条复核过期用例（推荐先做）→ 确认后触发 bmad-uat-build-cases 增量重建
[2] 跳过过期用例，直接对【重验范围】跑回归 → bmad-uat-prepare-data → bmad-uat-run --mode=regression
[3] 仅查看完整清单，稍后手动决定
请选择 [1]/[2]/[3]：
```

### Step 6: Dispatch on Selection
- **[1]**：逐条呈现过期用例，用户对每条选择 重建 / 保留 / 废弃。对"确认重建"的用例集合，调用 `bmad-uat-build-cases`（增量，传入这些用例/story）。重建后回到本菜单或继续 [2]。绝不自动重建未经确认的用例。
- **[2]**：以清单①重验范围为 scope，依次引导 `bmad-uat-prepare-data`（备/校验数据）→ `bmad-uat-run --mode=regression --scope=<范围>`。
- **[3]**：保留报告，结束；提示用户报告路径与后续命令。

Validate against `./checklist.md` before finishing.

## On Complete
Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow.on_complete`. If non-empty, follow it as the final terminal instruction.
